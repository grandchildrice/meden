"""Real PTY streaming with gated HTTP fixtures and offline Bash.

No live provider account is used. Gates prove observations precede completion.
"""

import codecs
import json
import os
import select
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pyte
import pytest
from test_subscriptions import call, environment, native_reply

from meden_checks import find_meden

pytestmark = pytest.mark.ux


def frame(value):
    return f"data: {json.dumps(value, ensure_ascii=False)}\r\n\r\n".encode()


def response_parts(provider, text, calls=()):
    if provider == "codex":
        head = frame({"type": "response.reasoning_summary_text.delta", "delta": "Reviewing files."})
        head += frame({"type": "response.output_text.delta", "delta": text})
        return head, native_reply(provider, text, calls)
    if provider == "claude":
        head = frame({"type": "message_start", "message": {
            "type": "message", "role": "assistant", "content": [], "stop_reason": None,
        }})
        head += frame({"type": "content_block_start", "index": 0,
                       "content_block": {"type": "text", "text": ""}})
        head += frame({"type": "content_block_delta", "index": 0,
                       "delta": {"type": "text_delta", "text": text}})
        tail = frame({"type": "content_block_stop", "index": 0})
        for index, item in enumerate(calls, 1):
            tail += frame({"type": "content_block_start", "index": index,
                           "content_block": {"type": "tool_use", "id": item["id"],
                                             "name": item["name"], "input": {}}})
            encoded = json.dumps(item["arguments"])
            for part in (encoded[:5], encoded[5:]):
                tail += frame({"type": "content_block_delta", "index": index,
                               "delta": {"type": "input_json_delta", "partial_json": part}})
            tail += frame({"type": "content_block_stop", "index": index})
        tail += frame({"type": "message_delta", "delta": {
            "stop_reason": "tool_use" if calls else "end_turn",
        }})
        tail += frame({"type": "message_stop"})
        return head, tail
    head = frame({"choices": [{"index": 0, "delta": {
        "reasoning_content": "Reviewing files.", "content": text,
    }}]})
    tail = b""
    for index, item in enumerate(calls):
        encoded = json.dumps(item["arguments"])
        tail += frame({"choices": [{"index": 0, "delta": {"tool_calls": [{
            "index": index, "id": item["id"], "type": "function",
            "function": {"name": item["name"], "arguments": encoded[:5]},
        }]}}]})
        tail += frame({"choices": [{"index": 0, "delta": {"tool_calls": [{
            "index": index, "function": {"arguments": encoded[5:]},
        }]}}]})
    tail += frame({"choices": [{"index": 0, "delta": {},
                                "finish_reason": "tool_calls" if calls else "stop"}]})
    return head, tail + b"data: [DONE]\r\n\r\n"


