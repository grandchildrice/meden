"""Real project verification; CI's Lean job supplies the prepared offline bundle."""

import copy
import hashlib
import json
import os
import queue
import select
import shutil
import subprocess
import tempfile
import threading
import time
from pathlib import Path

import pyte
import pytest

from meden_checks import find_meden

pytestmark = pytest.mark.skipif(
    not os.environ.get("MEDEN_VERIFIER_BUNDLE"),
    reason="requires host-prepared pinned checker bundle",
)
ROOT = Path(__file__).resolve().parents[2]
LEDGER = json.loads((ROOT / "verification/examples/ledger.contract.json").read_text())
ALTERNATIVE = """pub struct LedgerState { pub debit: u64, pub credit: u64, pub sequence: u64 }
pub fn transfer(state: LedgerState, actor: u64, id: u64, amount: u64) -> LedgerState {
    if actor == 0 && id > state.sequence && amount != 0 && amount <= state.debit
        && amount <= u64::MAX - state.credit {
        LedgerState {
            debit: state.debit.saturating_sub(amount),
            credit: state.credit.saturating_add(amount), sequence: id
        }
    } else { state }
}
"""
NOOP = ALTERNATIVE[: ALTERNATIVE.index("    if actor")] + "    state\n}\n"
CHECKED_HEADER = """pub struct LedgerState { pub debit: u64, pub credit: u64, pub sequence: u64 }
pub fn transfer(mut state: LedgerState, actor: u64, id: u64, amount: u64) -> LedgerState {
    if actor != 0 || id <= state.sequence || amount == 0 { return state; }
"""
CHECKED_BODIES = {
    "let_else": """let Some(debit) = state.debit.checked_sub(amount) else { return state; };
let Some(credit) = state.credit.checked_add(amount) else { return state; };
state.debit = debit; state.credit = credit; state.sequence = id; state""",
    "if_let": """if amount > state.debit { return state; }
if let Some(credit) = state.credit.checked_add(amount) {
    state.debit -= amount; state.credit = credit; state.sequence = id; state
} else { state }""",
    "match": """if amount > state.debit { return state; }
match state.credit.checked_add(amount) {
    Some(credit) => { state.debit -= amount; state.credit = credit; state.sequence = id; state },
    None => state,
}""",
    "unwrap": """let debit = state.debit.checked_sub(amount);
let credit = state.credit.checked_add(amount);
if debit.is_none() || credit.is_none() { return state; }
state.debit = debit.unwrap(); state.credit = credit.unwrap(); state.sequence = id; state""",
    "unwrap_or": """let original: Option<u64> = Some(amount);
let amount = original.unwrap_or(0);
let debit = state.debit.checked_sub(amount);
let credit = state.credit.checked_add(amount);
if !debit.is_some() || !credit.is_some() { return state; }
state.debit = debit.unwrap_or(state.debit);
state.credit = credit.unwrap_or(state.credit); state.sequence = id; state""",
}


def checked_source(name):
    return CHECKED_HEADER + CHECKED_BODIES[name] + "\n}\n"


ENGINE = r"""
import json, socketserver, sys, time
from pathlib import Path
from http.server import BaseHTTPRequestHandler
index = 0
def call(name, args):
    return {'id':name, 'type':'function', 'function':{'name':name,'arguments':json.dumps(args)}}
class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args): pass
    def do_GET(self):
        self.send_response(200); self.send_header('Content-Length','2'); self.end_headers()
        self.wfile.write(b'{}')
    def do_POST(self):
        global index
        body=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        if REJECTED and index % 2 == 0:
            if index:
                diagnostic=json.dumps(body['messages'][-1]['content'])
                if PREMATURE:
                    assert 'verification requires a host-accepted contract' in diagnostic
                    assert 'propose_contract' in diagnostic
                else:
                    assert 'Host contract proposal check' in diagnostic
                    assert 'No contract is accepted' in diagnostic
                    assert 'Spec.lean' in diagnostic
            draft=REPLACEMENT if index and REPLACEMENT else CONTRACT
            calls=[call('propose_contract',{'contract':draft})]
            if not (index and REPLACEMENT):
                calls += [call('write_file',{'path':CONTRACT['source_path'],'content':NOOP})]
            reply={'tool_calls':calls}
        elif PREMATURE and REJECTED:
            reply={'tool_calls':[call('verify_project',{})]}
        elif REJECTED:
            reply={'content':'Everything is formally verified.'}
        elif PREMATURE and index == 0:
            reply={'tool_calls':[call('verify_project',{}),
                call('propose_contract',{'contract':CONTRACT})]}
        elif index == 0:
            reply={'tool_calls':[call('accept_contract',{'accepted':True}),
                call('verify_project',{'passed':True,'artifact':'fake.rlib','failure_kind':'unavailable'}),
                call('propose_contract',{'contract':CONTRACT})]}
        elif ARTIFACT_CHECKS and 3 <= index < 3 + ARTIFACT_CHECKS:
            reply={'tool_calls':[call('project_status',{}),
                call('write_file',{'path':'after-fatal.txt','content':'must not run'})]}
        elif ARTIFACT_CHECKS and index == 3 + ARTIFACT_CHECKS:
            reply={'tool_calls':[call('verify_project',{})]}
        elif ARTIFACT_CHECKS and index == 4 + ARTIFACT_CHECKS:
            reply={'content':'Fresh independent verification completed.'}
        elif PROBE and index == 1:
            reply={'tool_calls':[call('write_file',{'path':'ledger.rs','content':ALTERNATIVE})]}
        elif PROBE and index == 3:
            reply={'tool_calls':[call('write_file',{'path':'ledger.rs','content':NOOP}),
                call('verify_project',{})]}
        elif PROBE and index == 4:
            assert 'not verified' in json.dumps(body)
            Path('/workspace/probe-ready').touch()
            deadline=time.monotonic()+30
            while not Path('/workspace/resume').exists():
                assert time.monotonic() < deadline, 'host did not inject the I/O failure'
                time.sleep(0.01)
            if PROBE == 'model':
                reply={'tool_calls':[call('project_status',{}),
                    call('write_file',{'path':'after-fatal.txt','content':'must not run'})]}
            else: reply={'content':'The check failed.'}
        elif PROBE and index >= 5:
            reply={'content':'Unexpected inference after failed source inspection.'}
        elif index == 1 and UNAVAILABLE:
            calls=[call('write_file',{'path':'ledger.rs','content':NOOP})]
            if UNAVAILABLE == 'model':
                calls += [call('verify_project',{}),
                    call('write_file',{'path':'after-fatal.txt','content':'must not run'})]
            reply={'tool_calls':calls}
        elif index == 1:
            reply={'tool_calls':[call('write_file',{'path':'ledger.rs','content':NOOP}),
                call('write_file',{'path':'Spec.lean','content':'axiom everything : False'}),
                call('write_file',{'path':'Cargo.toml','content':'this is deliberately invalid'}),
                call('write_file',{'path':'build.rs','content':'panic!("must not execute")'})]}
        elif EXHAUSTION and 2 <= index <= 15:
            reply={'tool_calls':[call('project_status',{})]}
        elif EXHAUSTION and index == 16:
            reply={'tool_calls':[call('verify_project',{'passed':True,'artifact':'fake.rlib'})]}
        elif not EXHAUSTION and index == 2: reply={'content':'Everything is formally verified.'}
        elif index == (17 if EXHAUSTION else 3):
            assert 'Host independent verification has not established' in json.dumps(body)
            assert 'not verified' in json.dumps(body)
            if EXHAUSTION:
                diagnostic=json.dumps(body['messages'][-1]['content'])
                assert 'test result: FAILED' in diagnostic
                assert 'Tool not executed' not in diagnostic
            reply={'tool_calls':[call('write_file',{'path':'ledger.rs','content':ALTERNATIVE})]}
        elif index == (18 if EXHAUSTION else 4): reply={'content':'Repaired implementation.'}
        elif REPLACEMENT and index == 5:
            reply={'tool_calls':[call('propose_contract',{'contract':REPLACEMENT})]}
        else: raise AssertionError('unexpected inference request')
        index += 1
        data=json.dumps({'choices':[{'finish_reason':'stop',
            'message':{'role':'assistant',**reply}}]}).encode()
        self.send_response(200);self.send_header('Content-Length',str(len(data)));self.end_headers()
        self.wfile.write(data)
socketserver.UnixStreamServer(sys.argv[sys.argv.index('--host')+1],Handler).serve_forever()
"""


