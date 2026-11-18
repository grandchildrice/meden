"""Native GUI adapter and the public TUI share files/history, never approval."""

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
from urllib.request import Request, urlopen

import pyte
import pytest

from meden_checks import find_meden

pytestmark = pytest.mark.ux


def test_gui_to_tui_and_back_retains_context_but_requires_fresh_review(tmp_path, tui_verifier):
    project = tmp_path / "project"
    project.mkdir()
    assets = tmp_path / "assets"
    assets.mkdir()
    # Only the HTTP adapter is used here. Real browser tests load the built GUI.
    (assets / "index.html").write_text("<title>GUI adapter fixture</title>")
    requests = []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_POST(self):
            data = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            requests.append(data)
            message = {"role": "assistant", "content": "GUI-TURN-SAVED"}
            if len(requests) == 2:
                message = {"role": "assistant", "content": None, "tool_calls": [{
                    "id": "resume-write", "type": "function", "function": {
                        "name": "write_file", "arguments": json.dumps({
                            "path": "continued.txt", "content": "written from the TUI",
                        }),
                    },
                }]}
            elif len(requests) == 3:
                message = {"role": "assistant", "content": "TUI-CONTINUED"}
            body = json.dumps({"choices": [{"message": message}]}).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    model = ["--base-url", f"http://127.0.0.1:{server.server_port}/v1",
             "--model", "resume-fixture", "--api-key", "GUI-TUI-KEY-NOT-SAVED"]
    env = {**os.environ, "TERM": "xterm-256color", "XDG_DATA_HOME": str(tmp_path / "data")}
    gui = None
    process = None
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 130, 0, 0))
    initial = termios.tcgetattr(slave)
    screen = pyte.Screen(130, 40)
    terminal = pyte.Stream(screen)
    decoder = codecs.getincrementaldecoder("utf-8")("replace")

    def launch_gui():
        process = subprocess.Popen([
            find_meden(), "gui", "--workspace", str(project), "--assets", str(assets),
            "--port", "0", *model,
        ], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env)
        assert select.select([process.stdout], [], [], 15)[0], "gateway URL missing"
        line = process.stdout.readline()
        assert line.startswith("Meden GUI: "), line
        url, token = line.removeprefix("Meden GUI: ").strip().split("#session=")
        return process, url.rstrip("/"), token

    def api(origin, token, command=None):
        request = Request(origin + ("/api/state" if command is None else "/api/command"),
                          data=None if command is None else json.dumps(command).encode(),
                          headers={"Authorization": f"Bearer {token}", "Origin": origin,
                                   "Content-Type": "application/json"})
        with urlopen(request, timeout=5) as response:
            return json.load(response)

    def wait_for(predicate):
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            if predicate():
                return
            assert process.poll() is None, "\n".join(screen.display)
            if select.select([master], [], [], .05)[0]:
                terminal.feed(decoder.decode(os.read(master, 65536)))
        pytest.fail("\n".join(screen.display))

    try:
        gui, origin, token = launch_gui()
        state = api(origin, token)
        assert not state["authorized"]
        api(origin, token, {"type": "approve_processor", "endpoint": state["endpoint"]})
        api(origin, token, {"type": "instruction", "text": "Remember the GUI project goal"})
        deadline = time.monotonic() + 15
        while api(origin, token)["running"]:
            assert time.monotonic() < deadline
            threading.Event().wait(.05)
        assert api(origin, token)["messages"][-1]["text"] == "GUI-TURN-SAVED"
        gui.terminate()
        assert gui.wait(timeout=5) == 0
        gui = None
        process = subprocess.Popen([
            find_meden(), "tui", "--workspace", str(project), *model,
        ], stdin=slave, stdout=slave, stderr=slave, env=env)
        wait_for(lambda: "Connection details" in "\n".join(screen.display))
        assert len(requests) == 1, "reopening must not send old context before review"
        os.write(master, b"y")
        wait_for(lambda: "GUI-TURN-SAVED" in "\n".join(screen.display)
                 and "shift+enter" in "\n".join(screen.display))
        os.write(master, b"Continue the GUI project in the TUI\r")
        wait_for(lambda: "TUI-CONTINUED" in "\n".join(screen.display)
                 and "shift+enter" in "\n".join(screen.display))
        assert "Remember the GUI project goal" in json.dumps(requests[1])
        assert (project / "continued.txt").read_text() == "written from the TUI"
        os.write(master, b"/quit\r")
        assert process.wait(timeout=5) == 0
        assert termios.tcgetattr(slave) == initial
        gui, origin, token = launch_gui()
        state = api(origin, token)
        assert not state["authorized"]
        assert any(message["text"] == "TUI-CONTINUED" for message in state["messages"])
        assert len(requests) == 3
        for checkpoint in (tmp_path / "data/meden/projects").glob("*/*.json"):
            assert "GUI-TUI-KEY-NOT-SAVED" not in checkpoint.read_text()
    finally:
        for child in (gui, process):
            if child is not None and child.poll() is None:
                child.terminate()
                child.wait(timeout=5)
        os.close(master)
        os.close(slave)
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
