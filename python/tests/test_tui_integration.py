"""Exercise a real PTY, HTTP tool calls, edits, and a second user turn."""

import json
import os
import select
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import pytest

from meden_checks import find_meden


@pytest.mark.skipif(os.name != "posix", reason="PTY and termios test")
def test_tui_edits_files_retains_context_and_restores_terminal(tmp_path):
    import fcntl
    import pty
    import struct
    import termios

    binary = find_meden()
    if binary is None:
        pytest.skip("meden binary not built")
    project = tmp_path / "project"
    project.mkdir()
    data = tmp_path / "data"
    source_note = project / "note.txt"
    note = source_note
    note.write_text("status: draft\n")
    cache = tmp_path / "pages.json"
    document = "Untrusted documentation: 日本語🙂 <script>fetch('/secret')</script>"
    cache.write_text(
        json.dumps(
            {
                "pages": [
                    {
                        "url": "https://docs.example.test/",
                        "content_type": "text/html",
                        "body": document,
                    }
                ]
            }
        )
    )
    requests = []
    destinations = []
    replies = [
        ("read_cached_page", {"id": 0}),
        ("read_file", {"path": "note.txt"}),
        ("edit_file", {"path": "note.txt", "old_text": "draft", "new_text": "ready"}),
    ]
    replies.append(
        (
            "bash",
            {"command": 'test "$(cat note.txt)" = "status: ready" && printf passed > check.txt'},
        )
    )
    replies.extend(
        [
            "Updated note.txt.",
            "Status is ready.",
            "A fresh conversation.",
            ("read_file", {"path": "note.txt"}),
            "Read by the second processor.",
        ]
    )
    first_turn_count = 5

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_POST(self):
            request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            reply = replies[len(requests)]
            requests.append(request)
            destinations.append(self.server.server_port)
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
    other = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    other_thread = threading.Thread(target=other.serve_forever, daemon=True)
    other_thread.start()
    hermes = tmp_path / "hermes"
    hermes.mkdir()
    (hermes / "config.yaml").write_text(
        "custom_providers:\n  - name: other\n"
        f"    base_url: http://127.0.0.1:{other.server_port}/v1\n"
        "    model: fixture-other\n"
    )
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 120, 0, 0))
    initial = termios.tcgetattr(slave)
    env = {
        **os.environ,
        "TERM": "xterm-256color",
        "XDG_DATA_HOME": str(data),
        "MEDEN_LLM_BASE_URL": f"http://127.0.0.1:{server.server_port}/v1",
        "MEDEN_LLM_MODEL": "fixture",
        "HERMES_HOME": str(hermes),
        "CODEX_HOME": str(tmp_path / "no-codex-login"),
        "CLAUDE_CONFIG_DIR": str(tmp_path / "no-claude-login"),
    }
    env.pop("MEDEN_LLM_API_KEY", None)
    env.pop("OPENAI_API_KEY", None)
    env.pop("OLLAMA_API_KEY", None)
    env.pop("CLAUDE_CODE_OAUTH_TOKEN", None)
    env.pop("ANTHROPIC_TOKEN", None)
    command = [
        binary,
        "tui",
        "--workspace",
        str(project),
        "--pages",
        str(cache),
        "--provider",
        "auto",
    ]
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
        pytest.fail(f"TUI timed out: {output[-8000:].decode(errors='replace')}")

    try:
        wait_for(lambda: b"fixture" in output)
        copies = list((data / "meden/workspaces").glob("session-*"))
        assert len(copies) == 1
        workspace = copies[0]
        note = workspace / "note.txt"
        os.write(master, b"\r")
        wait_for(lambda: b"host control" in output)
        assert requests == []
        # Enter and a pasted approval cannot grant processor authority.
        os.write(master, b"\r\x1b[200~y\x1b[201~")
        time.sleep(0.1)
        assert requests == []
        os.write(master, b"y")
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
        assert requests[0]["messages"][-1]["content"][0]["text"] == ("Update the status to ready.")
        assert "bash" in [tool["function"]["name"] for tool in requests[0]["tools"]]
        assert (workspace / "check.txt").read_text() == "passed"
        # Bracketed paste must preserve Unicode and multiline input as one turn.
        follow_up = "Which status now?\n日本語で答えて。"
        os.write(master, b"\x1b[200~" + follow_up.encode() + b"\x1b[201~\r")
        wait_for(lambda: len(requests) == first_turn_count + 1)
        # A recorded HTTP request (or text from an earlier turn) does not
        # mean the UI is accepting input yet. Wait for the editor to return.
        wait_for(lambda: output.count(b"alt-enter") >= 3)
        messages = requests[-1]["messages"]
        assert len(messages) == 12
        assert document in json.dumps(messages, ensure_ascii=False)
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
        assert source_note.read_text() == "status: draft\n"
        # The final picker entry is our second processor. Other locally
        # discovered servers can appear in between; no cloud keys are loaded.
        os.write(master, b"\t" + b"\x1b[B" * 100 + b"\r")
        wait_for(lambda: output.count(b"host control") >= 2)
        assert other.server_port not in destinations
        os.write(master, b"n")
        time.sleep(0.1)
        assert other.server_port not in destinations
        os.write(master, b"\r")
        wait_for(lambda: output.count(b"host control") >= 3)
        os.write(master, b"y")
        wait_for(lambda: output.count(b"alt-enter") >= 5)
        os.write(master, b"Read the current status.\r")
        wait_for(lambda: len(requests) == first_turn_count + 4)
        wait_for(lambda: output.count(b"alt-enter") >= 6)
        assert (
            destinations == [server.server_port] * (first_turn_count + 2) + [other.server_port] * 2
        )
        assert len(requests[-2]["messages"]) == 2
        assert "status: ready" in requests[-1]["messages"][-1]["content"][0]["text"]
        os.write(master, b"\x03")
        process.wait(timeout=5)
        assert process.returncode == 0
        restored = termios.tcgetattr(slave)
        assert restored[3] & (termios.ICANON | termios.ECHO) == (
            initial[3] & (termios.ICANON | termios.ECHO)
        )
        # The headless path must use the same real tools, requests and
        # conversation history as the terminal for these two user turns.
        tui_requests = list(requests[: first_turn_count + 1])
        requests.clear()
        destinations.clear()
        completed = subprocess.run(
            [
                binary,
                "run",
                "--workspace",
                str(project),
                "--pages",
                str(cache),
                "--provider",
                "configured",
                "--approve-processor",
                f"http://127.0.0.1:{server.server_port}/v1/chat/completions",
            ],
            input="\n".join(
                json.dumps({"instruction": text})
                for text in ["Update the status to ready.", follow_up]
            )
            + "\n",
            text=True,
            capture_output=True,
            env=env,
            timeout=20,
        )
        assert completed.returncode == 0, completed.stderr
        assert requests == tui_requests
        records = [json.loads(line) for line in completed.stdout.splitlines()]
        assert [r["outcome"] for r in records if r["type"] == "done"] == ["done", "done"]
        assert all(r["type"] != "call" for r in records)
        headless_copy = Path(records[0]["workspace"])
        assert headless_copy != workspace
        assert (headless_copy / "note.txt").read_text() == "status: ready\n"
        assert source_note.read_text() == "status: draft\n"
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=5)
        os.close(master)
        os.close(slave)
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
        other.shutdown()
        other.server_close()
        other_thread.join(timeout=5)
