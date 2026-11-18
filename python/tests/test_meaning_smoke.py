"""The public repair driver rejects a model's matching verdict when real code fails."""

import json
import os
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from unittest.mock import patch

import pytest
from test_tui_setup import environment

from meden_checks import find_meden, meaning_smoke

pytestmark = pytest.mark.ux


@pytest.mark.parametrize("correct", [True, False], ids=["working-repair", "lying-model"])
def test_meaning_driver_uses_independent_rust_boundaries(tmp_path, tui_verifier, correct):
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
                    "file": "eligibility.rs",
                    "old_text": ">= 150",
                    "new_text": ">= 100" if correct else ">= 110",
                    "explanation": "Change the purchase threshold.",
                }
            elif tool == "record_informalization":
                fixed = ">= 150" not in data["files"]["eligibility.rs"]
                result = {
                    "lemmaName": "eligible",
                    "naturalLanguage": "threshold 100" if fixed else "threshold 150",
                    "preconditions": "Any u64 balance and stock",
                    "postcondition": "stock > 0 and balance reaches the threshold",
                    "scope": "All inputs",
                    "strength": "strong",
                    "confidence": 0.9,
                }
            else:
                fixed = "100" in data["backTranslation"]["naturalLanguage"]
                result = {
                    "lemmaName": "eligible",
                    "match": fixed,
                    "discrepancy": "" if fixed else "Threshold 150 differs from 100.",
                    "weakeningType": "none" if fixed else "wrong-property",
                    "explanation": "Both required conditions agree."
                    if fixed
                    else "The intended lower threshold is missing.",
                }
            # Real model explanations can exceed the visible transcript viewport.
            if tool == "record_informalization":
                result["naturalLanguage"] += "\nDetailed source explanation." * 45
            elif tool == "record_comparison":
                result["explanation"] += "\nDetailed requirement comparison." * 45
            response = json.dumps(
                {
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": None,
                                "tool_calls": [
                                    {
                                        "id": "synthetic-result",
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
    output = tmp_path / "repair-driver.json"
    try:
        binary = find_meden()
        with patch.dict(os.environ, environment(tmp_path), clear=True):
            result = meaning_smoke.main(
                [
                    "--binary",
                    binary,
                    "--base-url",
                    f"http://127.0.0.1:{server.server_port}/v1",
                    "--model",
                    "driver-fixture",
                    "--timeout",
                    "15",
                    "--out",
                    str(output),
                ]
            )
        report = json.loads(output.read_text())
        assert len(requests) == 5, report.get("error")
        assert report.get("reanalysis", {}).get("assessment", {}).get("verdict") == "match"
        assert report.get("success") == correct, report.get("error")
        assert result == (0 if correct else 1)
        observed = report["independent_check"]
        if correct:
            assert observed["result"]["exit_code"] == 0
        else:
            assert report["independent_host_exit_code"] != 0
            assert json.loads(observed["error"])["exit_code"] == 101
        assert report["unchanged_before_approval"] and report["requirement_preserved"]
        assert report["terminal_restored"]
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