@pytest.mark.parametrize("provider", ["codex", "claude", "configured"])
@pytest.mark.parametrize("outcome", ["complete", "cancel", "interrupted"])
def test_tui_shows_model_and_command_output_before_completion(
    tmp_path, provider, outcome, tui_verifier
):
    import fcntl
    import pty
    import struct
    import termios

    binary = find_meden()
    assert binary is not None
    release_first = threading.Event()
    release_model = threading.Event()
    release_text = threading.Event()
    release_final = threading.Event()
    requests = []
    errors = []
    command = (
        "touch started; printf 'LIVE-STDOUT\\n'; printf 'LIVE-STDERR\\n' >&2; "
        "while [ ! -f release ]; do sleep .05; done; printf 'COMMAND-END\\n'"
    )

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_POST(self):
            try:
                request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                requests.append(request)
                assert request["stream"] is True
                first = len(requests) == 1
                if first:
                    assert release_first.wait(15)
                head, tail = response_parts(
                    provider, "I" if first else "STREAM-FINAL",
                    [call("live-command", "bash", {"command": command})] if first else (),
                )
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream; charset=utf-8")
                self.end_headers()
                self.wfile.write(head)
                self.wfile.flush()
                if first:
                    assert release_text.wait(15)
                    rest = " will check the workspace."
                    if provider == "codex":
                        delta = {"type": "response.output_text.delta", "delta": rest}
                    elif provider == "claude":
                        delta = {"type": "content_block_delta", "index": 0,
                                 "delta": {"type": "text_delta", "text": rest}}
                    else:
                        delta = {"choices": [{"index": 0, "delta": {"content": rest}}]}
                    self.wfile.write(frame(delta))
                    self.wfile.flush()
                assert (release_model if first else release_final).wait(15)
                self.wfile.write(tail.rstrip() if outcome == "interrupted" else tail)
                self.wfile.flush()
            except (BrokenPipeError, ConnectionResetError):
                pass
            except Exception as error:
                errors.append(error)

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    base = f"http://127.0.0.1:{server.server_port}/v1"
    env = environment(tmp_path, "codex" if provider == "configured" else provider, base)
    if provider == "configured":
        env.update(MEDEN_LLM_BASE_URL=base, MEDEN_LLM_MODEL="fixture")
    env["TERM"] = "xterm-256color"
    project = tmp_path / "project"
    project.mkdir()
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 32, 110, 0, 0))
    initial = termios.tcgetattr(slave)
    process = subprocess.Popen(
        [binary, "tui", "--workspace", str(project), "--provider", provider, "--model", "fixture"],
        stdin=slave, stdout=slave, stderr=slave, env=env,
    )
    screen = pyte.Screen(110, 32)
    terminal = pyte.Stream(screen)
    decoder = codecs.getincrementaldecoder("utf-8")("replace")

    def view():
        return "\n".join("".join(screen.buffer[y][x].data for x in range(screen.columns))
                         for y in range(screen.lines))

    def wait_for(predicate):
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            if predicate():
                return
            assert process.poll() is None, view()
            assert not errors, errors
            if select.select([master], [], [], .05)[0]:
                terminal.feed(decoder.decode(os.read(master, 65536)))
        pytest.fail(view())

    try:
        wait_for(lambda: "Connection details" in view())
        os.write(master, b"y")
        wait_for(lambda: "shift+enter" in view())
        os.write(master, b"hello\r")
        wait_for(lambda: "elapsed" in view() and len(requests) == 1)
        spin = next(character for character in view() if character in "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
        wait_for(lambda: "elapsed" in view() and spin not in view())
        assert "shift+enter" not in view(), "silent model looked idle"
        release_first.set()
        wait_for(lambda: "● I" in view())
        assert "I will check the workspace." not in view()
        assert "shift+enter" not in view() and len(requests) == 1
        assert not (project / "started").exists()
        release_text.set()
        wait_for(lambda: "I will check the workspace." in view())
        assert "shift+enter" not in view(), "partial response prematurely completed the turn"
        assert "› hello" in view() and "YOU" not in view()
        if provider != "claude":
            wait_for(lambda: "∴ Reviewing files." in view())
        release_model.set()
        workspace = project.resolve()
        if outcome == "interrupted":
            wait_for(lambda: "Stopped: Model response could not be completed" in view()
                     and "shift+enter" in view())
            assert "unfinished tool calls were not executed" in view()
            assert "/retry" in view()
            assert len(requests) == 1
            assert not (workspace / "started").exists(), "partial stream executed its tool call"
            wait_for(lambda: "Response interrupted before completion" in view())
            assert not errors
            os.write(master, b"\x03")
            assert process.wait(timeout=5) == 0
            assert termios.tcgetattr(slave) == initial
            return
        wait_for(lambda: "LIVE-STDOUT" in [line.strip() for line in view().splitlines()]
                 and "LIVE-STDERR" in [line.strip() for line in view().splitlines()])
        assert len(requests) == 1, "model continued before its command finished"
        assert "$ touch started" in view()
        assert "exit 0" not in view()
        assert not (project / "release").exists()
        if outcome == "cancel":
            os.write(master, b"\x1b")
            wait_for(lambda: "shift+enter" in view())
            assert "Cancelled" in view()
            assert len(requests) == 1
            assert not (workspace / "release").exists()
        else:
            (workspace / "release").touch()
            wait_for(lambda: "STREAM-FINAL" in view())
            assert "COMMAND-END" in [line.strip() for line in view().splitlines()]
            assert "exit 0" in view()
            assert "shift+enter" not in view()
            release_final.set()
            wait_for(lambda: "shift+enter" in view())
            assert view().count("STREAM-FINAL") == 1, "completion duplicated streamed text"
            assert len(requests) == 2
        assert not errors
        os.write(master, b"\x03")
        assert process.wait(timeout=5) == 0
        assert termios.tcgetattr(slave) == initial
    finally:
        release_first.set()
        release_text.set()
        release_model.set()
        release_final.set()
        if process.poll() is None:
            process.kill()
            process.wait(timeout=5)
        os.close(master)
        os.close(slave)
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
