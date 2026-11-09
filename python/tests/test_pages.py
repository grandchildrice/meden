"""The public prefetch command cannot connect to private/proxy receivers."""

import json
import os
import socket
import subprocess

from meden_checks import find_meden


def test_prefetch_worker_bounds_requests_and_never_falls_back_from_missing_isolation(tmp_path):
    binary = find_meden()
    assert binary
    with socket.socket() as receiver:
        receiver.bind(("127.0.0.1", 0))
        receiver.listen()
        receiver.setblocking(False)
        target = f"https://127.0.0.1:{receiver.getsockname()[1]}/"
        for request in [
            {"urls": [target], "headers": {"authorization": "forged"}},
            [target, target],
            [target + str(i) for i in range(9)],
            [target],
        ]:
            result = subprocess.run(
                [binary, "worker-prefetch"], input=json.dumps(request),
                capture_output=True, text=True, timeout=10,
            )
            assert result.returncode != 0 and not result.stdout
        result = subprocess.run(
            [binary, "worker-prefetch"], input=" " * 524_289,
            capture_output=True, text=True, timeout=10,
        )
        assert result.returncode != 0 and "input budget" in result.stderr
        result = subprocess.run(
            [binary, "prefetch", "--url", target],
            env={**os.environ, "MEDEN_BWRAP": str(tmp_path / "missing-isolation")},
            capture_output=True, text=True, timeout=10,
        )
        assert result.returncode != 0 and not result.stdout
        try:
            receiver.accept()
        except BlockingIOError:
            pass
        else:
            raise AssertionError("a refused or unconfined prefetch reached the receiver")


def test_prefetch_refuses_local_targets_and_environment_proxies(tmp_path):
    binary = find_meden()
    assert binary is not None
    with socket.socket() as receiver:
        receiver.bind(("127.0.0.1", 0))
        receiver.listen()
        receiver.setblocking(False)
        port = receiver.getsockname()[1]
        env = {
            **os.environ,
            "HTTPS_PROXY": f"http://127.0.0.1:{port}",
            "ALL_PROXY": f"http://127.0.0.1:{port}",
            "NO_PROXY": "",
        }
        for target in [
            f"https://127.0.0.1:{port}/",
            f"https://localhost:{port}/",
            f"https://[::ffff:127.0.0.1]:{port}/",
            f"http://127.0.0.1:{port}/",
            "https://user:password@example.test/",
            "https://example.test/#fragment",
        ]:
            result = subprocess.run(
                [binary, "prefetch", "--url", target],
                env=env,
                capture_output=True,
                text=True,
                timeout=10,
            )
            assert result.returncode != 0
            assert not result.stdout
        try:
            receiver.accept()
        except BlockingIOError:
            pass
        else:
            raise AssertionError("prefetch connected to a private or environment proxy receiver")
    bad_cache = tmp_path / "bad-pages.json"
    bad_cache.write_text('{"pages":[], "approved":true}')
    result = subprocess.run(
        [binary, "run", "--pages", str(bad_cache)],
        env=env,
        capture_output=True,
        text=True,
        timeout=10,
    )
    assert result.returncode != 0
    assert "unknown field" in result.stderr
    assert not result.stdout