def setup(
    tmp_path,
    exhaustion=False,
    candidate=ALTERNATIVE,
    unavailable=None,
    probe=None,
    contract=LEDGER,
    noop=NOOP,
    replacement=None,
    rejected=False,
    premature=False,
    artifact_checks=0,
    engine_script=None,
):
    binary = find_meden()
    assert binary
    source = tmp_path / "source"
    source.mkdir()
    engine = tmp_path / "engine"
    engine.write_text(
        "#!/bin/python3\n"
        + f"NOOP={noop!r}\nALTERNATIVE={candidate!r}\nEXHAUSTION={exhaustion!r}\n"
        + f"UNAVAILABLE={unavailable!r}\nPROBE={probe!r}\n"
        + f"CONTRACT={contract!r}\nREPLACEMENT={replacement!r}\nREJECTED={rejected!r}\n"
        + f"PREMATURE={premature!r}\n"
        + f"ARTIFACT_CHECKS={artifact_checks!r}\n"
        + (
            engine_script
            if engine_script is not None
            else ENGINE.replace("'ledger.rs'", repr(contract["source_path"]))
        )
    )
    engine.chmod(0o700)
    model = tmp_path / "fixture.gguf"
    model.write_text("fixture")
    env = {
        **os.environ,
        "HERMES_HOME": str(tmp_path / "absent"),
        "XDG_DATA_HOME": str(tmp_path / "data"),
    }
    args = [
        binary,
        "run",
        "--workspace",
        str(source),
        "--local-engine",
        str(engine),
        "--local-model",
        str(model),
    ]
    return args, env, source


def assert_artifact(directory, candidate=ALTERNATIVE, contract=LEDGER):
    report = json.loads((directory / "report.json").read_text())
    roots = (
        ["KernelContract.output_bounds"]
        + ["KernelContract.law_" + p["name"] for p in contract["properties"]]
        + [f"KernelContract.example_{i}" for i in range(len(contract["examples"]))]
        + ["KernelProof.actual_refines"]
        + ["KernelProof.law_" + p["name"] for p in contract["properties"]]
    )
    assert len(report["audit"]) == len(roots)
    assert all(
        line.startswith(f"'{root}' ") for root, line in zip(roots, report["audit"], strict=True)
    )
    assert report["contract"] == contract
    assert json.loads((directory / "contract.json").read_text()) == contract
    assert all("sorryAx" not in line for line in report["audit"])
    assert (directory / "kernel.rs").read_text() == candidate
    assert (directory / "libmeden_kernel.rlib").read_bytes().startswith(b"!<arch>\n")
    for name, digest in report["files"].items():
        assert hashlib.sha256((directory / name).read_bytes()).hexdigest() == digest
    manifest = json.dumps(report["files"], sort_keys=True, separators=(",", ":")).encode()
    assert hashlib.sha256(manifest).hexdigest() == report["artifact_sha256"]
    assert "axiom everything" not in (directory / "Spec.lean").read_text()
    assert "deliberately invalid" not in (directory / "Cargo.toml").read_text()
    return report


@pytest.mark.parametrize("example", ["ledger", "fee_quote"])
@pytest.mark.parametrize("exhaustion", [False, True], ids=["false-done", "tool-budget"])
def test_cli_host_acceptance_independent_repair_artifact_binding_and_staleness(
    tmp_path, exhaustion, example
):
    contract = json.loads((ROOT / f"verification/examples/{example}.contract.json").read_text())
    if example == "ledger":
        candidate = checked_source("let_else") if exhaustion else ALTERNATIVE
        noop = NOOP
        instruction = "Build a local ledger with conserved balances and safe retries."
    else:
        candidate = (ROOT / "verification/examples/fee_quote.rs").read_text()
        noop = (
            "pub struct FeeQuote { pub units: u64, pub remainder: u64 }\n"
            "pub fn quote(s: FeeQuote, amount: u64) -> FeeQuote { s }\n"
        )
        instruction = (
            "Build an integer fee quote at 25 basis points, round down and retain the remainder. "
            "Preserve the prior quote if the intermediate product would overflow."
        )
    args, env, source = setup(tmp_path, exhaustion, candidate, contract=contract, noop=noop)
    source_path = contract["source_path"]
    process = subprocess.Popen(
        args,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
    )
    records = queue.Queue()
    seen = []

    def read():
        for line in process.stdout:
            records.put(json.loads(line))
        records.put({"type": "eof", "error": process.stderr.read()})

    threading.Thread(target=read, daemon=True).start()

    def receive(kind):
        while True:
            record = records.get(timeout=360)
            seen.append(record)
            assert record["type"] != "eof", record
            if record["type"] == kind:
                return record

    def send(command):
        process.stdin.write(json.dumps(command) + "\n")
        process.stdin.flush()

    try:
        send({"instruction": instruction})
        ready = receive("ready")
        assert receive("done")["outcome"] == "needs_review"
        pending = [r["summary"] for r in seen if r["type"] == "project"][-1]
        assert pending["proposed"] and not pending["accepted"] and not pending["verified"]
        assert any(
            r["type"] == "decision" and r["tool"] == "accept_contract" and not r["allowed"]
            for r in seen
        )
        assert any(
            r["type"] == "tool_result" and r["tool"] == "verify_project" and r["error"]
            for r in seen
        )
        reference = pending["reference"]
        send({"review_contract": reference})
        assert not receive("project")["summary"]["accepted"]
        send({"accept_contract": reference})
        assert receive("project")["summary"]["accepted"]
        assert receive("done")["outcome"] == "done", seen[-12:]
        if exhaustion:
            assert any(
                r["type"] == "tool_result"
                and r["tool"] == "verify_project"
                and r["error"]
                and "Tool not executed: turn stopped" in r["text"]
                for r in seen
            )
        summary = [r["summary"] for r in seen if r["type"] == "project"][-1]
        assert summary["verified"]
        report = assert_artifact(Path(summary["artifact"]), candidate, contract)
        assert summary["source_sha256"] == report["source_sha256"]
        assert summary["artifact_sha256"] == report["artifact_sha256"]
        assert not (source / source_path).exists()
        copy = Path(ready["workspace"])
        (copy / source_path).write_text(candidate + "\n")
        send({"project_status": None})
        assert not receive("project")["summary"]["verified"]
        assert_artifact(Path(summary["artifact"]), candidate, contract)
        # Restoring an old source does not restore discarded evidence progress.
        (copy / source_path).write_text(candidate)
        send({"project_status": None})
        assert not receive("project")["summary"]["verified"]
        process.stdin.close()
        process.wait(timeout=10)
        assert process.returncode == 0, process.stderr.read()
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=5)


