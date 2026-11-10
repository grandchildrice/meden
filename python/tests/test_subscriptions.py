"""Native subscription protocols through the real host approval/tool boundary.

All credentials and HTTP replies here are fixtures; these tests do not use a
subscription account or establish live provider entitlement.
"""

import json
import os
import select
import subprocess
import threading
import time
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import pytest

from meden_checks import find_meden


def call(call_id, name, arguments):
    return {"id": call_id, "name": name, "arguments": arguments}


def native_reply(provider, content="ready", calls=(), stop=None):
    if provider == "codex":
        output = [
            {
                "type": "function_call",
                "call_id": c["id"],
                "name": c["name"],
                "arguments": json.dumps(c["arguments"], ensure_ascii=False),
                "status": "completed",
            }
            for c in calls
        ]
        if content is not None:
            output.insert(
                0,
                {
                    "type": "message",
                    "status": "completed",
                    "content": [{"type": "output_text", "text": content}],
                },
            )
        event = {
            "type": "response.completed",
            "response": {"status": stop or "completed", "output": output},
        }
        return ("data: " + json.dumps(event, ensure_ascii=False) + "\r\n\r\n").encode()
    blocks = [
        {"type": "tool_use", "id": c["id"], "name": c["name"], "input": c["arguments"]}
        for c in calls
    ]
    if content is not None:
        blocks.insert(0, {"type": "text", "text": content})
    return json.dumps(
        {
            "type": "message",
            "role": "assistant",
            "content": blocks,
            "stop_reason": stop or ("tool_use" if calls else "end_turn"),
        },
        ensure_ascii=False,
    ).encode()


@contextmanager
def receiver(provider, replies, status=200, location=None, after_request=None):
    requests = []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            index = len(requests)
            requests.append((self.path, dict(self.headers), body))
            if after_request is not None:
                after_request(index)
            payload = replies[min(index, len(replies) - 1)]
            self.send_response(status)
            self.send_header(
                "Content-Type", "text/event-stream" if provider == "codex" else "application/json"
            )
            if location:
                self.send_header("Location", location)
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}/v1", requests
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)


def environment(tmp_path, provider, base, source="cli"):
    env = {
        k: v
        for k, v in os.environ.items()
        if not k.startswith(
            (
                "MEDEN_LLM_",
                "MEDEN_CODEX_",
                "MEDEN_CLAUDE_",
                "OLLAMA_",
                "OPENAI_",
                "ANTHROPIC_",
                "CLAUDE_CODE_",
            )
        )
    }
    codex, claude, hermes = (tmp_path / name for name in ("codex", "claude", "hermes"))
    for directory in (codex, claude, hermes):
        directory.mkdir(exist_ok=True)
    token = f"{provider}-SECRET-fixture"
    tokens = {"access_token": token, "account_id": "account-fixture"}
    if provider == "codex":
        if source == "hermes":
            (hermes / "auth.json").write_text(
                json.dumps({"providers": {"openai-codex": {"tokens": tokens}}})
            )
            (hermes / "config.yaml").write_text(
                "model:\n  provider: openai-codex\n  model: host-model\n"
            )
        else:
            (codex / "auth.json").write_text(json.dumps({"tokens": tokens}))
    elif source == "hermes":
        (hermes / ".env").write_text(f"ANTHROPIC_TOKEN={token}\n")
    else:
        (claude / ".credentials.json").write_text(
            json.dumps({"claudeAiOauth": {"accessToken": token}})
        )
    env.update(
        CODEX_HOME=str(codex),
        CLAUDE_CONFIG_DIR=str(claude),
        HERMES_HOME=str(hermes),
        XDG_DATA_HOME=str(tmp_path / "data"),
        **{f"MEDEN_{provider.upper()}_BASE_URL": base},
    )
    return env


def run(tmp_path, provider, base, env, approval=True):
    binary = find_meden()
    assert binary is not None, "build meden-cli first"
    source = tmp_path / "source"
    source.mkdir(exist_ok=True)
    endpoint = base + ("/responses" if provider == "codex" else "/messages")
    command = [
        binary,
        "run",
        "--workspace",
        str(source),
        "--provider",
        provider,
        "--model",
        "fixture",
    ]
    if approval:
        command += ["--approve-processor", endpoint if approval is True else approval]
    return subprocess.run(
        command,
        env=env,
        input=json.dumps({"instruction": "Build the requested files. 日本語🙂"}) + "\n",
        text=True,
        capture_output=True,
        timeout=30,
    )


