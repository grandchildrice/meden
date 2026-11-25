"""Native finite source registration through the public TUI without model approval.

Actual confined Z3, complete saved observations and a visible existing-file
recovery are exercised; this is not live inference or a cold tool installation.
"""

import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import pytest
from test_tui_setup import Terminal, environment

from meden_checks import find_meden

EXAMPLES = Path(__file__).resolve().parents[2] / "verification/examples"


@pytest.mark.ux
def test_native_finite_source_graph_recovers_and_reopens_without_lean_or_model_approval(tmp_path):
    root = tmp_path / "project"
    root.mkdir()
    contract = (EXAMPLES / "finite-counter.contract.json").read_text()
    model = (EXAMPLES / "finite-counter-broken.model.json").read_text()
    (root / "counter.contract.json").write_text(contract)
    (root / "counter.model.json").write_text(model)
    missing = json.loads(contract)
    missing["source_path"] = "absent.model.json"
    (root / "missing.contract.json").write_text(json.dumps(missing))
    requests = []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_POST(self):
            requests.append(self.rfile.read(int(self.headers["Content-Length"])))
            self.send_response(503)
            self.send_header("Content-Length", "0")
            self.end_headers()

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    env = environment(tmp_path)
    env["MEDEN_ATLAS_LEAN"] = str(tmp_path / "unavailable-lean")
    args = [
        find_meden(), "tui", "--workspace", str(root), "--verifier", "z3.finite.v1",
        "--base-url", f"http://127.0.0.1:{server.server_port}/v1",
        "--model", "unused-finite-source",
    ]
    terminal = None

    def open_unapproved():
        opened = Terminal(args, env, root)
        opened.wait(lambda view: "Connection details" in view)
        opened.send("n")
        opened.wait(lambda view: "Choose a model connection" in view)
        opened.send("\x1b")
        opened.wait(lambda view: "shift+enter" in view)
        return opened

    def command(value, expected):
        terminal.paste(value)
        terminal.send("\r")
        terminal.wait(lambda view: expected in view and "shift+enter" in view)
        assert requests == []

    try:
        terminal = open_unapproved()
        command("/graph missing.contract.json", "Source observation failed for absent.model.json")
        command("/graph counter.contract.json", "property.monotone · counter.contract.json:")
        assert "Source graph ready: 2 files" in terminal.view()
        assert "transition.value · counter.model.json:" in terminal.view()
        assert "256 assignments" in " ".join(terminal.view().split())
        terminal.quit()
        terminal.close()
        terminal = open_unapproved()
        command("/graph counter.contract.json", "property.step · counter.contract.json:")
        assert "Source graph ready: 2 files" in terminal.view()
        assert (root / "counter.contract.json").read_text() == contract
        assert (root / "counter.model.json").read_text() == model
        command("/graph absent.contract.json", "Source observation failed for absent.contract.json")
        terminal.quit()
    finally:
        if terminal:
            terminal.close()
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