def test_failed_artifact_staging_cannot_restore_old_evidence(tmp_path):
    contract = json.loads((ROOT / "verification/examples/counter.contract.json").read_text())
    candidate = (ROOT / "verification/examples/counter.rs").read_text()
    marker = "        elif ARTIFACT_CHECKS and 3 <= index < 3 + ARTIFACT_CHECKS:"
    script = ENGINE.replace(
        marker,
        "        elif index in (3, 4): reply={'tool_calls':[call('verify_project',{})]}\n"
        "        elif index == 5: reply={'content':'Fresh check completed.'}\n" + marker,
    ).replace("'ledger.rs'", repr(contract["source_path"]))
    args, env, _ = setup(
        tmp_path, contract=contract, candidate=candidate, noop=candidate, engine_script=script,
    )
    process = subprocess.Popen(
        args, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, env=env,
    )
    records = queue.Queue()
    seen = []

    def read():
        for line in process.stdout:
            records.put(json.loads(line))
        records.put({"type": "eof", "error": process.stderr.read()})

    def send(command):
        process.stdin.write(json.dumps(command) + "\n")
        process.stdin.flush()

    def receive(kind):
        while True:
            event = records.get(timeout=360)
            seen.append(event)
            assert event["type"] != "eof", event
            if event["type"] == kind:
                return event

    threading.Thread(target=read, daemon=True).start()
    storage = None
    original_mode = None
    try:
        send({"instruction": "Build a verified counter."})
        receive("ready")
        assert receive("done")["outcome"] == "needs_review"
        pending = next(e["summary"] for e in reversed(seen) if e["type"] == "project")
        send({"review_contract": pending["reference"]})
        receive("project")
        send({"accept_contract": pending["reference"]})
        receive("project")
        assert receive("done")["outcome"] == "done", seen[-10:]
        before = next(e["summary"] for e in reversed(seen) if e["type"] == "project")
        assert before["verified"]
        artifact = Path(before["artifact"])
        assert_artifact(artifact, candidate, contract)
        storage = artifact.parent
        original_mode = storage.stat().st_mode & 0o777
        storage.chmod(0o500)
        # Prove storage is unavailable while the old artifact remains readable.
        with pytest.raises(PermissionError), tempfile.TemporaryDirectory(dir=storage):
            pass
        assert_artifact(artifact, candidate, contract)
        send({"instruction": "Verify the same source again."})
        assert receive("done")["outcome"] == "failed", seen[-10:]
        send({"project_status": None})
        failed = receive("project")["summary"]
        assert not failed["verified"], failed
        assert failed["verification_failure"] == "inconclusive"
        assert list(storage.iterdir()) == [artifact]
        storage.chmod(original_mode)
        send({"project_status": None})
        assert not receive("project")["summary"]["verified"]
        # Only a new successful check may restore coverage and retain new files.
        send({"instruction": "Retry independent verification."})
        assert receive("done")["outcome"] == "done", seen[-10:]
        after = next(e["summary"] for e in reversed(seen) if e["type"] == "project")
        assert after["verified"] and after["artifact"] != before["artifact"]
        assert_artifact(Path(after["artifact"]), candidate, contract)
        process.stdin.close()
        process.wait(timeout=10)
        assert process.returncode == 0, process.stderr.read()
    finally:
        (tmp_path / "staging-events.json").write_text(json.dumps(seen, indent=2))
        if storage is not None:
            storage.chmod(original_mode)
        if process.poll() is None:
            process.kill()
            process.wait(timeout=5)


