"""Public second-model review, failure without fallback, recovery and restart."""

import fcntl
import json
import struct
import termios
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pyte
import pytest
from test_tui_setup import Terminal, environment

from meden_checks import find_meden

pytestmark = pytest.mark.ux


@pytest.mark.parametrize("failure", ["http", "invalid-category"])
def test_separate_judge_review_failure_recovery_and_restart(tmp_path, tui_verifier, failure):
    project = tmp_path / "project"
    project.mkdir()
    (project / "stock.rs").write_text("// SOURCE_ONLY\npub fn stock(n: u64) -> u64 { n }\n")
    requests = []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            requests.append(body)
            name = body["tools"][0]["function"]["name"]
            assert body["model"] == (
                "translator-fixture" if name == "record_informalization" else "judge-fixture"
            ), "comparison must never fall back to the primary model"
            if len(requests) == 2 and failure == "http":
                self.send_response(401)
                self.send_header("Content-Length", "0")
                self.end_headers()
                return
            data = (
                dict(
                    lemmaName="stock",
                    naturalLanguage="Stock stays unchanged.",
                    preconditions="Any u64",
                    postcondition="Output equals input",
                    scope="One call",
                    strength="moderate",
                    confidence=0.9,
                )
                if name == "record_informalization"
                else {
                    "lemmaName": "stock",
                    "match": False,
                    "discrepancy": "Missing decrement",
                    "weakeningType": "missing-case",
                    "explanation": "Input remains unchanged.",
                }
            )
            if len(requests) == 2 and failure == "invalid-category":
                data["weakeningType"] = "wrong property"
            payload = json.dumps(
                {
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": None,
                                "tool_calls": [
                                    {
                                        "id": "candidate",
                                        "type": "function",
                                        "function": {"name": name, "arguments": json.dumps(data)},
                                    }
                                ],
                            }
                        }
                    ]
                }
            ).encode()
            self.send_response(200)
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    endpoint = f"http://127.0.0.1:{server.server_port}/v1"
    args = [
        find_meden(),
        "tui",
        "--workspace",
        str(project),
        "--base-url",
        endpoint,
        "--model",
        "translator-fixture",
        "--judge-model",
        "judge-fixture",
    ]
    env = environment(tmp_path)
    terminal = Terminal(args, env, project)

    def command(text, result, count):
        terminal.paste(text)
        terminal.send("\r")
        terminal.wait(
            lambda view: result in view and "shift+enter" in view and len(requests) == count
        )

    try:
        terminal.wait(
            lambda view: "Judge model: judge-fixture" in view and "Allow for this session" in view
        )
        assert not requests
        for rows, columns in [(12, 45), (36, 130)]:
            fcntl.ioctl(
                terminal.slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, columns, 0, 0)
            )
            terminal.screen = pyte.Screen(columns, rows)
            terminal.stream = pyte.Stream(terminal.screen)
            if rows == 12:
                terminal.wait(lambda view: "Enlarge" in view)
                terminal.send("y")
                for _ in range(6):
                    terminal.pump()
                assert not requests
            else:
                terminal.wait(
                    lambda view: (
                        "Judge model: judge-fixture" in view and "Allow for this session" in view
                    )
                )
        terminal.send("y")
        terminal.wait(lambda view: "shift+enter" in view)
        command("/sources stock.rs", "Source graph ready", 0)
        command(
            "/analyze stock.rs:2 stock -- REQUIREMENT_ONLY: decrement every input",
            "Analysis stopped",
            2,
        )
        command("/analysis", "Blind translation retained", 2)
        command(
            "/analyze stock.rs:2 stock -- REQUIREMENT_ONLY: decrement every input",
            "Candidate Mismatch",
            4,
        )
        assert [r["model"] for r in requests] == ["translator-fixture", "judge-fixture"] * 2
        assert "SOURCE_ONLY" in json.dumps(requests[2])
        assert "REQUIREMENT_ONLY" not in json.dumps(requests[2])
        assert "SOURCE_ONLY" not in json.dumps(requests[3])
        assert "REQUIREMENT_ONLY" in json.dumps(requests[3])
        saved = next((tmp_path / "data/meden/projects").glob("*/meaning-analysis.json"))
        records = json.loads(saved.read_text())["entries"]
        assert records[0]["state"] == "failed" and records[0]["assessment"] is None
        assert records[0]["partial_translation"]["naturalLanguage"] == "Stock stays unchanged."
        assert all(
            r["judge"] == {"model": "judge-fixture", "endpoint": endpoint + "/chat/completions"}
            for r in records
        )
        terminal.quit()
        terminal.close()
        terminal = Terminal(args, env, project)
        terminal.wait(lambda view: "Judge model: judge-fixture" in view)
        terminal.send("n")
        terminal.wait(lambda view: "Choose a model connection" in view)
        terminal.send("\x1b")
        terminal.wait(lambda view: "shift+enter" in view)
        command("/analysis", "Historical", 4)
        terminal.quit()
    finally:
        terminal.close()
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
