"""Real project verification; CI's Lean job supplies the prepared offline bundle."""

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
        if index == 0:
            reply={'tool_calls':[call('accept_contract',{'accepted':True}),
                call('verify_ledger',{'passed':True,'artifact':'fake.rlib','failure_kind':'unavailable'}),
                call('propose_ledger',{})]}
        elif PROBE and index == 1:
            reply={'tool_calls':[call('write_file',{'path':'ledger.rs','content':ALTERNATIVE})]}
        elif PROBE and index == 3:
            reply={'tool_calls':[call('write_file',{'path':'ledger.rs','content':NOOP}),
                call('verify_ledger',{})]}
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
                calls += [call('verify_ledger',{}),
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
            reply={'tool_calls':[call('verify_ledger',{'passed':True,'artifact':'fake.rlib'})]}
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
        else: raise AssertionError('unexpected inference request')
        index += 1
        data=json.dumps({'choices':[{'finish_reason':'stop',
            'message':{'role':'assistant',**reply}}]}).encode()
        self.send_response(200);self.send_header('Content-Length',str(len(data)));self.end_headers()
        self.wfile.write(data)
socketserver.UnixStreamServer(sys.argv[sys.argv.index('--host')+1],Handler).serve_forever()
"""


def setup(tmp_path, exhaustion=False, candidate=ALTERNATIVE, unavailable=None, probe=None):
    binary = find_meden()
    assert binary
    source = tmp_path / "source"
    source.mkdir()
    engine = tmp_path / "engine"
    engine.write_text(
        "#!/bin/python3\n"
        + f"NOOP={NOOP!r}\nALTERNATIVE={candidate!r}\nEXHAUSTION={exhaustion!r}\n"
        + f"UNAVAILABLE={unavailable!r}\nPROBE={probe!r}\n"
        + ENGINE
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


def assert_artifact(directory, candidate=ALTERNATIVE):
    report = json.loads((directory / "report.json").read_text())
    assert len(report["audit"]) == 8
    assert "actual_refines" in report["audit"][0]
    assert all("sorryAx" not in line for line in report["audit"])
    assert (directory / "ledger.rs").read_text() == candidate
    assert (directory / "libmeden_ledger.rlib").read_bytes().startswith(b"!<arch>\n")
    for name, digest in report["files"].items():
        assert hashlib.sha256((directory / name).read_bytes()).hexdigest() == digest
    manifest = json.dumps(report["files"], sort_keys=True, separators=(",", ":")).encode()
    assert hashlib.sha256(manifest).hexdigest() == report["artifact_sha256"]
    assert "axiom everything" not in (directory / "Spec.lean").read_text()
    assert "deliberately invalid" not in (directory / "Cargo.toml").read_text()
    return report


@pytest.mark.parametrize("exhaustion", [False, True], ids=["false-done", "tool-budget"])
def test_cli_host_acceptance_independent_repair_artifact_binding_and_staleness(
    tmp_path, exhaustion
):
    candidate = checked_source("let_else") if exhaustion else ALTERNATIVE
    args, env, source = setup(tmp_path, exhaustion, candidate)
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
        send({"instruction": "Build a local ledger with conserved balances and safe retries."})
        ready = receive("ready")
        assert receive("done")["outcome"] == "needs_review"
        pending = [r["summary"] for r in seen if r["type"] == "project"][-1]
        assert pending["proposed"] and not pending["accepted"] and not pending["verified"]
        assert any(
            r["type"] == "decision" and r["tool"] == "accept_contract" and not r["allowed"]
            for r in seen
        )
        assert any(
            r["type"] == "tool_result" and r["tool"] == "verify_ledger" and r["error"] for r in seen
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
                and r["tool"] == "verify_ledger"
                and r["error"]
                and "Tool not executed: turn stopped" in r["text"]
                for r in seen
            )
        summary = [r["summary"] for r in seen if r["type"] == "project"][-1]
        assert summary["verified"]
        report = assert_artifact(Path(summary["artifact"]), candidate)
        assert summary["source_sha256"] == report["source_sha256"]
        assert summary["artifact_sha256"] == report["artifact_sha256"]
        assert not (source / "ledger.rs").exists()
        copy = Path(ready["workspace"])
        (copy / "ledger.rs").write_text(candidate + "\n")
        send({"project_status": None})
        assert not receive("project")["summary"]["verified"]
        assert_artifact(Path(summary["artifact"]), candidate)
        # Restoring an old source does not restore discarded evidence progress.
        (copy / "ledger.rs").write_text(candidate)
        send({"project_status": None})
        assert not receive("project")["summary"]["verified"]
        process.stdin.close()
        process.wait(timeout=10)
        assert process.returncode == 0, process.stderr.read()
    finally:
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
            if "Failed:" in rendered() and "alt-enter" in rendered():
                pytest.fail(rendered())
            if select.select([master], [], [], 0.1)[0]:
                chunk = os.read(master, 65536)
                output.extend(chunk)
                terminal.feed(chunk.decode(errors="replace"))
        pytest.fail(output[-8000:].decode(errors="replace"))

    try:
        wait_for(lambda: "alt-enter" in rendered())
        os.write(master, b"Build a local ledger with conserved balances and safe retries.\r")
        wait_for(lambda: "y: accept and continue" in rendered())
        os.write(master, b"\x1b[200~y\x1b[201~\r")
        # Process queued terminal output before inspecting the host state.
        deadline = time.monotonic() + 0.5
        while time.monotonic() < deadline:
            if select.select([master], [], [], 0.05)[0]:
                chunk = os.read(master, 65536)
                output.extend(chunk)
                terminal.feed(chunk.decode(errors="replace"))
        assert "y: accept and continue" in rendered()
        os.write(master, b"y")
        wait_for(lambda: "VERIFIED ledger kernel" in rendered() and "alt-enter" in rendered(), 360)
        artifacts = list((Path(env["XDG_DATA_HOME"]) / "meden/artifacts").glob("ledger-*"))
        assert len(artifacts) == 1
        assert_artifact(artifacts[0])
        os.write(master, b"\x03")
        process.wait(timeout=5)
        assert process.returncode == 0
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
    env["MEDEN_VERIFIER_BUNDLE"] = str(tmp_path / "absent-bundle.json")
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
        for command in ["review_contract", "accept_contract"]:
            process.stdin.write(json.dumps({command: reference}) + "\n")
        process.stdin.flush()
        done = until_done()
        assert done["outcome"] == "failed", seen
        assert "Verification Unavailable" in done["error"]
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
                if event["tool"] == "verify_ledger":
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


def verify_source(tmp_path, content):
    binary = find_meden()
    assert binary
    source = tmp_path / "ledger.rs"
    source.write_text(content)
    output = tmp_path / "artifacts"
    result = subprocess.run(
        [
            binary,
            "verify-ledger",
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


def test_native_counterexample_names_accepted_requirement(tmp_path):
    result, output = verify_source(
        tmp_path, ALTERNATIVE.replace("&& amount <= u64::MAX - state.credit", "")
    )
    assert result.returncode != 0
    assert "contract requirement: credit overflow leaves all state unchanged" in result.stderr
    assert "left: (8, 18446744073709551615, 1)" in result.stderr
    assert "right: (10, 18446744073709551614, 0)" in result.stderr
    assert "test result: FAILED" in result.stderr
    assert "meden-check: extraction" not in result.stderr
    assert not output.exists()


@pytest.mark.parametrize("fault", ["wrong-result", "unguarded-unwrap"])
def test_lean_rejects_bug_that_passes_fixed_examples(tmp_path, fault):
    source = (ROOT / "verification/ledger/ledger.rs").read_text()
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
    assert "error: scalar_tac failed to prove the goal" in result.stderr, result.stderr
    assert "'LedgerProof.actual_refines' depends on axioms: [propext, sorryAx" in result.stderr
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