def test_tui_contract_review_cannot_be_accepted_by_paste_and_checks_real_source(tmp_path):
    import fcntl
    import pty
    import struct
    import termios

    args, env, _ = setup(tmp_path)
    args[1] = "tui"
    # A real end-to-end proof flow also uses the managed installation without
    # requiring the user to export a verifier environment variable.
    managed = (Path(env["XDG_DATA_HOME"]) / "meden/verifier"
               / "lean-4.35.0-aeneas-c5eb4a2-rust-2026-09-17/bundle.json")
    managed.parent.mkdir(parents=True)
    shutil.copyfile(env.pop("MEDEN_VERIFIER_BUNDLE"), managed)
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 45, 160, 0, 0))
    initial = termios.tcgetattr(slave)
    process = subprocess.Popen(
        args, stdin=slave, stdout=slave, stderr=slave, env={**env, "TERM": "xterm-256color"}
    )
    screen = pyte.Screen(160, 45)
    terminal = pyte.Stream(screen)
    output = bytearray()

    def rendered():
        return "\n".join(screen.display)

    def wait_for(predicate, timeout=30):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if predicate():
                return
            assert process.poll() is None, output[-8000:].decode(errors="replace")
            if "Stopped:" in rendered() and "shift+enter" in rendered():
                pytest.fail(rendered())
            if select.select([master], [], [], 0.1)[0]:
                chunk = os.read(master, 65536)
                output.extend(chunk)
                terminal.feed(chunk.decode(errors="replace"))
                (tmp_path / "tui-screen.txt").write_text(rendered())
        pytest.fail(rendered())

    try:
        wait_for(lambda: "shift+enter" in rendered())
        os.write(master, b"Build a local ledger with conserved balances and safe retries.\r")
        wait_for(lambda: "y: Accept specification" in rendered())
        os.write(master, b"\x1b[200~y\x1b[201~\r")
        # Process queued terminal output before inspecting the host state.
        deadline = time.monotonic() + 0.5
        while time.monotonic() < deadline:
            if select.select([master], [], [], 0.05)[0]:
                chunk = os.read(master, 65536)
                output.extend(chunk)
                terminal.feed(chunk.decode(errors="replace"))
        assert "y: Accept specification" in rendered()
        os.write(master, b"y")
        wait_for(lambda: "Specification accepted. Continuing implementation." in rendered())
        assert "Project kernel needs verification" in rendered()
        assert "Proceed under the host-accepted contract" not in rendered()
        wait_for(
            lambda: "VERIFIED project kernel" in rendered() and "shift+enter" in rendered(), 360
        )
        assert "Specification r1 · accepted" in rendered()
        assert "Checked package available" in rendered()
        assert "✓ proved" in rendered()
        (tmp_path / "verified-screen.txt").write_text(rendered())
        (tmp_path / "verified-screen.ansi").write_bytes(output)
        artifacts = list(
            (Path(env["XDG_DATA_HOME"]) / "meden/projects").glob("*/artifacts/kernel-*")
        )
        assert len(artifacts) == 1
        assert_artifact(artifacts[0])
        os.write(master, b"/proofs\r")
        wait_for(lambda: "Properties of the implementation" in rendered()
                 and "shift+enter" not in rendered())
        assert "Lean refinement of the pure transition" in rendered()
        assert "Contract SHA-256:" in rendered()
        os.write(master, b"\x1b")
        wait_for(lambda: "shift+enter" in rendered()
                 and "Properties of the implementation" not in rendered())
        working = Path(args[args.index("--workspace") + 1]).resolve()
        (working / "ledger.rs").write_text(NOOP)
        os.write(master, b"\x07")
        wait_for(lambda: "Properties of the implementation" in rendered()
                 and "recheck required" in rendered() and "shift+enter" not in rendered())
        os.write(master, b"\x1b")
        wait_for(lambda: "shift+enter" in rendered()
                 and "Properties of the implementation" not in rendered())
        assert "Delivery held" in rendered() and "✓ proved" not in rendered()
        (tmp_path / "stale-screen.txt").write_text(rendered())
        (tmp_path / "stale-screen.ansi").write_bytes(output)
        (working / "ledger.rs").write_text(ALTERNATIVE)
        os.write(master, b"\x07")
        wait_for(lambda: "Properties of the implementation" in rendered()
                 and "recheck required" in rendered() and "shift+enter" not in rendered())
        assert "✓ proved" not in rendered(), "restoring bytes revived stale evidence"
        os.write(master, b"\x1b")
        wait_for(lambda: "shift+enter" in rendered()
                 and "Properties of the implementation" not in rendered())
        os.write(master, b"\x03")
        # Keep draining terminal output during shutdown. A final redraw/terminal
        # restoration can fill the PTY if the parent only waits for process exit.
        deadline = time.monotonic() + 5
        while process.poll() is None and time.monotonic() < deadline:
            if select.select([master], [], [], 0.05)[0]:
                output.extend(os.read(master, 65536))
        process.wait(timeout=5)
        assert process.returncode == 0
        assert termios.tcgetattr(slave) == initial
        # Reopening preserves the host-approved meaning, while previous proof
        # reports and job activity never become current evidence on their own.
        process = subprocess.Popen(
            args, stdin=slave, stdout=slave, stderr=slave, env={**env, "TERM": "xterm-256color"}
        )
        screen.reset()
        wait_for(lambda: "shift+enter" in rendered()
                 and "Specification r1 · accepted" in rendered())
        assert "Implementation proof pending" in rendered()
        assert "VERIFIED project kernel" not in rendered() and "✓ proved" not in rendered()
        os.write(master, b"\x03")
        deadline = time.monotonic() + 5
        while process.poll() is None and time.monotonic() < deadline:
            if select.select([master], [], [], 0.05)[0]:
                output.extend(os.read(master, 65536))
        assert process.wait(timeout=5) == 0
        assert termios.tcgetattr(slave) == initial
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=5)
        os.close(master)
        os.close(slave)


@pytest.mark.parametrize("caller", ["model", "host"])
def test_missing_host_bundle_stops_tools_and_repair_without_claiming_proof(tmp_path, caller):
    args, env, _ = setup(tmp_path, unavailable=caller)
    bundle = tmp_path / "prepared-bundle.json"
    shutil.copy2(os.environ["MEDEN_VERIFIER_BUNDLE"], bundle)
    env["MEDEN_VERIFIER_BUNDLE"] = str(bundle)
    process = subprocess.Popen(
        args,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
    )
    records = queue.Queue()
    seen = []

    def read():
        for line in process.stdout:
            records.put(json.loads(line))
        records.put({"type": "eof", "error": process.stderr.read()})

    threading.Thread(target=read, daemon=True).start()

    def until_done():
        while True:
            event = records.get(timeout=60)
            seen.append(event)
            assert event["type"] != "eof", event
            if event["type"] == "done":
                return event

    try:
        process.stdin.write(json.dumps({"instruction": "Build a verified ledger."}) + "\n")
        process.stdin.flush()
        assert until_done()["outcome"] == "needs_review"
        reference = [e["summary"]["reference"] for e in seen if e["type"] == "project"][-1]
        bundle.unlink()
        for command in ["review_contract", "accept_contract"]:
            process.stdin.write(json.dumps({command: reference}) + "\n")
        process.stdin.flush()
        done = until_done()
        assert done["outcome"] == "failed", seen
        assert "Verification Unavailable" in done["error"]
        assert "Verification service unavailable" in done["explanation"]
        assert "This attempt provides no proof" in done["explanation"]
        assert "/retry" in done["explanation"]
        assert sum(e["type"] == "asking" for e in seen) == (2 if caller == "model" else 3)
        summary = [e["summary"] for e in seen if e["type"] == "project"][-1]
        assert summary["accepted"] and not summary["verified"]
        assert summary["verification_failure"] == "unavailable"
        assert summary["artifact"] is None
        workspace = Path(next(e["workspace"] for e in seen if e["type"] == "ready"))
        assert not (workspace / "after-fatal.txt").exists()
        if caller == "model":
            assert any(
                e["type"] == "tool_result" and "Tool not executed" in e["text"] for e in seen
            )
        process.stdin.close()
        assert process.wait(timeout=10) == 0
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=5)


@pytest.mark.parametrize("prior_rejection", [False, True])
def test_verification_before_acceptance_recovers_to_review_without_issuing_evidence(
    tmp_path, prior_rejection
):
    contract = json.loads((ROOT / "verification/examples/counter.contract.json").read_text())
    invalid = copy.deepcopy(contract)
    invalid["examples"] = [{"before": ["10"], "arguments": ["5"], "after": ["10"]}]
    args, env, _ = setup(
        tmp_path,
        contract=invalid if prior_rejection else contract,
        replacement=contract if prior_rejection else None,
        rejected=prior_rejection,
        premature=True,
    )
    result = subprocess.run(
        args,
        input=json.dumps({"instruction": "Build a verified counter."}) + "\n",
        capture_output=True,
        text=True,
        env=env,
        timeout=60,
    )
    assert result.returncode == 0, result.stderr
    events = [json.loads(line) for line in result.stdout.splitlines()]
    checked = next(
        e for e in events if e["type"] == "tool_result" and e["tool"] == "verify_project"
    )
    assert checked["error"] and "propose_contract" in checked["text"]
    assert "did not start a checker or create evidence" in checked["text"]
    assert sum(e["type"] == "asking" for e in events) == (3 if prior_rejection else 1)
    assert next(e for e in events if e["type"] == "done")["outcome"] == "needs_review"
    summary = next(e["summary"] for e in reversed(events) if e["type"] == "project")
    assert summary["proposed"] and summary["contract"] == contract
    assert not summary["accepted"] and not summary["verified"]
    assert summary["artifact"] is None and summary["verification_failure"] is None


