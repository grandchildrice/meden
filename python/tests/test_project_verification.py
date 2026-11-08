"""Real project verification; CI's Lean job supplies the prepared offline bundle."""

import hashlib
import json
import os
import queue
import select
import subprocess
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
ENGINE = r"""
import json, socketserver, sys
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
            reply={'tool_calls':[call('propose_ledger',{}),
                call('accept_contract',{'accepted':True}),
                call('verify_ledger',{'passed':True,'artifact':'fake.rlib'})]}
        elif index == 1:
            reply={'tool_calls':[call('write_file',{'path':'ledger.rs','content':NOOP}),
                call('write_file',{'path':'Spec.lean','content':'axiom everything : False'}),
                call('write_file',{'path':'Cargo.toml','content':'this is deliberately invalid'}),
                call('write_file',{'path':'build.rs','content':'panic!("must not execute")'})]}
        elif index == 2: reply={'content':'Everything is formally verified.'}
        elif index == 3:
            assert 'Host independent verification has not established' in json.dumps(body)
            assert 'not verified' in json.dumps(body)
            reply={'tool_calls':[call('write_file',{'path':'ledger.rs','content':ALTERNATIVE})]}
        elif index == 4: reply={'content':'Repaired implementation.'}
        else: raise AssertionError('unexpected inference request')
        index += 1
        data=json.dumps({'choices':[{'finish_reason':'stop',
            'message':{'role':'assistant',**reply}}]}).encode()
        self.send_response(200);self.send_header('Content-Length',str(len(data)));self.end_headers()
        self.wfile.write(data)
socketserver.UnixStreamServer(sys.argv[sys.argv.index('--host')+1],Handler).serve_forever()
"""


def setup(tmp_path):
    binary = find_meden()
    assert binary
    source = tmp_path / "source"
    source.mkdir()
    engine = tmp_path / "engine"
    engine.write_text("#!/bin/python3\n" + f"NOOP={NOOP!r}\nALTERNATIVE={ALTERNATIVE!r}\n" + ENGINE)
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


def assert_artifact(directory):
    report = json.loads((directory / "report.json").read_text())
    assert len(report["audit"]) == 8
    assert "actual_refines" in report["audit"][0]
    assert all("sorryAx" not in line for line in report["audit"])
    assert (directory / "ledger.rs").read_text() == ALTERNATIVE
    assert (directory / "libmeden_ledger.rlib").read_bytes().startswith(b"!<arch>\n")
    for name, digest in report["files"].items():
        assert hashlib.sha256((directory / name).read_bytes()).hexdigest() == digest
    manifest = json.dumps(report["files"], sort_keys=True, separators=(",", ":")).encode()
    assert hashlib.sha256(manifest).hexdigest() == report["artifact_sha256"]
    assert "axiom everything" not in (directory / "Spec.lean").read_text()
    assert "deliberately invalid" not in (directory / "Cargo.toml").read_text()
    return report


def test_cli_host_acceptance_independent_repair_artifact_binding_and_staleness(tmp_path):
    args, env, source = setup(tmp_path)
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
        summary = [r["summary"] for r in seen if r["type"] == "project"][-1]
        assert summary["verified"]
        report = assert_artifact(Path(summary["artifact"]))
        assert summary["source_sha256"] == report["source_sha256"]
        assert summary["artifact_sha256"] == report["artifact_sha256"]
        assert not (source / "ledger.rs").exists()
        copy = Path(ready["workspace"])
        (copy / "ledger.rs").write_text(ALTERNATIVE + "\n")
        send({"project_status": None})
        assert not receive("project")["summary"]["verified"]
        assert_artifact(Path(summary["artifact"]))
        # Restoring an old source does not restore discarded evidence progress.
        (copy / "ledger.rs").write_text(ALTERNATIVE)
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


def test_lean_rejects_bug_that_passes_fixed_examples(tmp_path):
    binary = find_meden()
    assert binary
    source = tmp_path / "ledger.rs"
    source.write_text(
        (ROOT / "verification/ledger/ledger.rs")
        .read_text()
        .replace("    if actor != 0", "    if amount == 4 { return state; }\n    if actor != 0")
    )
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
