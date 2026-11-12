"""Real terminal recovery, explicit continuation and host stop explanations."""

import codecs
import fcntl
import json
import os
import pty
import select
import struct
import subprocess
import termios
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pyte
import pytest

from meden_checks import find_meden


@pytest.mark.parametrize("mode", ["repair", "provider"])
def test_tui_recovers_tools_and_explains_retained_stops(tmp_path, tui_verifier, mode):
    project = tmp_path / "project"
    project.mkdir()
    requests = []
    errors = []
    allow_repair = threading.Event()

    def call(name, arguments):
        return {"id": f"call-{len(requests)}", "type": "function",
                "function": {"name": name, "arguments": json.dumps(arguments)}}

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_POST(self):
            try:
                request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                assert self.headers.get("Authorization") == "Bearer CLI-KEY-DO-NOT-SAVE"
                requests.append(request)
                index = len(requests)
                if mode == "provider" and index == 2:
                    self.send_response(429)
                    self.end_headers()
                    return
                message = {"role": "assistant", "content": None}
                if mode == "repair" and index == 1:
                    message["tool_calls"] = [call("bash", {
                        "command": "printf 'fixture compile failure' >&2; exit 1",
                    })]
                elif mode == "repair" and index == 2:
                    assert "fixture compile failure" in json.dumps(request["messages"])
                    assert allow_repair.wait(15)
                    message["tool_calls"] = [call("bash", {
                        "command": "printf 'fixed' > result.txt; test -s result.txt",
                    })]
                elif mode == "provider" and index == 1:
                    message["tool_calls"] = [call("write_file", {
                        "path": "result.txt", "content": "already completed",
                    })]
                elif mode == "provider" and index == 3:
                    history = json.dumps(request["messages"])
                    assert "build fixture" in history and "already completed" in history
                    assert "do not blindly repeat" in history
                    message["tool_calls"] = [call("read_file", {"path": "result.txt"})]
                else:
                    message["content"] = "RECOVERY-COMPLETE"
                body = json.dumps({"choices": [{"message": message}]}).encode()
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
            except Exception as error:
                errors.append(error)

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    env = {**os.environ, "TERM": "xterm-256color",
           "XDG_DATA_HOME": str(tmp_path / "data"),
           "MEDEN_LLM_BASE_URL": f"http://127.0.0.1:{server.server_port}/v1",
           "MEDEN_LLM_MODEL": "fixture"}
    args = [find_meden(), "tui", "--workspace", str(project),
            "--base-url", env["MEDEN_LLM_BASE_URL"], "--model", "fixture",
            "--api-key", "CLI-KEY-DO-NOT-SAVE"]
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 130, 0, 0))
    initial = termios.tcgetattr(slave)
    process = subprocess.Popen(args, stdin=slave, stdout=slave, stderr=slave, env=env)
    screen = pyte.Screen(130, 40)
    terminal = pyte.Stream(screen)
    decoder = codecs.getincrementaldecoder("utf-8")("replace")

    def view():
        return "\n".join(screen.display)

    def wait_for(predicate):
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            if predicate():
                return
            assert process.poll() is None, view()
            assert not errors, errors
            if select.select([master], [], [], .05)[0]:
                terminal.feed(decoder.decode(os.read(master, 65536)))
        pytest.fail(view())

    def quit_and_drain():
        os.write(master, b"/quit\r")
        deadline = time.monotonic() + 5
        while process.poll() is None and time.monotonic() < deadline:
            if select.select([master], [], [], .05)[0]:
                terminal.feed(decoder.decode(os.read(master, 65536)))
        assert process.wait(timeout=5) == 0
        assert termios.tcgetattr(slave) == initial

    try:
        wait_for(lambda: "Connection details" in view())
        os.write(master, b"y")
        wait_for(lambda: "shift+enter" in view())
        os.write(master, b"build fixture\r")
        if mode == "repair":
            wait_for(lambda: "Recovery:" in view() and len(requests) == 2)
            assert "shift+enter" not in view(), "tool error prematurely stopped work"
            allow_repair.set()
            wait_for(lambda: "RECOVERY-COMPLETE" in view() and "shift+enter" in view())
            assert (project / "result.txt").read_text() == "fixed"
            assert len(requests) == 3
        else:
            wait_for(lambda: "Stopped: Model usage limit reached" in view()
                     and "shift+enter" in view())
            assert "Repository edits remain in place" in view()
            assert "/retry" in view() and "Task stopped" in view()
            assert len(requests) == 2, "provider failure was resent automatically"
            assert (project / "result.txt").read_text() == "already completed"
            quit_and_drain()
            process = subprocess.Popen(args, stdin=slave, stdout=slave, stderr=slave, env=env)
            screen.reset()
            wait_for(lambda: "Connection details" in view())
            assert len(requests) == 2
            os.write(master, b"y")
            wait_for(lambda: "Stopped: Model usage limit reached" in view()
                     and "shift+enter" in view())
            assert "Previous turn stopped" in view()
            # Scrolling/host status do not resume inference. Continuation is
            # explicitly requested and starts a new bounded host turn.
            os.write(master, b"/status\r")
            wait_for(lambda: "Connection: configured" in view())
            assert len(requests) == 2
            os.write(master, b"/retry\r")
            wait_for(lambda: "RECOVERY-COMPLETE" in view() and "shift+enter" in view())
            assert "Task stopped" not in view()
            assert "Specification accepted" not in view()
            assert (project / "result.txt").read_text() == "already completed"
            assert len(requests) == 4
        assert not errors
        for checkpoint in (tmp_path / "data/meden/projects").glob("*/*.json"):
            assert "CLI-KEY-DO-NOT-SAVE" not in checkpoint.read_text()
        quit_and_drain()
    finally:
        allow_repair.set()
        if process.poll() is None:
            process.kill()
            process.wait(timeout=5)
        os.close(master)
        os.close(slave)
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
