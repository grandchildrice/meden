"""Host review binds the real receiver to an immutable workspace snapshot."""

import json
import os
import queue
import select
import subprocess
import threading
import time
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import pyte
import pytest

from meden_checks import find_meden


@contextmanager
def fixture(tmp_path, targets=4):
    source = tmp_path / "source"
    source.mkdir()
    original = b"\x00\xff" + b"CONFIDENTIAL-ORIGINAL\n" * 10
    (source / "payload.bin").write_bytes(original)
    deliveries, other_deliveries, requests = [], [], []

    class Other(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_POST(self):
            other_deliveries.append(self.rfile.read(int(self.headers["Content-Length"])))
            self.send_response(200)
            self.send_header("Content-Length", "0")
            self.end_headers()

    other = ThreadingHTTPServer(("127.0.0.1", 0), Other)
    other_target = f"http://127.0.0.1:{other.server_port}/unapproved"

    class Receiver(Other):
        def do_POST(self):
            deliveries.append(
                (
                    self.path,
                    dict(self.headers),
                    self.rfile.read(int(self.headers["Content-Length"])),
                )
            )
            if len(deliveries) == 3:
                self.close_connection = True
                return
            self.send_response(307 if len(deliveries) == 2 else 201)
            self.send_header("Content-Length", "0")
            self.send_header("Location", other_target)
            self.end_headers()

    receiver = ThreadingHTTPServer(("127.0.0.1", 0), Receiver)
    target = f"http://127.0.0.1:{receiver.server_port}/approved?exact=1"

    def call(name, arguments, index):
        return {
            "id": f"call-{index}",
            "type": "function",
            "function": {"name": name, "arguments": json.dumps(arguments)},
        }

    replies = [
        {
            "tool_calls": [
                call(
                    "propose_export",
                    {
                        "path": "payload.bin",
                        "target": other_target if index == 1 else target,
                        "approved": True,
                    },
                    index,
                )
                for index in range(targets)
            ]
        },
        {
            "tool_calls": [
                call("approve_export", {"approved": True}, "forged"),
                call("bash", {"command": "printf CHANGED > payload.bin"}, "change"),
            ]
        },
        {"content": "Export proposals await host review."},
    ]

    class Model(Other):
        def do_POST(self):
            requests.append(json.loads(self.rfile.read(int(self.headers["Content-Length"]))))
            reply = replies[len(requests) - 1]
            body = json.dumps(
                {"choices": [{"finish_reason": "stop", "message": {"role": "assistant", **reply}}]}
            ).encode()
            self.send_response(200)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

    model = ThreadingHTTPServer(("127.0.0.1", 0), Model)
    threads = []
    for server in [other, receiver, model]:
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        threads.append(thread)
    env = os.environ.copy()
    env.update(
        MEDEN_LLM_BASE_URL=f"http://127.0.0.1:{model.server_port}/v1",
        MEDEN_LLM_MODEL="fixture",
        MEDEN_LLM_API_KEY="fixture-key",
        HERMES_HOME=str(tmp_path / "absent"),
        XDG_DATA_HOME=str(tmp_path / "data"),
    )
    env.pop("OLLAMA_API_KEY", None)
    try:
        yield source, original, target, env, requests, deliveries, other_deliveries
    finally:
        for server in [other, receiver, model]:
            server.shutdown()
            server.server_close()
        for thread in threads:
            thread.join(timeout=5)


def test_cli_review_reject_cross_session_freeze_replay_redirect_and_uncertain_delivery(tmp_path):
    binary = find_meden()
    assert binary is not None
    with fixture(tmp_path) as (source, original, target, env, requests, deliveries, other):
        process = subprocess.Popen(
            [
                binary,
                "run",
                "--workspace",
                str(source),
                "--provider",
                "configured",
                "--approve-processor",
                f"{env['MEDEN_LLM_BASE_URL']}/chat/completions",
            ],
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

        reader = threading.Thread(target=read, daemon=True)
        reader.start()

        def receive(kind):
            while True:
                record = records.get(timeout=15)
                assert record["type"] != "eof", record
                if record["type"] == kind:
                    return record

        def send(command, response):
            process.stdin.write(json.dumps(command) + "\n")
            process.stdin.flush()
            return receive(response)

        try:
            process.stdin.write(json.dumps({"instruction": "Prepare exports for review."}) + "\n")
            process.stdin.flush()
            ready = receive("ready")
            assert receive("done")["outcome"] == "done"
            assert deliveries == other == []
            assert (Path(ready["workspace"]) / "payload.bin").read_bytes() == b"CHANGED"
            pending = send({"list_exports": None}, "exports")["exports"]
            assert len(pending) == 4
            assert all(item["state"] == "pending" for item in pending)
            references = [{"session": item["session"], "id": item["id"]} for item in pending]
            rejected = send(
                {"approve_export": {**references[0], "session": "other-session"}}, "export_result"
            )
            assert rejected["error"]
            assert deliveries == other == []
            assert send({"approve_export": references[0]}, "export_result")["error"]
            assert deliveries == other == []
            review = send({"review_export": references[0]}, "export_review")
            assert review["method"] == "POST"
            assert review["content_type"] == "application/octet-stream"
            assert review["summary"]["target"] == target
            assert bytes.fromhex(review["body_hex"]) == original
            assert deliveries == []
            sent = send({"approve_export": references[0]}, "export_result")
            assert sent["http_status"] == 201
            assert sent["summary"]["state"] == "response_received"
            assert send({"approve_export": references[0]}, "export_result")["error"]
            assert (
                send({"reject_export": references[1]}, "export_result")["summary"]["state"]
                == "rejected"
            )
            assert send({"approve_export": references[1]}, "export_result")["error"]
            send({"review_export": references[2]}, "export_review")
            assert send({"approve_export": references[2]}, "export_result")["http_status"] == 307
            send({"review_export": references[3]}, "export_review")
            uncertain = send({"approve_export": references[3]}, "export_result")
            assert uncertain["summary"]["state"] == "delivery_uncertain"
            assert uncertain["http_status"] is None
            assert send({"approve_export": references[3]}, "export_result")["error"]
            assert [body for _, _, body in deliveries] == [original] * 3
            assert all(
                path == "/approved?exact=1"
                and headers["content-type"] == "application/octet-stream"
                for path, headers, _ in deliveries
            )
            assert other == []
            assert len(requests) == 3
            assert (source / "payload.bin").read_bytes() == original
            process.stdin.close()
            process.wait(timeout=5)
            assert process.returncode == 0, process.stderr.read()
        finally:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)
            reader.join(timeout=5)


@pytest.mark.skipif(os.name != "posix", reason="PTY/termios required")
def test_tui_export_review_uses_host_keys_and_exact_frozen_bytes(tmp_path):
    import codecs
    import fcntl
    import pty
    import signal
    import struct
    import termios

    binary = find_meden()
    assert binary is not None
    with fixture(tmp_path, targets=2) as (
        source,
        original,
        target,
        env,
        requests,
        deliveries,
        other,
    ):
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 36, 112, 0, 0))
        initial = termios.tcgetattr(slave)
        process = subprocess.Popen(
            [binary, "tui", "--workspace", str(source), "--provider", "configured"],
            stdin=slave,
            stdout=slave,
            stderr=slave,
            env={**env, "TERM": "xterm-256color"},
        )
        screen = pyte.Screen(112, 36)
        stream = pyte.Stream(screen)
        decoder = codecs.getincrementaldecoder("utf-8")("replace")

        def view():
            return "\n".join(screen.display)

        def wait_for(predicate):
            deadline = time.monotonic() + 15
            while time.monotonic() < deadline:
                if predicate():
                    return
                assert process.poll() is None, view()
                if select.select([master], [], [], 0.1)[0]:
                    # CI can split a draw between its fixed heading and body.
                    stream.feed(decoder.decode(os.read(master, 17)))
            pytest.fail(view())

        try:
            wait_for(lambda: "choose" in view())
            os.write(master, b"\r")
            wait_for(lambda: "processor approval" in view())
            os.write(master, b"y")
            wait_for(lambda: "alt-enter" in view())
            os.write(master, b"Prepare exports for review.\r")
            wait_for(lambda: len(requests) == 3 and "alt-enter" in view())
            assert deliveries == other == []
            os.write(master, b"/exports\r")
            wait_for(
                lambda: "immutable export review" in view()
                and "00 ff 43 4f 4e 46 49 44" in view()
            )
            assert target in view()
            assert "Method: POST" in view()
            assert "application/octet-stream" in view()
            assert "00 ff 43 4f 4e 46 49 44" in view()
            # The byte-page update acknowledges that Enter and a pasted 'y'
            # were processed without dispatching. No timing-only assertion.
            os.write(master, b"\r\x1b[200~y\x1b[201~\x1b[C")
            wait_for(lambda: "Byte offset: 128" in view())
            assert target in view()
            assert deliveries == other == []
            # A painted but clipped authority heading cannot authorize a send.
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 16, 20, 0, 0))
            screen.resize(16, 20)
            os.kill(process.pid, signal.SIGWINCH)
            wait_for(lambda: "Approval" in view() and "unavailable:" in view())
            os.write(master, b"yn")
            wait_for(lambda: "rejected." in view())
            assert deliveries == other == []
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 36, 112, 0, 0))
            screen.resize(36, 112)
            os.kill(process.pid, signal.SIGWINCH)
            wait_for(lambda: "Export #0 rejected." in view() and "alt-enter" in view())
            os.write(master, b"/exports 1\r")
            wait_for(lambda: "immutable export review" in view() and "Export #1" in view())
            os.write(master, b"y")
            wait_for(lambda: "response_received" in view() and "alt-enter" in view())
            assert other == [original]
            assert deliveries == []
            os.write(master, b"/exports 1\r")
            wait_for(lambda: "immutable export review" in view())
            os.write(master, b"y")
            wait_for(lambda: "no replay" in view() and "alt-enter" in view())
            assert other == [original]
            assert len(requests) == 3
            assert (source / "payload.bin").read_bytes() == original
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
