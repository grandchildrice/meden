"""Keep the live-model driver honest against the actual current TUI."""

import json
import shlex
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest

from meden_checks import find_meden
from meden_checks.tui_smoke import main

pytestmark = pytest.mark.ux


@pytest.mark.parametrize("broken", [False, True])
def test_live_smoke_driver_checks_repository_restart_and_independent_execution(
    tmp_path, monkeypatch, tui_verifier, broken
):
    requests = []
    errors = []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_POST(self):
            try:
                body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                requests.append(body)
                index = len(requests)
                if index in (1, 3):
                    if index == 3:
                        history = json.dumps(body["messages"])
                        assert "Fix the implementation" in history
                        assert "FIRST-TURN-DONE" in history
                        assert "In the same workspace" in history
                    source = "def add(a, b):\n    return a + b\n"
                    command = f"printf %s {shlex.quote(source)} > calc.py; "
                    if index == 3:
                        source += "\ndef subtract(a, b):\n    return a - b\n"
                        tests = "import unittest\nfrom calc import subtract\n"
                        tests += ("class T(unittest.TestCase):\n"
                                  "    def test_subtract(self):\n"
                                  "        self.assertEqual(subtract(7, 2), 5)\n")
                        command = (f"printf %s {shlex.quote(source)} > calc.py; "
                                   f"printf %s {shlex.quote(tests)} > test_subtract.py; ")
                    command += "python3 -m unittest -v test_calc.py"
                    if index == 3:
                        command += " test_subtract.py"
                    if broken and index == 3:
                        # A lying completion/marker cannot replace independent tests.
                        source = "def add(a, b):\n    return a + b\n"
                        source += "def subtract(a, b):\n    return 0\n"
                        command += f"; printf %s {shlex.quote(source)} > calc.py"
                    message = {"tool_calls": [{"id": f"call-{index}", "type": "function",
                               "function": {"name": "bash", "arguments": json.dumps({
                                   "command": command,
                               })}}]}
                else:
                    message = {"content": "FIRST-TURN-DONE" if index == 2 else "ALL-DONE"}
                data = json.dumps({"choices": [{"message": {"role": "assistant",
                                                            **message}}]}).encode()
                self.send_response(200)
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)
            except Exception as error:
                errors.append(error)

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    worker = threading.Thread(target=server.serve_forever, daemon=True)
    worker.start()
    monkeypatch.setenv("MEDEN_LLM_BASE_URL", f"http://127.0.0.1:{server.server_port}/v1")
    monkeypatch.setenv("MEDEN_LLM_MODEL", "scripted-smoke-fixture")
    report = tmp_path / "smoke.json"
    try:
        result = main(["--binary", str(find_meden()), "--provider", "configured",
                       "--timeout", "30", "--out", str(report)])
        data = json.loads(report.read_text())
        assert not errors
        assert result == (1 if broken else 0), data
        assert data["success"] is (not broken)
        assert len(requests) == 4
        assert data["reopened_with_fresh_review"]
        assert data["edited_selected_repository"]
        assert data["independent_workspace"] != data["workspace"]
        assert data["terminal_restored"] and data["terminal_restored_before_restart"]
        assert data["independent_test_exit_code"] == (1 if broken else 0)
        transcript = report.with_suffix(".terminal.txt").read_text()
        assert "fresh review after restart" in transcript and "follow-up complete" in transcript
    finally:
        server.shutdown()
        server.server_close()
        worker.join(timeout=5)
