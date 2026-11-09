"""Real checked artifacts, HTTP observation and current evidence at dispatch."""

import hashlib
import json
import os
import queue
import subprocess
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import pytest
from test_project_verification import ROOT, setup

pytestmark = pytest.mark.skipif(
    not os.environ.get("MEDEN_VERIFIER_BUNDLE"), reason="requires pinned offline checker bundle"
)

SCRIPT = r"""
import json,socketserver,sys
from http.server import BaseHTTPRequestHandler
index=0
class Handler(BaseHTTPRequestHandler):
    def log_message(self,*args): pass
    def do_GET(self):
        self.send_response(200);self.send_header('Content-Length','2');self.end_headers()
        self.wfile.write(b'{}')
    def do_POST(self):
        global index
        self.rfile.read(int(self.headers['Content-Length']))
        reply=REPLIES[index];index+=1
        data=json.dumps({'choices':[{'finish_reason':'stop','message':{'role':'assistant',**reply}}]}).encode()
        self.send_response(200);self.send_header('Content-Length',str(len(data)));self.end_headers()
        self.wfile.write(data)
socketserver.UnixStreamServer(sys.argv[sys.argv.index('--host')+1],Handler).serve_forever()
"""


def test_project_delivery_requires_current_evidence_and_frozen_checked_bytes(tmp_path):
    received = []

    class Receiver(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_POST(self):
            received.append((self.path, self.rfile.read(int(self.headers["Content-Length"]))))
            self.send_response(200)
            self.send_header("Content-Length", "0")
            self.end_headers()

    server = ThreadingHTTPServer(("127.0.0.1", 0), Receiver)
    worker = threading.Thread(target=server.serve_forever, daemon=True)
    worker.start()
    target = f"http://127.0.0.1:{server.server_port}/checked"
    contract = json.loads((ROOT / "verification/examples/counter.contract.json").read_text())
    candidate = (ROOT / "verification/examples/counter.rs").read_text()
    replacement = {**contract, "name": "Counter reviewed again"}

    def call(name, arguments):
        return {
            "id": name,
            "type": "function",
            "function": {"name": name, "arguments": json.dumps(arguments)},
        }

    artifact = call("propose_artifact_export", {"target": target})
    raw = call("propose_export", {"path": "notes.txt", "target": target})
    check = call("verify_project", {})
    replies = [
        {"tool_calls": [raw, call("propose_contract", {"contract": contract})]},
        {
            "tool_calls": [
                call("write_file", {"path": contract["source_path"], "content": candidate}),
                check,
                raw,
                call("propose_artifact_export", {"target": target, "verified": True}),
                artifact,
            ]
        },
        {"content": "Development finished."},
        {"tool_calls": [check, artifact]},
        {"content": "Fresh proof finished."},
        {"tool_calls": [call("propose_contract", {"contract": replacement})]},
        {"tool_calls": [check, artifact]},
        {"content": "Replacement proof finished."},
    ]
    args, env, source = setup(
        tmp_path, contract=contract, engine_script=f"REPLIES={replies!r}\n" + SCRIPT
    )
    (source / "notes.txt").write_text("CONFIDENTIAL-CANARY")
    process = subprocess.Popen(
        args,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
    )
    events = queue.Queue()
    seen = []

    def read():
        for line in process.stdout:
            events.put(json.loads(line))
        events.put({"type": "eof", "error": process.stderr.read()})

    threading.Thread(target=read, daemon=True).start()

    def receive(kind):
        while True:
            event = events.get(timeout=360)
            seen.append(event)
            assert event["type"] != "eof", event
            if event["type"] == kind:
                return event

    def send(command):
        process.stdin.write(json.dumps(command) + "\n")
        process.stdin.flush()

    def summary():
        return next(e["summary"] for e in reversed(seen) if e["type"] == "project")

    def reference(index):
        return {"session": summary()["reference"]["session"], "id": index}

    def review(index):
        send({"review_export": reference(index)})
        return receive("export_review")

    def approve(index, success=False):
        send({"approve_export": reference(index)})
        result = receive("export_result")
        if success:
            assert result["error"] is None and result["http_status"] == 200, result
        else:
            assert result["error"] and result["http_status"] is None, result

    def accept():
        ref = summary()["reference"]
        send({"review_contract": ref})
        receive("project")
        send({"accept_contract": ref})
        assert receive("project")["summary"]["accepted"]
        assert receive("done")["outcome"] == "done"
        assert summary()["verified"]

    try:
        send({"instruction": "Build a checked counter and propose delivery."})
        workspace = Path(receive("ready")["workspace"])
        assert receive("done")["outcome"] == "needs_review"
        accept()
        # A raw file queued before the contract cannot bypass its obligations.
        assert review(0)["summary"]["contract"] is None
        approve(0)
        assert received == []
        errors = [e for e in seen if e["type"] == "tool_result" and e["error"]]
        assert any(
            e["tool"] == "propose_export" and "checked artifact" in e["text"] for e in errors
        )
        assert any(e["tool"] == "propose_artifact_export" for e in errors)
        frozen = review(1)
        assert frozen["summary"]["contract"] == summary()["reference"]["contract"]
        package = json.loads(bytes.fromhex(frozen["body_hex"]))
        assert package["format"] == "meden-verified-kernel"
        files = {name: bytes(data) for name, data in package["files"].items()}
        report = json.loads(files["report.json"])
        assert set(files) == set(report["files"]) | {"report.json"}
        assert files["kernel.rs"] == candidate.encode()
        assert files["libmeden_kernel.rlib"].startswith(b"!<arch>\n")
        for name, digest in report["files"].items():
            assert hashlib.sha256(files[name]).hexdigest() == digest
        # Source changes after review discard coverage before any HTTP request.
        path = workspace / contract["source_path"]
        path.write_text(candidate + "\n")
        approve(1)
        path.write_text(candidate)
        send({"project_status": None})
        assert not receive("project")["summary"]["verified"]
        approve(1)
        assert received == []
        send({"instruction": "Obtain fresh independent verification and prepare delivery."})
        assert receive("done")["outcome"] == "done"
        assert summary()["verified"]
        delivered = review(2)
        approve(2, success=True)
        assert received == [("/checked", bytes.fromhex(delivered["body_hex"]))]
        approve(2)
        assert len(received) == 1
        # A pending or newly accepted replacement cannot reuse the old binding.
        send({"instruction": "Propose the replacement for review."})
        assert receive("done")["outcome"] == "needs_review"
        approve(1)
        accept()
        approve(1)
        assert len(received) == 1
        review(3)
        library = Path(summary()["artifact"]) / "libmeden_kernel.rlib"
        original = library.read_bytes()
        library.write_bytes(b"tampered:" + original)
        approve(3)
        library.write_bytes(original)
        send({"project_status": None})
        assert not receive("project")["summary"]["verified"]
        approve(3)
        assert len(received) == 1
        process.stdin.close()
        process.wait(timeout=10)
        assert process.returncode == 0, process.stderr.read()
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=5)
        server.shutdown()
        server.server_close()
        worker.join(timeout=5)
