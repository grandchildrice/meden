"""Registered finite backend through real public TUI/core/jobs/offline Z3.

Inference is scripted; checker execution and counterexample feedback are real.
No Lean bundle or private configuration repair is used to complete the journey.
"""
import hashlib
import json
from pathlib import Path
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest

from meden_checks import find_meden
from test_tui_setup import Terminal, environment

EXAMPLES = Path(__file__).resolve().parents[2] / "verification/examples"


@pytest.mark.ux
def test_finite_contract_real_refutation_repair_staleness_and_restart(tmp_path):
    contract = json.loads((EXAMPLES / "finite-counter.contract.json").read_text())
    correct = (EXAMPLES / "finite-counter.model.json").read_text()
    broken = (EXAMPLES / "finite-counter-broken.model.json").read_text()
    calls = [
        ("propose_contract", {"contract": contract}),
        ("write_file", {"path": contract["source_path"], "content": broken}),
        ("verify_project", {}),
        ("write_file", {"path": contract["source_path"], "content": correct}),
        ("verify_project", {}),
    ]
    requests = []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            index = len(requests)
            requests.append(body)
            if index < len(calls):
                name, args = calls[index]
                reply = {"content": None, "tool_calls": [{
                    "id": f"finite-{index}", "type": "function",
                    "function": {"name": name, "arguments": json.dumps(args)},
                }]}
            else:
                reply = {"content": "FINITE-COMPLETE"}
            payload = json.dumps({"choices": [{"message": {"role": "assistant", **reply}}]}).encode()
            self.send_response(200)
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    root = tmp_path / "project"
    root.mkdir()
    env = environment(tmp_path)
    env.pop("MEDEN_VERIFIER_BUNDLE", None)
    args = [find_meden(), "tui", "--workspace", str(root), "--verifier", "z3.finite.v1",
            "--base-url", f"http://127.0.0.1:{server.server_port}/v1", "--model", "finite-fixture"]
    terminal = Terminal(args, env, root)
    try:
        terminal.wait(lambda s: "Connection details" in s)
        assert not requests
        terminal.send("y")
        terminal.wait(lambda s: "shift+enter" in s)
        terminal.paste("Check the complete four-bit counter transition, retaining state on overflow.")
        terminal.send("\r")
        terminal.wait(lambda s: "y: Accept specification" in s)
        assert "256 assignments" in terminal.view()
        assert "one-step" in terminal.view()
        assert not (root / contract["source_path"]).exists()
        (tmp_path / "review-screen.txt").write_text(terminal.view())
        terminal.send("y")
        terminal.wait(lambda s: "FINITE-COMPLETE" in s and "shift+enter" in s)
        screen = terminal.view()
        assert "CHECKED · complete within finite bounds" in screen
        assert "✓ proved" not in screen and "Lean refinement" not in screen
        assert (root / contract["source_path"]).read_text() == correct
        # The repair must have received a real solver/evaluator counterexample.
        assert any("Finite property failed" in json.dumps(r) for r in requests)
        artifacts = list((Path(env["XDG_DATA_HOME"]) / "meden/projects").glob("*/artifacts/checked-*"))
        assert len(artifacts) == 1
        artifact = artifacts[0]
        report = json.loads((artifact / "report.json").read_text())
        assert report["class"] == "bounded" and report["backend"] == "z3.finite.v1"
        assert report["contract"] == contract and report["source_sha256"] == hashlib.sha256(correct.encode()).hexdigest()
        assert (artifact / "solver.txt").read_text().strip() == "unsat"
        assert "256" in " ".join(report["audit"])
        (tmp_path / "checked-screen.txt").write_text(screen)
        # External host edits model a real editor. The public evidence command
        # must invalidate coverage, even if bytes are subsequently restored.
        (root / contract["source_path"]).write_text(broken)
        terminal.send(b"\x07")
        terminal.wait(lambda s: "recheck required" in s and "Properties of the implementation" in s)
        assert "✓ proved" not in terminal.view()
        (tmp_path / "stale-screen.txt").write_text(terminal.view())
        terminal.send(b"\x1b")
        terminal.wait(lambda s: "shift+enter" in s)
        terminal.quit()
        terminal.close()
        before = len(requests)
        terminal = Terminal(args, env, root)
        terminal.wait(lambda s: "Connection details" in s)
        assert len(requests) == before
        assert "complete within finite bounds" not in terminal.view()
        terminal.send("n")
        terminal.wait(lambda s: "Choose a model connection" in s)
        (tmp_path / "restart-screen.txt").write_text(terminal.view())
        journal = json.loads(next((Path(env["XDG_DATA_HOME"]) / "meden/projects").glob("*/contracts.json")).read_text())
        assert journal["version"] == 2
        assert journal["events"][0]["required"]["class"] == "bounded"
    finally:
        (tmp_path / "requests.json").write_text(json.dumps(requests, indent=2))
        terminal.close()
        server.shutdown()
        server.server_close()
