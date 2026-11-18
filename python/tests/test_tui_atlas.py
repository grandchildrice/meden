"""Installed-Lean TUI journey; separate from the fast, source-only UX gate."""

import json
import os
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import pytest
from test_tui_setup import Terminal, environment

from meden_checks import find_meden


def test_public_tui_compiles_atlas_and_analyzes_an_exact_declaration(tmp_path, tui_verifier):
    lean = Path(
        os.environ.get(
            "MEDEN_ATLAS_LEAN", str(Path.home() / ".elan/toolchains/leanprover--lean4---v4.35.0")
        )
    )
    if not (lean / "bin/lean").is_file():
        pytest.skip("actual installed Lean is required for TUI Atlas coverage")
    root = tmp_path / "project"
    root.mkdir()
    (root / "Domain.lean").write_text("def eligible (n : Nat) := n ≥ 150\n")
    (root / "App.lean").write_text(
        "import Domain\n-- COMMENT_NOT_TRANSLATOR_CONTEXT\n"
        "theorem permitted (n : Nat) (h : eligible n) : eligible n := h\n"
    )
    requests = []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            requests.append(body)
            tool = body["tools"][0]["function"]["name"]
            data = (
                {
                    "lemmaName": "permitted",
                    "naturalLanguage": "Eligible input stays eligible.",
                    "preconditions": "eligible n",
                    "postcondition": "eligible n",
                    "scope": "Given assumption",
                    "strength": "trivial",
                    "confidence": 0.9,
                }
                if tool == "record_informalization"
                else {
                    "lemmaName": "permitted",
                    "match": False,
                    "discrepancy": "The assumption is restated; eligibility is not established.",
                    "weakeningType": "tautology",
                    "explanation": "The desired bound is assumed.",
                }
            )
            response = json.dumps(
                {
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": None,
                                "tool_calls": [
                                    {
                                        "id": "atlas-candidate",
                                        "type": "function",
                                        "function": {"name": tool, "arguments": json.dumps(data)},
                                    }
                                ],
                            }
                        }
                    ]
                }
            ).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(response)))
            self.end_headers()
            self.wfile.write(response)

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    env = environment(tmp_path)
    env["MEDEN_ATLAS_LEAN"] = str(lean)
    args = [
        find_meden(),
        "tui",
        "--workspace",
        str(root),
        "--base-url",
        f"http://127.0.0.1:{server.server_port}/v1",
        "--model",
        "atlas-fixture",
        "--api-key",
        "SYNTHETIC-ATLAS-KEY",
    ]
    terminal = Terminal(args, env, root)
    screens = {}

    def command(text, result):
        terminal.paste(text)
        terminal.send("\r")
        deadline = time.monotonic() + 180
        while time.monotonic() < deadline:
            terminal.pump()
            if result in terminal.view() and "shift+enter" in terminal.view():
                screens[text] = terminal.view()
                return
            assert terminal.process.poll() is None, terminal.view()
        pytest.fail(terminal.view())

    try:
        terminal.wait(lambda view: "Connection details" in view)
        terminal.send("y")
        terminal.wait(lambda view: "shift+enter" in view)
        command("/atlas-prepare", "Atlas is ready")
        command("/atlas App.lean Domain.lean", "permitted · App.lean:3")
        assert not requests
        command(
            "/analyze permitted -- REQUIREMENT_ONLY: Establish eligibility for every input.",
            "Candidate Mismatch",
        )
        assert len(requests) == 2
        assert "elaborated" in json.dumps(requests[0])
        assert "eligible" in json.dumps(requests[0])
        assert "COMMENT_NOT_TRANSLATOR_CONTEXT" not in json.dumps(requests[0])
        assert "REQUIREMENT_ONLY" not in json.dumps(requests[0])
        assert "REQUIREMENT_ONLY" in json.dumps(requests[1])
        assert "elaborated" not in json.dumps(requests[1])
        terminal.send("/quit\r")
        assert terminal.process.wait(timeout=5) == 0
        (tmp_path / "tui-atlas-observation.json").write_text(
            json.dumps(
                {
                    "scope": "Installed-Lean Atlas and public TUI; scripted inference, no proof",
                    "screens": screens,
                    "requests": requests,
                },
                indent=2,
            )
        )
    finally:
        terminal.close()
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