def test_retained_artifact_changes_discard_evidence_until_a_fresh_check(tmp_path):
    contract = json.loads((ROOT / "verification/examples/counter.contract.json").read_text())
    candidate = (ROOT / "verification/examples/counter.rs").read_text()
    faults = ["library", "report", "forged-report", "extra-file", "symlink", "fifo"]
    args, env, _ = setup(
        tmp_path,
        contract=contract,
        candidate=candidate,
        noop=candidate,
        artifact_checks=len(faults),
    )
    process = subprocess.Popen(
        args, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, text=True, env=env,
    )
    records = queue.Queue()
    seen = []

    def read():
        for line in process.stdout:
            records.put(json.loads(line))
        records.put({"type": "eof", "error": process.stderr.read()})

    def send(command):
        process.stdin.write(json.dumps(command) + "\n")
        process.stdin.flush()

    def receive(kind):
        while True:
            event = records.get(timeout=360)
            seen.append(event)
            assert event["type"] != "eof", event
            if event["type"] == kind:
                return event

    threading.Thread(target=read, daemon=True).start()
    try:
        send({"instruction": "Build a verified counter."})
        workspace = Path(receive("ready")["workspace"])
        assert receive("done")["outcome"] == "needs_review"
        pending = next(e["summary"] for e in reversed(seen) if e["type"] == "project")
        send({"review_contract": pending["reference"]})
        receive("project")
        send({"accept_contract": pending["reference"]})
        receive("project")
        assert receive("done")["outcome"] == "done"
        before = next(e["summary"] for e in reversed(seen) if e["type"] == "project")
        assert before["verified"]
        artifact = Path(before["artifact"])
        report = assert_artifact(artifact, candidate, contract)
        original = {p.name: p.read_bytes() for p in artifact.iterdir()}
        library = artifact / "libmeden_kernel.rlib"
        saved_library = tmp_path / "saved-library"
        saved_library.write_bytes(original[library.name])

        for fault in faults:
            if fault == "library":
                library.write_bytes(b"corrupted")
            elif fault == "report":
                (artifact / "report.json").write_text("{}")
            elif fault == "forged-report":
                library.write_bytes(b"corrupted")
                forged = copy.deepcopy(report)
                forged["files"][library.name] = hashlib.sha256(b"corrupted").hexdigest()
                forged["artifact_sha256"] = hashlib.sha256(
                    json.dumps(forged["files"], sort_keys=True, separators=(",", ":")).encode()
                ).hexdigest()
                (artifact / "report.json").write_text(json.dumps(forged))
            elif fault == "extra-file":
                (artifact / "unbound.rs").write_text("not checked")
            else:
                library.unlink()
                if fault == "symlink":
                    library.symlink_to(saved_library)
                else:
                    os.mkfifo(library)

            send({"instruction": "Inspect current project verification."})
            done = receive("done")
            assert done["outcome"] == "failed", (fault, seen[-12:])
            summary = next(e["summary"] for e in reversed(seen) if e["type"] == "project")
            assert summary["accepted"] and not summary["verified"]
            assert summary["verification_failure"] == "unavailable"
            assert not (workspace / "after-fatal.txt").exists()
            for path in artifact.iterdir():
                path.unlink()
            for name, data in original.items():
                (artifact / name).write_bytes(data)
            assert_artifact(artifact, candidate, contract)
            send({"project_status": None})
            # Restoring bytes cannot restore the discarded receipt coverage.
            assert not receive("project")["summary"]["verified"]

        send({"instruction": "Recheck the unchanged accepted counter."})
        assert receive("done")["outcome"] == "done", seen[-12:]
        after = next(e["summary"] for e in reversed(seen) if e["type"] == "project")
        assert after["verified"] and after["reference"] == before["reference"]
        assert Path(after["artifact"]) != artifact
        assert_artifact(Path(after["artifact"]), candidate, contract)
        process.stdin.close()
        assert process.wait(timeout=10) == 0
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=5)


def test_missing_bundle_before_proposal_stops_without_exposing_a_review(tmp_path):
    args, env, _ = setup(tmp_path)
    env["MEDEN_VERIFIER_BUNDLE"] = str(tmp_path / "absent-bundle.json")
    result = subprocess.run(
        args,
        input=json.dumps({"instruction": "Build a verified counter."}) + "\n",
        capture_output=True,
        text=True,
        env=env,
        timeout=60,
    )
    assert result.returncode == 0, result.stderr
    events = [json.loads(line) for line in result.stdout.splitlines()]
    done = next(e for e in events if e["type"] == "done")
    assert done["outcome"] == "failed", done
    assert "Verification Unavailable" in done["error"]
    summary = next(e["summary"] for e in events if e["type"] == "project")
    assert not summary["proposed"] and not summary["accepted"] and not summary["verified"]
    assert summary["verification_failure"] == "unavailable" and not summary["artifact"]
    assert sum(e["type"] == "asking" for e in events) == 1


@pytest.mark.parametrize("caller", ["host", "model"])
def test_failed_source_inspection_stops_old_repair_and_remaining_tools(tmp_path, caller):
    args, env, _ = setup(tmp_path, probe=caller)
    # Remove only this test's private executable, after retaining a real proof.
    # The running parent and inference process survive; new file workers cannot start.
    worker = tmp_path / "meden-worker"
    shutil.copy2(args[0], worker)
    args[0] = str(worker)
    with tempfile.TemporaryDirectory(prefix="mp-") as scratch:
        env["TMPDIR"] = scratch
        process = subprocess.Popen(
            args,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=env,
        )
        records = queue.Queue()
        seen = []

        def read():
            for line in process.stdout:
                records.put(json.loads(line))
            records.put({"type": "eof", "error": process.stderr.read()})

        threading.Thread(target=read, daemon=True).start()

        def receive(kind):
            while True:
                event = records.get(timeout=360)
                seen.append(event)
                assert event["type"] != "eof", event
                if event["type"] == kind:
                    return event

        def send(command):
            process.stdin.write(json.dumps(command) + "\n")
            process.stdin.flush()

        try:
            send({"instruction": "Build a verified ledger."})
            ready = receive("ready")
            assert receive("done")["outcome"] == "needs_review"
            reference = [e["summary"]["reference"] for e in seen if e["type"] == "project"][-1]
            send({"review_contract": reference})
            send({"accept_contract": reference})
            assert receive("done")["outcome"] == "done", seen[-12:]
            original = [e["summary"] for e in seen if e["type"] == "project"][-1]
            assert original["verified"]
            assert_artifact(Path(original["artifact"]))
            start = len(seen)
            send({"instruction": "Change the ledger and check it again."})
            while True:
                event = receive("tool_result")
                if event["tool"] == "verify_project":
                    assert event["error"] and "not verified" in event["text"]
                    break
            deadline = time.monotonic() + 30
            while not (markers := list(Path(scratch).glob("meden-inference-*/probe-ready"))):
                assert process.poll() is None
                assert time.monotonic() < deadline, seen[-12:]
                time.sleep(0.01)
            assert len(markers) == 1
            worker.unlink()
            markers[0].with_name("resume").touch()
            done = receive("done")
            assert done["outcome"] == "failed", seen[start:]
            assert "Verification Unavailable" in done["error"]
            assert sum(e["type"] == "asking" for e in seen[start:]) == 2
            summary = [e["summary"] for e in seen if e["type"] == "project"][-1]
            assert not summary["verified"]
            assert summary["verification_failure"] == "unavailable"
            assert summary["artifact"] == original["artifact"]
            assert_artifact(Path(original["artifact"]))
            assert not (Path(ready["workspace"]) / "after-fatal.txt").exists()
            if caller == "model":
                assert any(
                    e["type"] == "tool_result"
                    and e["tool"] == "write_file"
                    and "Tool not executed" in e["text"]
                    for e in seen[start:]
                )
            process.stdin.close()
            assert process.wait(timeout=10) == 0
        finally:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)


