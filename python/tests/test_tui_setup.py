"""First-run connection settings through the real TUI, without model flags."""

import codecs
import fcntl
import json
import os
import pty
import select
import shlex
import stat
import struct
import subprocess
import termios
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pyte
import pytest

from meden_checks import find_meden


class Terminal:
    def __init__(self, args, env, cwd):
        self.master, self.slave = pty.openpty()
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", 36, 130, 0, 0))
        self.initial = termios.tcgetattr(self.slave)
        self.screen = pyte.Screen(130, 36)
        self.stream = pyte.Stream(self.screen)
        self.decoder = codecs.getincrementaldecoder("utf-8")("replace")
        self.raw = bytearray()
        self.process = subprocess.Popen(
            args, stdin=self.slave, stdout=self.slave, stderr=self.slave, env=env, cwd=cwd,
        )

    def view(self):
        return "\n".join(self.screen.display)

    def pump(self):
        if select.select([self.master], [], [], .05)[0]:
            data = os.read(self.master, 65536)
            self.raw.extend(data)
            self.stream.feed(self.decoder.decode(data))

    def wait(self, predicate):
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            if predicate(self.view()):
                return
            assert self.process.poll() is None, self.raw[-6000:].decode(errors="replace")
            self.pump()
        pytest.fail(self.view())

    def send(self, text):
        os.write(self.master, text.encode() if isinstance(text, str) else text)

    def paste(self, text):
        self.send(b"\x1b[200~" + text.encode() + b"\x1b[201~")

    def quit(self):
        self.send(b"\x03")
        deadline = time.monotonic() + 5
        while self.process.poll() is None and time.monotonic() < deadline:
            self.pump()
        assert self.process.wait(timeout=5) == 0
        assert termios.tcgetattr(self.slave) == self.initial

    def close(self):
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait(timeout=5)
        os.close(self.master)
        os.close(self.slave)


def environment(tmp_path):
    env = {k: v for k, v in os.environ.items()
           if not k.startswith(("MEDEN_LOCAL_", "MEDEN_LLM_", "MEDEN_CODEX_", "MEDEN_CLAUDE_"))
           and k not in {"OLLAMA_API_KEY", "OPENAI_API_KEY", "CLAUDE_CODE_OAUTH_TOKEN",
                         "ANTHROPIC_TOKEN", "NO_COLOR"}}
    home = tmp_path / "home"
    home.mkdir()
    env.update(HOME=str(home), HERMES_HOME=str(home / "hermes"),
               CODEX_HOME=str(home / "codex"), CLAUDE_CONFIG_DIR=str(home / "claude"),
               XDG_DATA_HOME=str(tmp_path / "data"), TERM="xterm-256color")
    original_home = os.environ.get("HOME", "")
    env.setdefault("RUSTUP_HOME", f"{original_home}/.rustup")
    env.setdefault("CARGO_HOME", f"{original_home}/.cargo")
    return env


def open_http_form(terminal):
    terminal.wait(lambda text: "Choose a model connection" in text or "Connection details" in text)
    if "Connection details" in terminal.view():
        terminal.send("n")
        terminal.wait(lambda text: "Choose a model connection" in text)
    terminal.send("n")
    terminal.wait(lambda text: "API base URL" in text and "API key (optional)" in text)


