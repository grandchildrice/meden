"""Exercise a real PTY, HTTP tool calls, edits, and a second user turn."""

import json
import os
import select
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest

from meden_agentdojo import find_meden


@pytest.mark.skipif(os.name != "posix", reason="PTY and termios test")
@pytest.mark.parametrize("allow_shell", [False, True])
def test_tui_edits_files_retains_context_and_restores_terminal(tmp_path, allow_shell):
    import fcntl
    import pty
    import struct
    import termios

    binary = find_meden()
    if binary is None:
        pytest.skip("meden binary not built")
    note = tmp_path / "note.txt"
    note.write_text("status: draft\n")
    requests = []
    replies = [
        ("read_file", {"path": "note.txt"}),
        ("edit_file", {"path": "note.txt", "old_text": "draft", "new_text": "ready"}),
    ]
    if allow_shell:
        replies.append(
            (
                "bash",
                {
                    "command": 'test "$(cat note.txt)" = "status: ready" '
                    "&& printf passed > check.txt"
                },
            )
        )
    replies.extend(["Updated note.txt.", "Status is ready.", "A fresh conversation."])
    first_turn_count = 4 if allow_shell else 3

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_POST(self):
            request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            reply = replies[len(requests)]
            requests.append(request)
            if isinstance(reply, tuple):
                name, args = reply
                message = {
                    "role": "assistant",
                    "content": None,
                    "tool_calls": [
                        {
                            "id": f"call-{len(requests)}",
                            "type": "function",
                            "function": {"name": name, "arguments": json.dumps(args)},
                        }
                    ],
                }
            else:
                message = {"role": "assistant", "content": reply}
            payload = json.dumps(
                {"choices": [{"message": message, "finish_reason": "stop"}]}
            ).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 120, 0, 0))
    initial = termios.tcgetattr(slave)
    env = {
        **os.environ,
        "TERM": "xterm-256color",
        "MEDEN_LLM_BASE_URL": f"http://127.0.0.1:{server.server_port}/v1",
        "MEDEN_LLM_MODEL": "fixture",
    }
    env.pop("MEDEN_TUI_SELFTEST", None)
    env.pop("MEDEN_LLM_API_KEY", None)
    command = [binary, "tui", "--workspace", str(tmp_path), "--provider", "configured"]
    if allow_shell:
        command.append("--allow-shell")
    process = subprocess.Popen(
        command,
        stdin=slave,
        stdout=slave,
        stderr=slave,
        env=env,
    )
    output = bytearray()

    def wait_for(predicate):
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            if predicate():
                return
            assert process.poll() is None, output.decode(errors="replace")
            if select.select([master], [], [], 0.1)[0]:
                output.extend(os.read(master, 65536))
        pytest.fail(f"TUI timed out: {output.decode(errors='replace')}")

    try:
        wait_for(lambda: b"fixture" in output)
        os.write(master, b"\r")
        wait_for(lambda: b"alt-enter" in output)
        # Correct a typo in the middle with cursor keys, then submit from End.
        os.write(
            master,
            b"Update the status to reXady." + b"\x1b[D" * 4 + b"\x7f\x1b[F\r",
        )
        wait_for(
            lambda: len(requests) == first_turn_count and note.read_text() == "status: ready\n"
        )
        wait_for(lambda: b"retained" in output)
        assert note.read_text() == "status: ready\n"
        assert requests[0]["messages"][-1]["content"][0]["text"] == (
            "Update the status to ready."
        )
        assert (
            "bash" in [tool["function"]["name"] for tool in requests[0]["tools"]]
        ) is allow_shell
        if allow_shell:
            assert (tmp_path / "check.txt").read_text() == "passed"
        # Bracketed paste must preserve Unicode and multiline input as one turn.
        follow_up = "Which status now?\n日本語で答えて。"
        os.write(master, b"\x1b[200~" + follow_up.encode() + b"\x1b[201~\r")
        wait_for(lambda: len(requests) == first_turn_count + 1)
        # A recorded HTTP request (or text from an earlier turn) does not
        # mean the UI is accepting input yet. Wait for the editor to return.
        wait_for(lambda: output.count(b"alt-enter") >= 3)
        messages = requests[-1]["messages"]
        assert len(messages) == (10 if allow_shell else 8)
        assert messages[-1]["role"] == "user"
        assert messages[-1]["content"][0]["text"] == follow_up
        assert any(message.get("tool_call_id") == "call-2" for message in messages)
        os.write(master, b"/clear\r")
        wait_for(lambda: b"cleared" in output)
        assert len(requests) == first_turn_count + 1
        os.write(master, b"Fresh turn.\r")
        wait_for(lambda: len(requests) == first_turn_count + 2)
        wait_for(lambda: output.count(b"alt-enter") >= 4)
        assert len(requests[-1]["messages"]) == 2
        assert requests[-1]["messages"][-1]["content"][0]["text"] == "Fresh turn."
        assert note.read_text() == "status: ready\n"
        os.write(master, b"\x03")
        process.wait(timeout=5)
        assert process.returncode == 0
        restored = termios.tcgetattr(slave)
        assert restored[3] & (termios.ICANON | termios.ECHO) == (
            initial[3] & (termios.ICANON | termios.ECHO)
        )
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=5)
        os.close(master)
        os.close(slave)
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
