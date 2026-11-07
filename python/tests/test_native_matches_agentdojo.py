"""Compare actual HTTP requests, not just system-prompt strings.

Both agents talk to one deterministic local model fixture and run the real
banking tools. Differences in roles, content blocks, schemas, result YAML,
call ids, argument numbers or sampling defaults fail this test.
"""

import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest
from agentdojo.functions_runtime import FunctionsRuntime
from agentdojo.task_suite.load_suites import get_suite

from meden_agentdojo import find_meden, toolcalling
from meden_agentdojo.declarations import BANKING
from meden_agentdojo.pipeline import MedenPipeline


def _call(name, args, call_id):
    return {
        "id": call_id,
        "type": "function",
        "function": {
            "name": name,
            "arguments": json.dumps(args, ensure_ascii=False, separators=(",", ":")),
        },
    }


def test_native_http_conversation_matches_agentdojo(monkeypatch):
    binary = find_meden()
    if binary is None:
        pytest.skip("meden binary not built")
    replies = [
        {
            "content": None,
            "tool_calls": [
                _call("get_user_info", {}, "user-info"),
                _call("get_most_recent_transactions", {"n": 2}, "transactions"),
            ],
        },
        {
            "content": "Checked the history.",
            "tool_calls": [
                _call(
                    "send_money",
                    {
                        "recipient": "GB29NWBK60161331926819",
                        "amount": 5.29,
                        "subject": "返金🙂",
                        "date": "2022-04-01",
                    },
                    "refund",
                )
            ],
        },
        {"content": "Refunded 5.29.", "tool_calls": None},
    ]
    requests = []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_GET(self):
            body = {"data": [{"id": "fixture"}]}
            self._send(body)

        def _send(self, body):
            payload = json.dumps(body).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

        def do_POST(self):
            request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            reply = replies[len(requests) % len(replies)]
            requests.append(request)
            self._send(
                {
                    "id": "fixture",
                    "model": "fixture",
                    "created": 0,
                    "object": "chat.completion",
                    "choices": [
                        {
                            "index": 0,
                            "finish_reason": "tool_calls" if reply["tool_calls"] else "stop",
                            "message": {"role": "assistant", **reply},
                        }
                    ],
                }
            )

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    base = f"http://127.0.0.1:{server.server_port}/v1"
    monkeypatch.setenv("MEDEN_LLM_BASE_URL", base)
    monkeypatch.setenv("MEDEN_LLM_MODEL", "fixture")
    monkeypatch.delenv("MEDEN_LLM_API_KEY", raising=False)
    suite = get_suite("v1", "banking")
    monkeypatch.setattr(toolcalling, "api_key", lambda: "fixture")
    reference = toolcalling.build(model="fixture", base_url=base)
    native = MedenPipeline(BANKING, binary, provider="configured", mode="tool-calling")
    environments = []
    try:
        for pipeline in (reference, native):
            env = suite.load_and_inject_default_environment({})
            pipeline.query("Check my history and refund 5.29.", FunctionsRuntime(suite.tools), env)
            environments.append(env.model_dump(mode="json"))
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
    assert len(requests) == 6
    assert requests[:3] == requests[3:]
    assert environments[0] == environments[1]
    assert native.last_report.answer == "Refunded 5.29."
    assert len(native.last_report.messages) == 8
    assert reference.last_report.outcome == "done"
    assert reference.last_report.answer == native.last_report.answer
    assert reference.last_report.messages == native.last_report.messages


def test_baseline_observer_failure_does_not_change_the_conversation(monkeypatch):
    monkeypatch.setattr(toolcalling, "api_key", lambda: "fixture")
    pipeline = toolcalling.build(model="fixture", base_url="http://127.0.0.1:1/v1")

    def broken_encoder(*args):
        raise RuntimeError("observer failure")

    monkeypatch.setattr(toolcalling, "_message_to_openai", broken_encoder)
    runtime, env, extra = object(), object(), {}
    messages = [{"role": "assistant", "content": [], "tool_calls": None}]
    result = pipeline.elements[-1].query("query", runtime, env, messages, extra)
    assert result == ("query", runtime, env, messages, extra)
    assert result[3] is messages
    assert pipeline.last_report.outcome == "unreported"