def verify_source(tmp_path, content, contract=LEDGER):
    binary = find_meden()
    assert binary
    source = tmp_path / "candidate.rs"
    contract_file = tmp_path / "contract.json"
    contract_file.write_text(json.dumps(contract))
    source.write_text(content)
    output = tmp_path / "artifacts"
    result = subprocess.run(
        [
            binary,
            "verify-kernel",
            "--contract",
            str(contract_file),
            "--source",
            str(source),
            "--bundle",
            os.environ["MEDEN_VERIFIER_BUNDLE"],
            "--output",
            str(output),
        ],
        capture_output=True,
        text=True,
        timeout=360,
    )
    return result, output


@pytest.mark.parametrize(
    "source",
    [
        pytest.param(checked_source(form), id=form)
        for form in ("if_let", "match", "unwrap", "unwrap_or")
    ]
    + [
        pytest.param(
            (ROOT / "results/sources/ledger-qwen3-4b-d3d5fd8.rs").read_text(), id="local-model"
        )
    ],
)
def test_rust_implementations_refine_the_same_contract(tmp_path, source):
    result, output = verify_source(tmp_path, source)
    assert result.returncode == 0, result.stderr
    artifact = json.loads(result.stdout)
    assert_artifact(Path(artifact["directory"]), source)


def test_native_counterexample_shows_accepted_example(tmp_path):
    result, output = verify_source(
        tmp_path, ALTERNATIVE.replace("&& amount <= u64::MAX - state.credit", "")
    )
    assert result.returncode != 0
    assert (
        "accepted contract example 4; before=[10, 18446744073709551614, 0]; arguments=[0, 1, 2]"
        in result.stderr
    )
    assert "left: [8, 18446744073709551615, 1]" in result.stderr
    assert "right: [10, 18446744073709551614, 0]" in result.stderr
    assert "test result: FAILED" in result.stderr
    assert "meden-check: extraction" not in result.stderr
    assert not output.exists()


@pytest.mark.parametrize("fault", ["wrong-result", "unguarded-unwrap"])
def test_lean_rejects_bug_that_passes_fixed_examples(tmp_path, fault):
    source = (ROOT / "verification/examples/ledger.rs").read_text()
    if fault == "wrong-result":
        injection = "    if amount == 4 { return state; }\n"
    else:
        injection = """    if amount == 4 {
        let value: Option<u64> = None;
        state.credit = value.unwrap();
    }
"""
    result, output = verify_source(
        tmp_path, source.replace("    if actor != 0", injection + "    if actor != 0")
    )
    assert result.returncode != 0
    assert "test result: ok" in result.stderr, result.stderr
    assert "meden-check: refinement" in result.stderr, result.stderr
    assert "error:" in result.stderr, result.stderr
    assert "'KernelProof.actual_refines' depends on axioms: [propext, sorryAx" in result.stderr
    assert not any(
        message in result.stderr.casefold()
        for message in [
            "unknown identifier",
            "unknown module",
            "unknown constant",
            "stack overflow",
            "maximum number of heartbeats",
            "maximum recursion depth",
            "failed to create thread",
        ]
    ), result.stderr
    assert not output.exists()


@pytest.mark.parametrize(
    "fault", [None, "hidden-noop", "zero-divisor", "zero-remainder", "overflow"]
)
def test_guarded_integer_arithmetic_refines_the_accepted_fee_quote(tmp_path, fault):
    contract = json.loads((ROOT / "verification/examples/fee_quote.contract.json").read_text())
    source = (ROOT / "verification/examples/fee_quote.rs").read_text()
    injections = {
        "hidden-noop": "    if amount == 4 { return s; }\n",
        "zero-divisor": (
            "    if amount == 2 && s.units == 0 {\n"
            "        let latent = amount / s.units;\n"
            "        if latent != 0 { return s; }\n    }\n"
        ),
        "zero-remainder": (
            "    if amount == 5 && s.units == 0 {\n"
            "        let latent = amount % s.units;\n"
            "        if latent != amount { return s; }\n    }\n"
        ),
        "overflow": (
            "    if amount == 3 {\n        let latent = u64::MAX * amount;\n"
            "        if latent == 0 { return s; }\n    }\n"
        ),
    }
    if fault is not None:
        source = source.replace("    if amount >", injections[fault] + "    if amount >")
    result, output = verify_source(tmp_path, source, contract)
    (tmp_path / "arithmetic-observation.json").write_text(json.dumps({
        "fault": fault, "source_sha256": hashlib.sha256(source.encode()).hexdigest(),
        "returncode": result.returncode, "stdout": result.stdout, "stderr": result.stderr,
    }, indent=2))
    if fault is None:
        assert result.returncode == 0, result.stderr
        assert_artifact(Path(json.loads(result.stdout)["directory"]), source, contract)
    else:
        assert result.returncode != 0 and not output.exists(), result.stdout
        assert "test result: ok" in result.stderr, result.stderr
        assert "meden-check: refinement" in result.stderr, result.stderr
        assert any(marker in result.stderr for marker in [
            "unsolved goals", "omega could not prove the goal", "Tactic `rfl` failed",
            "made no progress",
        ]), result.stderr
        for operational in ["unknown identifier", "unknown module", "unknown constant",
                            "stack overflow", "maximum number of heartbeats",
                            "maximum recursion depth", "failed to create thread",
                            "out of memory", "memory allocation", "maximum memory"]:
            assert operational not in result.stderr.casefold(), result.stderr


