"""Observe actual CLI model traffic, redirects and uncertain delivery."""

import json
import os
import subprocess
import threading
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest

from meden_agentdojo import find_meden


@contextmanager
def receiver(status=200, location=None, disconnect=False):
    requests = []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def handle_request(self):
            body = self.rfile.read(int(self.headers.get("Content-Length", "0")))
            requests.append((self.command, self.path, dict(self.headers), body))
            if disconnect:
                self.close_connection = True
                return
            payload = json.dumps(
                {
                    "data": [{"id": "fixture"}],
                    "choices": [
                        {
                            "finish_reason": "stop",
                            "message": {"role": "assistant", "content": "received"},
                        }
                    ],
                }
            ).encode()
            self.send_response(status)
            if location:
                self.send_header("Location", location)
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

        do_POST = do_GET = handle_request

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}", requests
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)


def environment(base, tmp_path):
    env = {**os.environ}
    for key in (
        "ALL_PROXY",
        "HTTPS_PROXY",
        "HTTP_PROXY",
        "NO_PROXY",
        "all_proxy",
        "https_proxy",
        "http_proxy",
        "no_proxy",
        "OLLAMA_API_KEY",
    ):
        env.pop(key, None)
    env.update(
        MEDEN_LLM_BASE_URL=f"{base}/v1",
        MEDEN_LLM_MODEL="fixture",
        MEDEN_LLM_API_KEY="fixture-key",
        HERMES_HOME=str(tmp_path / "absent-hermes"),
    )
    return env


def cli(env, command):
    binary = find_meden()
    assert binary is not None, "meden binary must be built"
    start = {
        "instruction": "CONFIDENTIAL-CANARY",
        "tools": [],
        "policy": {"name": "fixture", "grants": [], "denies": []},
        "max_iterations": 1,
    }
    completed = subprocess.run(
        [binary, *command],
        input=json.dumps(start) + "\n",
        text=True,
        capture_output=True,
        env=env,
        timeout=20,
    )
    assert completed.returncode == 0, completed.stderr
    return completed.stdout


@pytest.mark.parametrize(
    "status,disconnect", [(302, False), (307, False), (308, False), (503, False), (200, True)]
)
def test_no_redirect_or_automatic_retry_after_confidential_send(tmp_path, status, disconnect):
    with receiver() as (other, other_requests):
        with receiver(status, f"{other}/unapproved", disconnect) as (base, requests):
            output = cli(environment(base, tmp_path), ["run", "--provider", "configured"])
    assert len(requests) == 1
    method, path, headers, body = requests[0]
    assert (method, path) == ("POST", "/v1/chat/completions")
    assert headers["authorization"] == "Bearer fixture-key"
    assert b"CONFIDENTIAL-CANARY" in body
    assert other_requests == []
    done = json.loads(output.splitlines()[-1])
    assert done["outcome"] == "failed"


def test_model_and_discovery_ignore_environment_proxy(tmp_path):
    with receiver() as (proxy, proxy_requests):
        with receiver() as (base, requests):
            env = environment(base, tmp_path)
            # NO_PROXY is absent: even a loopback request would otherwise be
            # sent to this real proxy, including its credential and body.
            env.update(ALL_PROXY=proxy, HTTP_PROXY=proxy, HTTPS_PROXY=proxy)
            env.update(OLLAMA_API_KEY="fixture-discovery-key", OLLAMA_HOST=base)
            providers = cli(env, ["providers"])
            assert "fixture" in providers
            output = cli(env, ["run", "--provider", "configured"])
    assert proxy_requests == []
    assert [(method, path) for method, path, _, _ in requests] == [
        ("GET", "/v1/models"),
        ("GET", "/v1/models"),
        ("POST", "/v1/chat/completions"),
    ]
    assert b"CONFIDENTIAL-CANARY" in requests[-1][3]
    assert json.loads(output.splitlines()[-1])["outcome"] == "done"