@pytest.mark.parametrize("remember", [False, True])
def test_first_run_settings_are_masked_remembered_and_always_reviewed(
    tmp_path, tui_verifier, remember
):
    requests = []
    paths = []
    errors = []
    project = tmp_path / "project"
    project.mkdir()

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_POST(self):
            try:
                request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                requests.append((self.headers.get("Authorization"), request))
                paths.append(self.path)
                index = len(requests)
                if index == 1:
                    settings = next((tmp_path / "data/meden/projects").glob("*/connection.json"))
                    command = (f"test ! -e {shlex.quote(str(settings))} && "
                               "test -z \"$MEDEN_LLM_API_KEY\" && printf complete > marker.txt")
                    message = {"tool_calls": [{"id": "marker", "type": "function", "function": {
                        "name": "bash", "arguments": json.dumps({"command": command}),
                    }}]}
                else:
                    message = {"content": {2: "SETUP-COMPLETE", 3: "SETUP-RESUMED"}.get(
                        index, "SETUP-REVISED",
                    )}
                data = json.dumps({"choices": [
                    {"message": {"role": "assistant", **message}},
                ]}).encode()
                self.send_response(200)
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)
            except Exception as error:
                errors.append(error)

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    worker = threading.Thread(target=server.serve_forever, daemon=True)
    worker.start()
    base = f"http://127.0.0.1:{server.server_port}/v1"
    env = environment(tmp_path)
    binary = str(find_meden())
    # Cover the no-subcommand entry point as well as `meden tui --workspace`.
    args = [binary] if remember else [binary, "tui", "--workspace", str(project)]
    terminal = Terminal(args, env, project)
    key = "PRIVATE-TUI-KEY-CANARY"
    try:
        open_http_form(terminal)
        terminal.send(b"\x15")
        terminal.paste("not-a-url")
        terminal.send("\rfixture\r")
        terminal.paste(key)
        terminal.wait(lambda text: "••••" in text)
        assert key.encode() not in terminal.raw and requests == []
        terminal.send("\r")
        terminal.wait(lambda text: "processor requires HTTPS or loopback HTTP" in text)
        assert "API base URL" in terminal.view() and requests == []
        # Fixing the URL clears the old credential, even while editing a form.
        terminal.send(b"\x1b[Z\x1b[Z\x15")
        terminal.paste(base)
        terminal.send("\r\r")
        terminal.paste(key)
        if remember:
            terminal.send(b"\t ")
            terminal.wait(lambda text: "[x] Remember API key" in text)
        terminal.send("\r")
        terminal.wait(lambda text: "Connection details" in text and base in text)
        terminal.paste("y")
        terminal.send("\r")
        # Wait for another redraw before asserting pasted text did not approve.
        terminal.pump()
        assert requests == [] and "Connection details" in terminal.view()
        terminal.send("y")
        terminal.wait(lambda text: "shift+enter" in text)
        terminal.send("Write a marker and inspect this workspace.\r")
        terminal.wait(lambda text: "SETUP-COMPLETE" in text and "shift+enter" in text)
        assert (project / "marker.txt").read_text() == "complete"
        settings = next((tmp_path / "data/meden/projects").glob("*/connection.json"))
        saved = json.loads(settings.read_text())["profile"]
        assert saved["base_url"] == base and saved["model"] == "fixture"
        assert saved["api_key"] == (key if remember else None)
        assert stat.S_IMODE(settings.stat().st_mode) == 0o600
        assert stat.S_IMODE(settings.parent.stat().st_mode) == 0o700
        assert key not in (settings.parent / "conversation.json").read_text()
        assert key.encode() not in terminal.raw
        terminal.quit()
        terminal.close()

        terminal = Terminal(args, env, project)
        if not remember:
            terminal.wait(lambda text: "API key (optional)" in text and base in text)
            assert requests and len(requests) == 2
            terminal.paste(key)
            terminal.send("\r")
        terminal.wait(lambda text: "Connection details" in text and base in text)
        assert len(requests) == 2, "saved preferences restored inference authority"
        terminal.send("y")
        terminal.wait(lambda text: "shift+enter" in text)
        terminal.send("Resume the same project.\r")
        terminal.wait(lambda text: "SETUP-RESUMED" in text and "shift+enter" in text)
        assert "Write a marker" in json.dumps(requests[-1][1])
        assert all(auth == f"Bearer {key}" for auth, _ in requests)
        assert key.encode() not in terminal.raw and not errors
        terminal.send("/connect\r")
        terminal.wait(lambda text: "API base URL" in text)
        terminal.send(b"\x15")
        terminal.paste(base + "/other")
        terminal.send("\r\r")
        terminal.wait(lambda text: "[ ] Remember API key" in text and "••" not in text)
        terminal.send("\r")
        terminal.wait(lambda text: "Connection details" in text and "/other" in text)
        assert len(requests) == 3
        terminal.send("y")
        terminal.wait(lambda text: "shift+enter" in text)
        terminal.send("Use the revised connection.\r")
        terminal.wait(lambda text: "SETUP-REVISED" in text and "shift+enter" in text)
        assert len(requests) == 4 and requests[-1][0] is None
        assert paths[-1] == "/v1/other/chat/completions"
        terminal.quit()
        terminal.close()
        invalid = json.loads(settings.read_text())
        invalid["approved"] = True
        settings.write_text(json.dumps(invalid))
        terminal = Terminal(args, env, project)
        terminal.wait(lambda text: "Could not restore the saved connection" in text
                      and "Choose a model connection" in text)
        assert len(requests) == 4 and "Connection details" not in terminal.view()
        terminal.quit()
    finally:
        terminal.close()
        server.shutdown()
        server.server_close()
        worker.join(timeout=5)


