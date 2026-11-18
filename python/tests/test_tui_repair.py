"""Reviewed repair effects through the public TUI; inference is explicitly scripted."""

import fcntl
import hashlib
import json
import re
import signal
import struct
import termios
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest
from test_tui_setup import Terminal, environment

from meden_checks import find_meden

pytestmark = pytest.mark.ux


def test_finding_repair_reject_stale_clipped_review_apply_and_restart(tmp_path, tui_verifier):
    root = tmp_path / "project"
    root.mkdir()
    original = "pub fn stock(n: u64) -> u64 { n }\n"
    source = root / "stock.rs"
    source.write_text(original)
    requests = []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            requests.append(body)
            tool = body["tools"][0]["function"]["name"]
            data = json.loads(body["messages"][-1]["content"])
            if tool == "propose_repair":
                result = {
                    "file": "stock.rs",
                    "old_text": "{ n }",
                    "new_text": "{ n.saturating_sub(1) }",
                    "explanation": "Decrement positive stock and retain zero.",
                }
            elif tool == "record_informalization":
                fixed = "saturating_sub" in data["files"]["stock.rs"]
                result = {
                    "lemmaName": "stock",
                    "naturalLanguage": "Positive stock decreases; zero remains zero."
                    if fixed
                    else "The stock remains unchanged.",
                    "preconditions": "Any u64 stock",
                    "postcondition": "Saturating decrement" if fixed else "Output equals input",
                    "scope": "One transition",
                    "strength": "moderate",
                    "confidence": 0.9,
                }
            else:
                fixed = "decreases" in data["backTranslation"]["naturalLanguage"]
                result = {
                    "lemmaName": "stock",
                    "match": fixed,
                    "discrepancy": "" if fixed else "No decrement is established.",
                    "weakeningType": "none" if fixed else "wrong-property",
                    "explanation": "This matches the saturating decrement."
                    if fixed
                    else "Unchanged stock does not satisfy the requested decrement.",
                }
            response = json.dumps(
                {
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": None,
                                "tool_calls": [
                                    {
                                        "id": "scripted-data",
                                        "type": "function",
                                        "function": {"name": tool, "arguments": json.dumps(result)},
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
    args = [
        find_meden(),
        "tui",
        "--workspace",
        str(root),
        "--base-url",
        f"http://127.0.0.1:{server.server_port}/v1",
        "--model",
        "repair-fixture",
        "--api-key",
        "SYNTHETIC-REPAIR-KEY",
    ]
    terminal = Terminal(args, env, root)
    requirement = "Decrement positive stock and keep zero at zero."

    def command(text, result):
        previous = re.findall(r"Revision: (sha256:[a-f0-9]{64})", terminal.view())
        terminal.paste(text)
        terminal.send("\r")

        def completed(view):
            revisions = re.findall(r"Revision: (sha256:[a-f0-9]{64})", view)
            fresh = not text.startswith("/sources") or (
                revisions and (not previous or revisions[-1] != previous[-1])
            )
            return result in view and "shift+enter" in view and fresh

        terminal.wait(completed)

    def repair(count):
        terminal.paste("/repair")
        terminal.send("\r")
        terminal.wait(
            lambda view: (
                "Apply this repair and reanalyze?" in view
                and "Apply once and reanalyze" in view
                and "Accepted contract: none" in view
                and len(requests) == count
            )
        )

    try:
        terminal.wait(lambda view: "Connection details" in view)
        terminal.send("y")
        terminal.wait(lambda view: "shift+enter" in view)
        command("/sources stock.rs", "Source graph ready")
        command(f"/analyze stock.rs:1 stock -- {requirement}", "Candidate Mismatch")
        repair(3)
        assert source.read_text() == original, "preparing a diff must not edit the repository"
        terminal.send("n")
        terminal.wait(lambda view: "Repair rejected" in view and "shift+enter" in view)
        assert source.read_text() == original
        assert len(requests) == 3
        repair(4)
        # This deliberate concurrent host mutation tests refusal, never rescues the journey.
        concurrent = original + "// concurrent host edit\n"
        source.write_text(concurrent)
        terminal.send("y")
        terminal.wait(
            lambda view: "Sources changed since the Finding" in view and "shift+enter" in view
        )
        assert source.read_text() == concurrent
        assert len(requests) == 4, "stale review cannot apply or trigger new inference"
        command("/sources stock.rs", "Source graph ready")
        terminal.paste(f"/analyze stock.rs:1 stock -- {requirement}")
        terminal.send("\r")
        terminal.wait(
            lambda view: (
                "Candidate Mismatch" in view and "shift+enter" in view and len(requests) == 6
            )
        )
        repair(7)
        fcntl.ioctl(terminal.slave, termios.TIOCSWINSZ, struct.pack("HHHH", 12, 40, 0, 0))
        terminal.screen.resize(12, 40)
        terminal.process.send_signal(signal.SIGWINCH)
        terminal.wait(lambda view: "Approval unavailable" in view)
        terminal.send("y")
        for _ in range(3):
            terminal.pump()
        assert source.read_text() == concurrent
        assert len(requests) == 7
        fcntl.ioctl(terminal.slave, termios.TIOCSWINSZ, struct.pack("HHHH", 36, 130, 0, 0))
        terminal.screen.resize(36, 130)
        terminal.process.send_signal(signal.SIGWINCH)
        terminal.wait(lambda view: "Apply once and reanalyze" in view and "File: stock.rs" in view)
        terminal.send(b"\x1b[6~")
        terminal.wait(lambda view: "saturating_sub" in view and "File: stock.rs" in view)
        terminal.send("y")
        terminal.wait(
            lambda view: (
                "Repair applied and reanalyzed" in view
                and "Candidate Match" in view
                and "shift+enter" in view
            )
        )
        assert source.read_text() == concurrent.replace("{ n }", "{ n.saturating_sub(1) }")
        assert len(requests) == 9
        history_path = next((tmp_path / "data/meden/projects").glob("*/meaning-analysis.json"))
        entries = json.loads(history_path.read_text())["entries"]
        assert len(entries) == 3
        assert len({entry["requirement_id"] for entry in entries}) == 1
        assert entries[-1]["input"]["source_revision"] != entries[-2]["input"]["source_revision"]
        assert entries[-1]["finding_id"] is None
        retained = []
        for entry in entries:
            digest = entry["source_artifact"].removeprefix("sha256:")
            artifact = history_path.parent / "analysis-sources" / f"{digest}.json"
            assert hashlib.sha256(artifact.read_bytes()).hexdigest() == digest
            retained.append(json.loads(artifact.read_text())["files"]["stock.rs"])
        assert retained == [original, concurrent, source.read_text()]
        logs = [json.loads(path.read_text()) for path in history_path.parent.glob("repairs/*.json")]
        assert sorted(log["state"] for log in logs) == ["reanalyzed", "rejected", "stopped"]
        assert (
            next(log for log in logs if log["state"] == "reanalyzed")["analysis_id"]
            == entries[-1]["id"]
        )
        command("/repair", "Only a retained mismatch/unclear Finding")
        assert len(requests) == 9
        terminal.send("/quit\r")
        assert terminal.process.wait(timeout=5) == 0
        terminal.close()
        terminal = Terminal(args, env, root)
        terminal.wait(lambda view: "Connection details" in view)
        assert len(requests) == 9
        terminal.send("y")
        terminal.wait(lambda view: "shift+enter" in view)
        command("/sources stock.rs", "Source graph ready")
        command(f"/analysis {entries[-2]['finding_id']}", f"Run: {entries[-2]['id']}")
        command("/repair", "source revision changed")
        assert len(requests) == 9
        terminal.send("/quit\r")
        assert terminal.process.wait(timeout=5) == 0
    finally:
        terminal.close()
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