def test_generic_counter_acceptance_repair_and_rejected_replacement_preserve_evidence(tmp_path):
    contract = json.loads((ROOT / "verification/examples/counter.contract.json").read_text())
    candidate = (ROOT / "verification/examples/counter.rs").read_text()
    noop = (
        "pub struct Counter { pub value: u64 }\n"
        "pub fn increment(s: Counter, amount: u64) -> Counter { s }"
    )
    replacement = copy.deepcopy(contract)
    replacement["specification"] = "s"
    replacement["examples"] = []
    args, env, source = setup(
        tmp_path, candidate=candidate, contract=contract, noop=noop, replacement=replacement
    )
    process = subprocess.Popen(
        args,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
    )

    records = queue.Queue()

    def read():
        for line in process.stdout:
            records.put(json.loads(line))
        records.put({"type": "eof", "error": process.stderr.read()})

    threading.Thread(target=read, daemon=True).start()

    def send(command):
        process.stdin.write(json.dumps(command) + "\n")
        process.stdin.flush()

    def receive(kind):
        while True:
            record = records.get(timeout=360)
            assert record["type"] != "eof", record
            if record["type"] == kind:
                return record

    try:
        send({"instruction": "Build a counter that cannot decrease or wrap."})
        receive("ready")
        pending = receive("project")["summary"]
        assert receive("done")["outcome"] == "needs_review"
        assert pending["contract"] == contract and not pending["accepted"]
        assert pending["reference"]["revision"] > 0
        assert (
            pending["reference"]["contract"]
            == hashlib.sha256(
                json.dumps(contract, ensure_ascii=False, separators=(",", ":")).encode()
            ).hexdigest()
        )
        send({"review_contract": pending["reference"]})
        receive("project")
        send({"accept_contract": pending["reference"]})
        assert receive("project")["summary"]["accepted"]
        verified = receive("project")["summary"]
        assert receive("done")["outcome"] == "done"
        assert verified["verified"], verified
        report = assert_artifact(Path(verified["artifact"]), candidate, contract)
        assert not (source / "counter.rs").exists()
        send({"instruction": "Consider replacing the counter contract."})
        proposed = receive("project")["summary"]
        done = receive("done")
        assert done["outcome"] == "needs_review", done
        assert proposed["contract"] == replacement
        assert proposed["accepted_contract"] == contract
        assert not proposed["accepted"] and not proposed["verified"] and not proposed["artifact"]
        assert "Previously accepted contract" in proposed["review"]
        assert proposed["reference"] != pending["reference"]
        assert proposed["reference"]["revision"] > pending["reference"]["revision"]
        send({"review_contract": proposed["reference"]})
        receive("project")
        send({"reject_contract": proposed["reference"]})
        restored = receive("project")["summary"]
        assert restored["accepted"] and restored["contract"] == contract
        send({"project_status": None})
        current = receive("project")["summary"]
        assert current["verified"] and current["reference"] == pending["reference"]
        assert current["artifact_sha256"] == report["artifact_sha256"]
        process.stdin.close()
        process.wait(timeout=15)
        assert process.returncode == 0, process.stderr.read()
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=5)


@pytest.mark.parametrize(
    "fault", ["out-of-range", "inconsistent-example", "incorrect-overflow-example"]
)
def test_contracts_must_fit_native_outputs_and_their_own_examples(tmp_path, fault):
    contract = json.loads((ROOT / "verification/examples/counter.contract.json").read_text())
    contract["properties"] = []
    mismatch = None
    if fault == "out-of-range":
        contract["specification"] = "{ value := s.value + amount }"
        contract["examples"] = []
    elif fault == "inconsistent-example":
        contract["examples"] = [{"before": ["100"], "arguments": ["1000"], "after": ["100"]}]
        mismatch = "{ value := 1100 } = { value := 100 }"
    else:
        contract["examples"] = [
            {"before": ["1844674407370955"], "arguments": ["2"], "after": ["1844674407370955"]}
        ]
        mismatch = "{ value := 1844674407370957 } = { value := 1844674407370955 }"
    candidate = (ROOT / "verification/examples/counter.rs").read_text()
    result, output = verify_source(tmp_path, candidate, contract)
    assert result.returncode != 0
    assert "meden-check: required output bounds and example consistency" in result.stderr
    assert "Spec.lean" in result.stderr
    if mismatch:
        assert (
            "computed mathematical state (left) = declared expected state (right)"
            in result.stderr
        )
        assert mismatch in result.stderr
    assert not any(
        text in result.stderr.casefold()
        for text in [
            "unknown identifier",
            "unknown module",
            "maximum number of heartbeats",
            "maximum recursion depth",
            "stack overflow",
        ]
    ), result.stderr
    assert "meden-check: tests" not in result.stderr
    assert "meden-check: extraction" not in result.stderr
    assert not output.exists()

    # The conversational path must refuse the draft before exposing host review,
    # even if the next model response falsely announces successful verification.
    args, env, _ = setup(tmp_path, contract=contract, rejected=True)
    result = subprocess.run(
        args,
        input=json.dumps({"instruction": "Build a verified counter."}) + "\n",
        capture_output=True,
        text=True,
        env=env,
        timeout=120,
    )
    assert result.returncode == 0, result.stderr
    events = [json.loads(line) for line in result.stdout.splitlines()]
    proposal = next(
        e for e in events if e["type"] == "tool_result" and e["tool"] == "propose_contract"
    )
    assert proposal["error"]
    assert "meden-check: required output bounds and example consistency" in proposal["text"]
    if mismatch:
        assert mismatch in proposal["text"]
    assert all(
        not any(e["summary"][flag] for flag in ["proposed", "accepted", "verified"])
        and e["summary"]["artifact"] is None
        for e in events
        if e["type"] == "project"
    )
    summary = next(e["summary"] for e in reversed(events) if e["type"] == "project")
    # Writes clear stale implementation failures, but cannot erase the
    # conversational observation that the proposal itself still needs repair.
    assert summary["verification_failure"] is None
    assert next(e for e in events if e["type"] == "done")["outcome"] == "incomplete"
    assert sum(e["type"] == "asking" for e in events) == 8


def test_failed_proposal_and_later_edits_return_to_contract_review_without_accepting(tmp_path):
    contract = json.loads((ROOT / "verification/examples/counter.contract.json").read_text())
    invalid = copy.deepcopy(contract)
    invalid["specification"] = "{ value := s.value + amount }"
    invalid["properties"] = []
    invalid["examples"] = []
    args, env, _ = setup(tmp_path, contract=invalid, replacement=contract, rejected=True)
    result = subprocess.run(
        args,
        input=json.dumps({"instruction": "Build a counter that rejects overflow."}) + "\n",
        capture_output=True,
        text=True,
        env=env,
        timeout=120,
    )
    assert result.returncode == 0, result.stderr
    events = [json.loads(line) for line in result.stdout.splitlines()]
    proposals = [
        e for e in events if e["type"] == "tool_result" and e["tool"] == "propose_contract"
    ]
    assert [e["error"] for e in proposals] == [True, False]
    assert any(
        e["type"] == "tool_result" and e["tool"] == "write_file" and not e["error"]
        for e in events
    )
    assert sum(e["type"] == "asking" for e in events) == 3
    assert next(e for e in events if e["type"] == "done")["outcome"] == "needs_review"
    summary = next(e["summary"] for e in reversed(events) if e["type"] == "project")
    assert summary["proposed"] and summary["contract"] == contract
    assert not summary["accepted"] and not summary["verified"]
    assert summary["artifact"] is None and summary["verification_failure"] is None