LOCAL_ENGINE = r"""
import json, socketserver, sys
from http.server import BaseHTTPRequestHandler
class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args): pass
    def do_GET(self):
        self.send_response(200); self.send_header('Content-Length','2'); self.end_headers()
        self.wfile.write(b'{}')
    def do_POST(self):
        body=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        resumed='Continue using' in json.dumps(body['messages'][-1])
        text='LOCAL-SETUP-RESUMED' if resumed else 'LOCAL-SETUP-COMPLETE'
        data=json.dumps({'choices':[{'message':{'role':'assistant',
            'content':text}}]}).encode()
        self.send_response(200); self.send_header('Content-Length',str(len(data)))
        self.end_headers(); self.wfile.write(data)
socketserver.UnixStreamServer(sys.argv[sys.argv.index('--host')+1], Handler).serve_forever()
"""


def test_local_assets_can_be_selected_and_reopened_without_flags(tmp_path, tui_verifier):
    engine = tmp_path / "engine"
    engine.write_text("#!/bin/python3\n" + LOCAL_ENGINE)
    engine.chmod(0o700)
    weights = tmp_path / "fixture.gguf"
    weights.write_bytes(b"scripted engine fixture, not model weights")
    project = tmp_path / "project"
    project.mkdir()
    env = environment(tmp_path)
    args = [str(find_meden()), "tui", "--workspace", str(project)]
    terminal = Terminal(args, env, project)
    try:
        open_http_form(terminal)
        terminal.send(b"\x1b")
        terminal.wait(lambda text: "Choose a model connection" in text)
        terminal.send("l")
        terminal.wait(lambda text: "GGUF model file" in text)
        terminal.paste("../engine")
        terminal.send("\r")
        terminal.paste("../fixture.gguf")
        terminal.send("\r")
        terminal.wait(lambda text: "shift+enter" in text)
        assert "Connection details" not in terminal.view()
        terminal.send("Hello from the configured local engine.\r")
        terminal.wait(lambda text: "LOCAL-SETUP-COMPLETE" in text and "shift+enter" in text)
        saved = next((tmp_path / "data/meden/projects").glob("*/connection.json"))
        assert json.loads(saved.read_text())["profile"]["kind"] == "local"
        profile = json.loads(saved.read_text())["profile"]
        assert profile["model"].endswith("/project/../fixture.gguf")
        terminal.quit()
        terminal.close()
        terminal = Terminal(args, env, project)
        terminal.wait(lambda text: "shift+enter" in text and "LOCAL-SETUP-COMPLETE" in text)
        terminal.send("Continue using the same local configuration.\r")
        terminal.wait(lambda text: "LOCAL-SETUP-RESUMED" in text and "shift+enter" in text)
        terminal.quit()
    finally:
        terminal.close()
