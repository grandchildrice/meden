"""Public source/meaning journey shares the GUI's service and persisted result."""

import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest
from test_tui_setup import Terminal, environment

from meden_checks import find_meden

pytestmark = pytest.mark.ux


def test_tui_meaning_source_freshness_cancel_and_historical_restart(tmp_path, tui_verifier):
    root = tmp_path / "project"
    root.mkdir()
    (root / "stock.rs").write_text("// SOURCE_ONLY_SENTINEL\npub fn stock(n: u64) -> u64 { n }\n")
    requests = []
    translating = threading.Event()
    release = threading.Event()
    fail_next = threading.Event()
    hold_next = threading.Event()

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            requests.append(body)
            translating.set()
            if hold_next.is_set():
                hold_next.clear()
                release.wait(15)
            if fail_next.is_set():
                fail_next.clear()
                self.send_response(429)
                self.send_header("Content-Length", "0")
                self.end_headers()
                return
            tool = body["tools"][0]["function"]["name"]
            data = (
                {
                    "lemmaName": "stock",
                    "naturalLanguage": "The stock remains unchanged.",
                    "preconditions": "Any u64 stock",
                    "postcondition": "Output equals input",
                    "scope": "One transition",
                    "strength": "moderate",
                    "confidence": 0.9,
                }
                if tool == "record_informalization"
                else {
                    "lemmaName": "stock",
                    "match": False,
                    "discrepancy": "No decrement is established.",
                    "weakeningType": "missing-case",
                    "explanation": "Unchanged stock does not satisfy the requested decrement.",
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
                                        "id": "data-only",
                                        "type": "function",
                                        "function": {
                                            "name": tool,
                                            "arguments": json.dumps(data),
                                        },
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
        "meaning-fixture",
        "--api-key",
        "SYNTHETIC-ANALYSIS-KEY",
    ]
    terminal = None

    def command(text, result, request_count=None):
        terminal.paste(text)
        terminal.send("\r")
        terminal.wait(
            lambda view: (
                result in view
                and "shift+enter" in view
                and (request_count is None or len(requests) == request_count)
            )
        )

    try:
        terminal = Terminal(args, env, root)
        terminal.wait(lambda view: "Connection details" in view)
        terminal.send("y")
        terminal.wait(lambda view: "shift+enter" in view)
        command(
            "/analyze stock -- REQUIREMENT_ONLY_SENTINEL: Always decrement stock.",
            "Read the source graph first",
        )
        assert requests == []
        command("/sources stock.rs", "Source graph ready")
        command(
            "/analyze stock.rs:2 stock -- REQUIREMENT_ONLY_SENTINEL: Always decrement stock.",
            "Candidate Mismatch",
        )
        assert len(requests) == 2
        assert "SOURCE_ONLY_SENTINEL" in json.dumps(requests[0])
        assert "REQUIREMENT_ONLY_SENTINEL" not in json.dumps(requests[0])
        assert "REQUIREMENT_ONLY_SENTINEL" in json.dumps(requests[1])
        assert "SOURCE_ONLY_SENTINEL" not in json.dumps(requests[1])
        # A held actual HTTP request allows cancellation between the two passes.
        translating.clear()
        release.clear()
        hold_next.set()
        terminal.paste("/analyze stock.rs:2 stock -- Decrement stock")
        terminal.send("\r")
        terminal.wait(lambda view: "Translating" in view and "shift+enter" not in view)
        assert translating.wait(5)
        terminal.send(b"\x03")
        terminal.wait(lambda view: "Stopping tools" in view)
        release.set()
        terminal.wait(lambda view: "Analysis stopped" in view and "shift+enter" in view)
        assert len(requests) == 3, "cancellation must stop the comparison request"
        command("/analysis", "Cancelled")
        # A new explicit operation can retry; provider failure never revives a result.
        fail_next.set()
        command("/analyze stock.rs:2 stock -- Retry decrement stock", "Analysis stopped", 4)
        command("/analysis", "Failed")
        assert len(requests) == 4
        command(
            "/analyze stock.rs:2 stock -- Final decrement stock",
            "Requirement: Final decrement stock",
            6,
        )
        assert len(requests) == 6
        translating.clear()
        release.clear()
        hold_next.set()
        terminal.paste("/analyze stock.rs:2 stock -- Detect an external source change")
        terminal.send("\r")
        terminal.wait(lambda view: "Translating" in view and "shift+enter" not in view)
        assert translating.wait(5)
        # Deliberate concurrent host mutation tests refusal, never repairs the journey.
        (root / "stock.rs").write_text(
            "// CHANGED_SOURCE_SENTINEL\npub fn stock(n: u64) -> u64 { n - 1 }\n"
        )
        release.set()
        terminal.wait(
            lambda view: "Sources changed during meaning analysis" in view and "shift+enter" in view
        )
        command("/analysis", "Stale")
        assert len(requests) == 8
        command("/sources stock.rs", "Source graph ready")
        command(
            "/analyze stock.rs:2 stock -- Rechecked source", "Requirement: Rechecked source", 10
        )
        checkpoints = list((tmp_path / "data/meden/projects").glob("*/meaning-analysis.json"))
        assert len(checkpoints) == 1
        saved = json.loads(checkpoints[0].read_text())
        entries = saved["entries"]
        assert len(entries) == 6, (
            "completion, cancellation, failure and stale runs must all survive"
        )
        assert [entry["state"] for entry in entries] == [
            "complete",
            "cancelled",
            "failed",
            "complete",
            "stale",
            "complete",
        ]
        assert len({entry["id"] for entry in entries}) == 6
        assert all(entry["finding_id"] is None for entry in entries if not entry["assessment"])
        original_id = entries[0]["id"]
        finding_id = entries[0]["finding_id"]
        assert (
            finding_id
            and entries[0]["input"]["source_revision"] != entries[-1]["input"]["source_revision"]
        )
        terminal.send("/quit\r")
        assert terminal.process.wait(timeout=5) == 0
        terminal.close()
        terminal = Terminal(args, env, root)
        terminal.wait(lambda view: "Connection details" in view)
        assert len(requests) == 10, "historical results cannot restore a processor grant"
        terminal.send("y")
        terminal.wait(lambda view: "shift+enter" in view)
        command("/analysis", "Historical")
        command("/analysis history", original_id)
        command(f"/analysis {finding_id}", f"Run: {original_id}")
        assert len(requests) == 10
        terminal.send("/quit\r")
        assert terminal.process.wait(timeout=5) == 0
    finally:
        release.set()
        if terminal is not None:
            terminal.close()
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