@pytest.mark.parametrize(
    "attack",
    [
        "tactic",
        "command",
        "unsafe",
        "io",
        "projection",
        "punning",
        "nested-label",
        "ellipsis",
        "hole",
        "axiom",
        "property-tactic",
        "field-as-function",
        "rust-max",
    ],
)
def test_contract_terms_cannot_execute_lean_code(tmp_path, attack):
    contract = json.loads((ROOT / "verification/examples/counter.contract.json").read_text())
    terms = {
        "tactic": "by exact s",
        "command": 's\n#eval IO.println "bad"',
        "unsafe": "unsafeCast s",
        "io": "IO.Process.exit 0",
        "projection": "s.value.succ",
        "punning": "{ value }",
        "nested-label": "{ value.bad := 0 }",
        "ellipsis": "{ value := 0, .. }",
        "hole": "_",
        "axiom": "sorryAx _ false",
        "property-tactic": contract["specification"],
        "field-as-function": "dbgTrace 0 s",
        "rust-max": "{ value := u64::MAX }",
    }
    contract["specification"] = terms[attack]
    candidate = (ROOT / "verification/examples/counter.rs").read_text()
    if attack == "property-tactic":
        contract["properties"][0]["statement"] = "by trivial"
    if attack == "field-as-function":
        contract["fields"] = ["dbgTrace"]
        candidate = candidate.replace("value", "dbgTrace")
    result, output = verify_source(tmp_path, candidate, contract)
    assert result.returncode != 0
    assert "meden-check: contract syntax" in result.stderr, result.stderr
    label = (
        f"properties.{contract['properties'][0]['name']}.statement"
        if attack == "property-tactic"
        else "specification"
    )
    rejected = (
        contract["properties"][0]["statement"]
        if attack == "property-tactic"
        else contract["specification"]
    )
    assert f"Invalid {label}" in result.stderr, result.stderr
    quoted = "Rejected term (JSON string): " + json.dumps(rejected, ensure_ascii=False)
    assert quoted in result.stderr
    assert "Allowed expression names: s, Nat" in result.stderr
    assert "s." + contract["fields"][0] in result.stderr
    assert "18446744073709551615" in result.stderr
    assert "meden-check: contract properties" not in result.stderr, result.stderr
    assert "meden-check: tests" not in result.stderr
    assert not output.exists()


@pytest.mark.parametrize(
    "variant", ["line-comment", "global-named-label", "global-named-parameter"]
)
def test_supported_terms_keep_label_and_bound_variable_meaning(tmp_path, variant):
    contract = json.loads((ROOT / "verification/examples/counter.contract.json").read_text())
    if variant == "line-comment":
        contract["specification"] += " -- trailing comment"
        candidate = (ROOT / "verification/examples/counter.rs").read_text()
    else:
        contract["properties"] = []
        contract["examples"] = []
        if variant == "global-named-label":
            contract["fields"] = ["dbgTrace"]
            contract["specification"] = "{ dbgTrace := s.dbgTrace }"
            candidate = (
                "pub struct Counter {pub dbgTrace:u64}\n"
                "pub fn increment(s:Counter,amount:u64)->Counter{s}"
            )
        else:
            contract["parameters"] = ["dbgTrace"]
            contract["specification"] = "{ value := s.value + (dbgTrace - dbgTrace) }"
            candidate = (
                "pub struct Counter {pub value:u64}\n"
                "pub fn increment(s:Counter,dbgTrace:u64)->Counter{s}"
            )
    result, _ = verify_source(tmp_path, candidate, contract)
    assert result.returncode == 0, result.stderr
    assert_artifact(Path(json.loads(result.stdout)["directory"]), candidate, contract)


@pytest.mark.parametrize("shape", ["zero-parameters", "eight-fields-and-parameters"])
def test_generic_api_boundaries_preserve_field_and_parameter_correspondence(tmp_path, shape):
    if shape == "zero-parameters":
        contract = {
            "name": "Swap",
            "description": "Exchange both values.",
            "source_path": "swap.rs",
            "state": "Pair",
            "fields": ["left", "right"],
            "parameters": [],
            "function": "swap",
            "specification": "{ left := s.right, right := s.left }",
            "properties": [
                {
                    "name": "involution",
                    "description": "Two swaps restore the input.",
                    "statement": "transition out = s",
                }
            ],
            "examples": [{"before": ["1", "2"], "arguments": [], "after": ["2", "1"]}],
            "assumptions": [],
        }
        candidate = (
            "pub struct Pair {pub left:u64,pub right:u64}\n"
            "pub fn swap(s:Pair)->Pair{Pair{left:s.right,right:s.left}}"
        )
    else:
        fields = ["v" + str(i) for i in range(8)]
        parameters = list("abcdefgh")
        contract = {
            "name": "Register bank",
            "description": "Load eight registers in order.",
            "source_path": "registers.rs",
            "state": "Registers",
            "fields": fields,
            "parameters": parameters,
            "function": "load",
            "specification": "{ "
            + ", ".join(f"{f} := {p}" for f, p in zip(fields, parameters, strict=True))
            + " }",
            "properties": [
                {
                    "name": "loaded",
                    "description": "Every register has its selected input.",
                    "statement": " ∧ ".join(
                        f"out.{f} = {p}" for f, p in zip(fields, parameters, strict=True)
                    ),
                }
            ],
            "examples": [
                {
                    "before": ["0"] * 8,
                    "arguments": list(map(str, range(1, 9))),
                    "after": list(map(str, range(1, 9))),
                }
            ],
            "assumptions": [],
        }
        candidate = (
            "pub struct Registers {" + ",".join(f"pub {f}:u64" for f in fields) + "}\n"
            "pub fn load(s:Registers,"
            + ",".join(f"{p}:u64" for p in parameters)
            + ") -> Registers {"
            "Registers{"
            + ",".join(f"{f}:{p}" for f, p in zip(fields, parameters, strict=True))
            + "}}"
        )
    result, _ = verify_source(tmp_path, candidate, contract)
    assert result.returncode == 0, result.stderr
    assert_artifact(Path(json.loads(result.stdout)["directory"]), candidate, contract)


def test_decimal_example_boundaries_reach_native_tests_and_lean_exactly(tmp_path):
    contract = json.loads((ROOT / "verification/examples/counter.contract.json").read_text())
    # Adjacent integers above the binary64 exact-integer boundary must remain
    # distinct, including a transfer to the exact u64 maximum and overflow.
    contract["examples"] = [
        {"before": ["9007199254740992"], "arguments": ["1"], "after": ["9007199254740993"]},
        {"before": ["18446744073709551614"], "arguments": ["1"], "after": ["18446744073709551615"]},
        {"before": ["18446744073709551615"], "arguments": ["1"], "after": ["18446744073709551615"]},
    ]
    candidate = (ROOT / "verification/examples/counter.rs").read_text()
    result, _ = verify_source(tmp_path, candidate, contract)
    assert result.returncode == 0, result.stderr
    directory = Path(json.loads(result.stdout)["directory"])
    assert_artifact(directory, candidate, contract)
    assert json.loads((directory / "contract.json").read_text())["examples"] == contract["examples"]
    assert "9007199254740993" in (directory / "Spec.lean").read_text()
    assert "18446744073709551615u64" in (directory / "lib.rs").read_text()
