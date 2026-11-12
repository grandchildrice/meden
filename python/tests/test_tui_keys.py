"""Codex-style keys through a real PTY; the HTTP model and login are fixtures."""

import base64
import codecs
import json
import os
import re
import select
import shutil
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import pyte
import pytest
from test_subscriptions import environment

from meden_checks import find_meden

pytestmark = pytest.mark.ux


def test_modified_enter_commands_history_clipboard_and_terminal_restoration(tmp_path, tui_verifier):
    import fcntl
    import pty
    import struct
    import termios

    binary = find_meden()
    assert binary is not None
    requests = []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_POST(self):
            requests.append(json.loads(self.rfile.read(int(self.headers["Content-Length"]))))
            content = f"ANSWER-{len(requests)}"
            if len(requests) == 6:
                content = "\n".join(f"scroll-row-{i:02}" for i in range(50)) + "\n" + content
            payload = json.dumps({"choices": [{"message": {"content": content},
                                               "finish_reason": "stop"}]}).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    base = f"http://127.0.0.1:{server.server_port}/v1"
    env = environment(tmp_path, "codex", base)
    env.update(MEDEN_LLM_BASE_URL=base, MEDEN_LLM_MODEL="fixture", TERM="xterm-256color")
    # First-launch configuration is unnecessary when the managed installation
    # exists: exercise discovery with no verifier environment variable.
    managed = (Path(env["XDG_DATA_HOME"]) / "meden/verifier"
               / "lean-4.35.0-aeneas-c5eb4a2-rust-2026-09-17/bundle.json")
    managed.parent.mkdir(parents=True)
    shutil.copyfile(tui_verifier, managed)
    env.pop("MEDEN_VERIFIER_BUNDLE", None)
    project = tmp_path / "project"
    project.mkdir()
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 32, 120, 0, 0))
    initial = termios.tcgetattr(slave)
    process = subprocess.Popen(
        [binary, "tui", "--workspace", str(project), "--provider", "configured"],
        stdin=slave, stdout=slave, stderr=slave, env=env,
    )
    screen = pyte.Screen(120, 32)
    terminal = pyte.Stream(screen)
    decoder = codecs.getincrementaldecoder("utf-8")("replace")
    output = bytearray()

    def view():
        return "\n".join("".join(screen.buffer[y][x].data for x in range(screen.columns))
                         for y in range(screen.lines))

    def wait_for(predicate):
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            if predicate():
                return
            assert process.poll() is None, view()
            if select.select([master], [], [], .05)[0]:
                data = os.read(master, 65536)
                output.extend(data)
                terminal.feed(decoder.decode(data))
        pytest.fail(view())

    def answer(number):
        wait_for(lambda: f"ANSWER-{number}" in view() and "shift+enter" in view())
        assert len(requests) == number

    try:
        wait_for(lambda: "Connection details" in view())
        assert b"\x1b[>1u" in output, "modified-key reporting was not enabled"
        assert b"\x1b[?1006h" in output, "mouse reporting was not enabled"
        os.write(master, b"\x1b[13;2u\n\x1b[200~y\x1b[201~\r")
        os.write(master, b"y")
        wait_for(lambda: "shift+enter" in view())
        assert not requests, "new edit keys or pasted y granted inference"
        os.write(master, b"/proofs\r")
        wait_for(lambda: "No project specification yet" in view())
        assert not requests, "evidence inspection called the model"
        os.write(master, b"\x1b")
        wait_for(lambda: "No project specification yet" not in view())
        os.write(master, b"\x07")
        wait_for(lambda: "No project specification yet" in view())
        os.write(master, b"\x07")
        wait_for(lambda: "No project specification yet" not in view())
        os.write(master, b"hello\x1b[13;2u" + "日本語".encode() + b"\nthird")
        wait_for(lambda: "third" in view())
        assert not requests, "Shift+Enter or Ctrl+J submitted the draft"
        os.write(master, b"\r")
        answer(1)
        assert requests[0]["messages"][-1]["content"][0]["text"] == "hello\n日本語\nthird"

        # Bracketed paste cannot invoke the clipboard shortcut.
        os.write(master, b"\x1b[200~\x0f\x1b[201~\x15\x0f")
        copied = b"\x1b]52;c;" + base64.b64encode(b"ANSWER-1") + b"\x07"
        wait_for(lambda: copied in output)
        assert output.count(b"\x1b]52;c;") == 1
        os.write(master, b"\x0c")
        wait_for(lambda: "ANSWER-1" not in view())
        os.write(master, b"followup\r")
        answer(2)
        assert len(requests[1]["messages"]) == 4, "Ctrl+L discarded conversation context"

        os.write(master, b"unsent\x12hello")
        wait_for(lambda: "history search: hello" in view())
        assert len(requests) == 2
        os.write(master, b"\x1b")
        wait_for(lambda: "> unsent" in view() and "history search:" not in view())
        os.write(master, b"\x03")
        wait_for(lambda: "> unsent" not in view())
        assert process.poll() is None, "Ctrl+C should clear a nonempty draft"
        os.write(master, b"\x12follow\r")
        wait_for(lambda: "> followup" in view() and "history search:" not in view())
        assert len(requests) == 2, "accepting a history match sent it to the model"
        os.write(master, b"\r")
        answer(3)

        os.write(master, b"\t")
        wait_for(lambda: "shift+enter" in view())
        os.write(master, b"/mo")
        wait_for(lambda: "Commands ·" in view())
        os.write(master, b"\t")
        wait_for(lambda: "> /model" in view() and "Commands ·" not in view())
        os.write(master, b"\r")
        wait_for(lambda: "Choose a model connection" in view())
        os.write(master, b"\x1b")
        wait_for(lambda: "shift+enter" in view())
        os.write(master, b"/status\r")
        wait_for(lambda: "Connection: configured" in view())
        assert len(requests) == 3
        os.write(master, b"/new\r")
        wait_for(lambda: "New chat started." in view())
        assert "ANSWER-3" in view(), "/new should preserve the visible transcript"
        os.write(master, b"fresh\r")
        answer(4)
        assert len(requests[3]["messages"]) == 2
        os.write(master, b"/clear\r")
        wait_for(lambda: "Conversation cleared." in view())
        assert "ANSWER-4" not in view()
        os.write(master, b"fresh again\r")
        answer(5)
        assert len(requests[4]["messages"]) == 2
        os.write(master, b"scroll fixture\r")
        answer(6)
        os.write(master, b"draft survives scrolling")
        wait_for(lambda: "> draft survives scrolling" in view())

        def last_visible_row():
            return max((int(n) for n in re.findall(r"scroll-row-(\d\d)", view())), default=-1)

        def first_visible_row():
            return min((int(n) for n in re.findall(r"scroll-row-(\d\d)", view())), default=50)

        bottom_first = first_visible_row()

        # Real SGR wheel reports must scroll the transcript, without recalling
        # old prompts into the composer or making an inference request.
        os.write(master, b"\x1b[<64;50;10M")
        wait_for(lambda: 0 <= last_visible_row() < 49)
        assert "> draft survives scrolling" in view() and len(requests) == 6
        os.write(master, b"\x1b[<65;50;10M")
        wait_for(lambda: first_visible_row() == bottom_first)
        for up, down in [(b"\x1b[5~", b"\x1b[6~"), (b"\x1b[1;2A", b"\x1b[1;2B")]:
            os.write(master, up)
            wait_for(lambda: first_visible_row() < bottom_first)
            assert "> draft survives scrolling" in view()
            os.write(master, down)
            wait_for(lambda: first_visible_row() == bottom_first)

        os.write(master, b"\x1b[1;5H")  # Ctrl+Home: oldest transcript row.
        wait_for(lambda: "Conversation cleared." in view())
        os.write(master, b"\x1b[<64;50;10M" * 30)
        os.write(master, b"\x1b[<65;50;10M")
        wait_for(lambda: "Conversation cleared." not in view())
        os.write(master, b"\x1b[1;5F")  # Ctrl+End: latest transcript row.
        wait_for(lambda: first_visible_row() == bottom_first)
        os.write(master, b"\x15/help\r")
        wait_for(lambda: "Keyboard shortcuts" in view())
        os.write(master, b"\x1b[<65;50;10M")
        wait_for(lambda: "Keyboard shortcuts" not in view())
        os.write(master, b"\x1b")
        wait_for(lambda: last_visible_row() == 49)
        os.write(master, b"/quit\r")
        assert process.wait(timeout=5) == 0
        # Drain the terminal restoration bytes after the child exits.
        while select.select([master], [], [], .05)[0]:
            output.extend(os.read(master, 65536))
        assert b"\x1b[<1u" in output, "keyboard mode was not restored"
        assert b"\x1b[?1006l" in output, "mouse reporting was not restored"
        assert termios.tcgetattr(slave) == initial
        assert len(requests) == 6, "scrolling or host commands were sent to inference"
        # Reopen the same repository: restore its visible conversation and API
        # context, while requiring a fresh processor approval before inference.
        process = subprocess.Popen(
            [binary, "tui", "--workspace", str(project), "--provider", "configured"],
            stdin=slave, stdout=slave, stderr=slave, env=env,
        )
        screen.reset()
        wait_for(lambda: "Connection details" in view())
        assert len(requests) == 6
        os.write(master, b"y")
        wait_for(lambda: "ANSWER-6" in view() and "shift+enter" in view())
        assert "Conversation restored." in view()
        os.write(master, b"resume fixture\r")
        answer(7)
        assert "scroll fixture" in json.dumps(requests[-1]["messages"])
        assert "ANSWER-6" in json.dumps(requests[-1]["messages"])
        os.write(master, b"/quit\r")
        assert process.wait(timeout=5) == 0
        assert termios.tcgetattr(slave) == initial
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=5)
        os.close(master)
        os.close(slave)
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