@pytest.mark.parametrize("provider", ["codex", "claude"])
@pytest.mark.parametrize("source", ["cli", "hermes"])
def test_subscription_parallel_results_and_frozen_auth(tmp_path, provider, source):
    calls = [
        call("write-a", "write_file", {"path": "a.txt", "content": "日本語🙂\n"}),
        call("write-b", "write_file", {"path": "b.txt", "content": "second\n"}),
    ]
    replies = [native_reply(provider, calls=calls), native_reply(provider)]
    env = None

    def mutate_auth(index):
        if index == 0:
            # A later token/account edit must not replace the approved snapshot.
            Path(env["CODEX_HOME"], "auth.json").write_text(
                json.dumps(
                    {
                        "tokens": {
                            "access_token": "UNAPPROVED-token",
                            "account_id": "UNAPPROVED-account",
                        }
                    }
                )
            )
            Path(env["CLAUDE_CONFIG_DIR"], ".credentials.json").write_text(
                json.dumps({"claudeAiOauth": {"accessToken": "UNAPPROVED-token"}})
            )

    with receiver(provider, replies, after_request=mutate_auth) as (base, requests):
        env = environment(tmp_path, provider, base, source)
        env.update(HTTPS_PROXY="http://127.0.0.1:1", ALL_PROXY="http://127.0.0.1:1")
        completed = run(tmp_path, provider, base, env)
    assert completed.returncode == 0, completed.stderr
    records = [json.loads(line) for line in completed.stdout.splitlines()]
    assert records[-1]["outcome"] == "done", records
    assert len(requests) == 2
    workspace = Path(next(record["workspace"] for record in records if "workspace" in record))
    assert (workspace / "a.txt").read_text() == "日本語🙂\n"
    assert (workspace / "b.txt").read_text() == "second\n"
    assert list((tmp_path / "source").iterdir()) == []
    assert "SECRET-fixture" not in completed.stdout + completed.stderr
    for path, headers, body in requests:
        assert path == ("/v1/responses" if provider == "codex" else "/v1/messages")
        assert headers["authorization"] == f"Bearer {provider}-SECRET-fixture"
        assert body["model"] == "fixture"
        if provider == "codex":
            assert headers["chatgpt-account-id"] == "account-fixture"
            assert body["store"] is False and body["stream"] is True
            assert "max_output_tokens" not in body
            assert body["instructions"]
            assert body["tools"][0]["type"] == "function"
        else:
            assert headers["anthropic-version"] == "2023-06-01"
            assert headers["anthropic-beta"] == "oauth-2025-04-20"
            assert body["system"]
            assert body["max_tokens"] > 0
            assert "input_schema" in body["tools"][0]
    last = requests[-1][2]
    if provider == "codex":
        results = [item for item in last["input"] if item.get("type") == "function_call_output"]
        assert [item["call_id"] for item in results] == ["write-a", "write-b"]
        replay = [item for item in last["input"] if item.get("type") == "function_call"]
        assert [
            (item["call_id"], item["name"], json.loads(item["arguments"])) for item in replay
        ] == [(c["id"], c["name"], c["arguments"]) for c in calls]
    else:
        results = last["messages"][-1]["content"]
        assert [item["tool_use_id"] for item in results] == ["write-a", "write-b"]
        replay = last["messages"][-2]["content"][1:]
        assert [item["input"] for item in replay] == [c["arguments"] for c in calls]


@pytest.mark.parametrize("provider", ["codex", "claude"])
@pytest.mark.parametrize("approval", [False, "http://127.0.0.1:1/wrong"])
def test_subscription_needs_exact_host_approval(tmp_path, provider, approval):
    with receiver(provider, [native_reply(provider)]) as (base, requests):
        completed = run(tmp_path, provider, base, environment(tmp_path, provider, base), approval)
    assert completed.returncode != 0
    assert requests == []


