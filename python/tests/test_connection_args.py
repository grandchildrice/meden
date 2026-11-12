"""Observe command-line connection selection at the actual HTTP boundary."""

import json
import subprocess

import pytest
from test_model_transport import environment, receiver

from meden_checks import find_meden


@pytest.mark.parametrize("authentication", ["argument", "environment", "none"])
def test_command_line_connection_is_exact_and_never_borrows_another_key(tmp_path, authentication):
    with receiver() as (other, other_requests), receiver() as (base, requests):
        # Conflicting connection and local defaults must not select another
        # model/credential, or trigger discovery GETs for an explicit URL.
        env = environment(other, tmp_path)
        env.update(CLI_TEST_KEY="COMMAND-KEY-CANARY", MEDEN_LOCAL_ENGINE="/not-an-engine",
                   MEDEN_LOCAL_MODEL="/not-weights")
        args = [find_meden(), "run", "--workspace", str(tmp_path / "project"),
                "--base-url", f"{base}/v1/", "--model", "command-line-model",
                "--approve-processor", f"{base}/v1/chat/completions"]
        if authentication == "argument":
            args += ["--api-key", "COMMAND-KEY-CANARY"]
        elif authentication == "environment":
            args += ["--api-key-env", "CLI_TEST_KEY"]
        completed = subprocess.run(
            args, input='{"instruction":"Inspect this project."}\n',
            text=True, capture_output=True, env=env, timeout=20,
        )
    assert completed.returncode == 0, completed.stderr
    assert other_requests == [] and len(requests) == 1
    method, path, headers, body = requests[0]
    assert (method, path) == ("POST", "/v1/chat/completions")
    assert headers.get("authorization") == (
        None if authentication == "none" else "Bearer COMMAND-KEY-CANARY"
    )
    assert json.loads(body)["model"] == "command-line-model"
    assert "COMMAND-KEY-CANARY" not in completed.stdout + completed.stderr
    assert "fixture-key" not in completed.stdout + completed.stderr
    assert json.loads(completed.stdout.splitlines()[-1])["outcome"] == "done"


@pytest.mark.parametrize("options", [
    ["--base-url", "{base}/v1"],
    ["--api-key", "COMMAND-KEY-CANARY"],
    ["--base-url", "{base}/v1", "--model", "fixture", "--provider", "claude"],
    ["--base-url", "{base}/v1", "--model", "fixture", "--local-engine", "/engine"],
    ["--base-url", "{base}/v1", "--model", "fixture", "--api-key", "COMMAND-KEY-CANARY",
     "--api-key-env", "CLI_TEST_KEY"],
    ["--base-url", "{base}/v1", "--model", "fixture", "--api-key-env", "CLI_TEST_KEY"],
    ["--base-url", "{base}/v1", "--model", "fixture", "--api-key", ""],
])
def test_invalid_connection_stops_before_requests_without_printing_keys(tmp_path, options):
    with receiver() as (base, requests):
        env = environment(base, tmp_path)
        env["CLI_TEST_KEY"] = ""
        completed = subprocess.run(
            [find_meden(), "run", "--workspace", str(tmp_path / "project"),
             "--approve-processor", f"{base}/v1/chat/completions",
             *[option.format(base=base) for option in options]],
            input='{"instruction":"Inspect this project."}\n',
            text=True, capture_output=True, env=env, timeout=15,
        )
    assert completed.returncode != 0 and requests == []
    assert "COMMAND-KEY-CANARY" not in completed.stdout + completed.stderr
    assert "fixture-key" not in completed.stdout + completed.stderr