@pytest.mark.parametrize("provider", ["codex", "claude"])
@pytest.mark.parametrize("status", [302, 307, 308, 401, 403, 429, 503])
def test_subscription_failure_never_redirects_retries_or_falls_back(tmp_path, provider, status):
    with receiver(provider, [native_reply(provider)]) as (other, other_requests):
        with receiver(provider, [native_reply(provider)], status, other) as (base, requests):
            completed = run(tmp_path, provider, base, environment(tmp_path, provider, base))
    assert completed.returncode == 0, completed.stderr
    assert json.loads(completed.stdout.splitlines()[-1])["outcome"] == "failed"
    assert len(requests) == 1
    assert other_requests == []
    assert "SECRET-fixture" not in completed.stdout + completed.stderr


@pytest.mark.parametrize("provider", ["codex", "claude"])
@pytest.mark.parametrize("fault", ["truncated", "duplicate", "failure", "malformed", "framing"])
def test_invalid_subscription_replies_cannot_execute_partial_calls(tmp_path, provider, fault):
    calls = [call("unsafe", "write_file", {"path": "must-not-exist.txt", "content": "bad"})]
    if fault == "duplicate":
        calls *= 2
    payload = native_reply(provider, calls=calls)
    if fault == "truncated":
        if provider == "codex":
            payload = b'data: {"type":"response.output_item.done","item":{}}\n\n'
        else:
            payload = native_reply(provider, calls=calls, stop="max_tokens")
    elif fault == "failure":
        if provider == "codex":
            payload += b'data: {"type":"response.failed"}\n\n'
        else:
            payload = native_reply(provider, calls=calls, stop="refusal")
    elif fault == "malformed":
        payload = b"data: invalid\n\n" if provider == "codex" else b'{"secret":'
    elif fault == "framing":
        payload = payload.rstrip() if provider == "codex" else payload[:-1]
    with receiver(provider, [payload]) as (base, requests):
        completed = run(tmp_path, provider, base, environment(tmp_path, provider, base))
    assert completed.returncode == 0, completed.stderr
    records = [json.loads(line) for line in completed.stdout.splitlines()]
    assert records[-1]["outcome"] == "failed"
    assert len(requests) == 1
    workspace = Path(next(record["workspace"] for record in records if "workspace" in record))
    assert not (workspace / "must-not-exist.txt").exists()
    assert not any(record["type"] == "tool_result" for record in records)


@pytest.mark.parametrize("provider", ["codex", "claude"])
def test_subscription_tui_review_precedes_native_model_request(tmp_path, provider):
    import fcntl
    import pty
    import struct
    import termios

    binary = find_meden()
    assert binary is not None
    project = tmp_path / "project"
    project.mkdir()
    with receiver(provider, [native_reply(provider, "SUBSCRIPTION-TUI-OK")]) as (base, requests):
        env = environment(tmp_path, provider, base)
        env["TERM"] = "xterm-256color"
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 35, 130, 0, 0))
        initial = termios.tcgetattr(slave)
        process = subprocess.Popen(
            [
                binary,
                "tui",
                "--workspace",
                str(project),
                "--provider",
                provider,
                "--model",
                "fixture",
            ],
            stdin=slave,
            stdout=slave,
            stderr=slave,
            env=env,
        )
        output = bytearray()

        def wait_for(predicate):
            deadline = time.monotonic() + 12
            while time.monotonic() < deadline:
                if predicate():
                    return
                assert process.poll() is None, output.decode(errors="replace")
                if select.select([master], [], [], 0.1)[0]:
                    output.extend(os.read(master, 65536))
            pytest.fail(f"TUI timeout: {output[-5000:].decode(errors='replace')}")

        try:
            wait_for(lambda: b"subscription" in output)
            os.write(master, b"\r")
            wait_for(lambda: b"processor approval" in output)
            assert base.encode() in output
            assert requests == []
            os.write(master, b"\x1b[200~y\x1b[201~\r")
            time.sleep(0.1)
            assert requests == []
            os.write(master, b"y")
            wait_for(lambda: b"alt-enter" in output)
            os.write(master, b"Hello\r")
            wait_for(lambda: b"SUBSCRIPTION-TUI-OK" in output)
            assert len(requests) == 1
            assert b"SECRET-fixture" not in output
            os.write(master, b"\x03")
            process.wait(timeout=10)
            assert termios.tcgetattr(slave) == initial
        finally:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)
            os.close(master)
            os.close(slave)
