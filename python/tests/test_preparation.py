"""Repository PATH entries must not execute during host-side preparation."""

import hashlib
import json
import os
import shutil
import socket
import subprocess
import sys
from pathlib import Path

import pytest

from meden_checks import find_meden


def test_installed_bubblewrap_needs_no_path_or_environment_setup(tmp_path):
    binary = find_meden()
    assert binary
    fixed = [Path(directory) / "bwrap" for directory in (
        "/usr/bin", "/bin", "/usr/local/bin", "/run/current-system/sw/bin",
        "/nix/var/nix/profiles/default/bin",
    )]
    mode = "fixed-system-location"
    if not any(path.is_file() for path in fixed):
        helper = Path("/run/current-system/sw/bin/nix-store")
        profile = Path("/run/current-system/sw")
        if not helper.is_file() or not profile.is_dir():
            pytest.skip("requires installed system Bubblewrap or a real Nix software closure")
        resolved = helper.resolve()
        assert resolved.is_relative_to("/nix/store")
        query = subprocess.run(
            ["nix-store", "--query", "--requisites", str(profile)],
            executable=str(resolved), cwd="/", text=True, capture_output=True,
            timeout=10, check=True,
        )
        roots = [Path(line) for line in query.stdout.splitlines()]
        assert all(root.parent == Path("/nix/store") for root in roots)
        if not any((root / "bin/bwrap").is_file() for root in roots):
            pytest.skip("real Nix system software closure has no Bubblewrap")
        mode = "real-nix-software-closure"

    source = tmp_path / "source"
    hostile = source / "bin"
    hostile.mkdir(parents=True)
    marker = tmp_path / "untrusted-host-execution"
    script = (
        f"#!{sys.executable}\nimport pathlib\n"
        f"pathlib.Path({str(marker)!r}).write_text('executed before isolation')\n"
        "raise SystemExit(1)\n"
    )
    for name in ("bwrap", "nix-store"):
        helper = hostile / name
        helper.write_text(script)
        helper.chmod(0o700)
    startup = tmp_path / "bash-startup"
    startup.write_text(f"printf startup > '{marker}'\n")

    # On conventional Linux, removing /usr/bin from PATH also hides Bash.
    # Expose only canonical public helpers, without exposing bwrap. The
    # application still rejects the repository's own helper executables.
    aliases = tmp_path / "public-tool-links"
    aliases.mkdir()
    for name in ("bash", "ldd", "nix-store"):
        located = shutil.which(name)
        if located:
            (aliases / name).symlink_to(Path(located).resolve())
    retained = [directory for directory in os.get_exec_path()
                if not (Path(directory) / "bwrap").exists()]
    env = {
        **os.environ,
        "PATH": os.pathsep.join([str(hostile), str(aliases), *retained]),
        "BASH_ENV": str(startup), "ENV": str(startup),
        "XDG_DATA_HOME": str(tmp_path / "data"),
    }
    env.pop("MEDEN_BWRAP", None)
    completed = subprocess.run(
        [binary, "exec", "--workspace", str(source)],
        input=("test ! -e /run/current-system/sw && "
               "test ! -e /nix/var/nix/profiles/default && printf automatic-isolation-ok"),
        cwd=source, env=env, text=True, capture_output=True, timeout=30,
    )
    assert not marker.exists(), "untrusted helper or startup file executed on host"
    assert completed.returncode == 0, completed.stderr
    result = json.loads(completed.stdout)["result"]
    assert result["success"] and result["stdout"] == "automatic-isolation-ok", result
    (tmp_path / "bubblewrap-discovery-observation.json").write_text(json.dumps({
        "mode": mode, "meden_bwrap_configured": False,
        "untrusted_host_execution": marker.exists(), "isolated_execution_passed": True,
        "worker_sha256": hashlib.sha256(Path(binary).read_bytes()).hexdigest(),
    }))


@pytest.mark.parametrize(("name", "prefix"), [
    (name, prefix)
    for name in ["ldd", "nix-store", "bwrap", "bash", "rustup", "cargo"]
    for prefix in ["absolute", "relative"]
] + [("bwrap", "only")])
def test_workspace_helpers_cannot_run_before_isolation(tmp_path, name, prefix):
    binary = find_meden()
    assert binary
    source = tmp_path / "source"
    tools = source / "bin"
    tools.mkdir(parents=True)
    private = tmp_path / "host-only"
    private.write_bytes(b"HOST-PREPARATION-CANARY-" + os.urandom(32))
    marker = tmp_path / "host-execution"
    bwrap = os.environ.get("MEDEN_BWRAP") or shutil.which("bwrap")
    assert bwrap
    with socket.socket() as receiver:
        receiver.bind(("127.0.0.1", 0))
        receiver.listen()
        receiver.setblocking(False)
        port = receiver.getsockname()[1]
        script = (
            f"#!{sys.executable}\nimport pathlib, socket\n"
            + f"data=pathlib.Path({str(private)!r}).read_bytes()\n"
            + f"pathlib.Path({str(marker)!r}).write_bytes(data)\n"
            + f"with socket.create_connection(('127.0.0.1',{port}),timeout=2) as stream:\n"
            + "    stream.sendall(data)\nraise SystemExit(1)\n"
        )
        helper = tools / name
        helper.write_text(script)
        helper.chmod(0o700)
        # A paired untrusted rustup/cargo proxy must not hide installed tools.
        if name in ["rustup", "cargo"]:
            os.link(helper, tools / ("cargo" if name == "rustup" else "rustup"))
        env = {
            **os.environ,
            "PATH": os.pathsep.join([
                str(tools) if prefix == "absolute" else "bin",
                os.environ["PATH"], str(Path(bwrap).parent),
            ]),
            "XDG_DATA_HOME": str(tmp_path / "data"),
        }
        if prefix == "only":
            env["PATH"] = str(tools)
        if name == "bwrap":
            env.pop("MEDEN_BWRAP", None)

        def control():
            with socket.create_connection(("127.0.0.1", port), timeout=2) as client:
                client.sendall(b"public-control")
                client.shutdown(socket.SHUT_WR)
                stream, _ = receiver.accept()
                with stream, stream.makefile("rb") as body:
                    stream.settimeout(2)
                    assert body.read() == b"public-control"

        control()
        completed = subprocess.run(
            [str(Path(binary).resolve()), "exec", "--workspace", str(source)],
            input="cargo --version && rustc --version && printf 'public-tools-usable'",
            capture_output=True, text=True, env=env, cwd=source, timeout=30,
        )
        # Check actual host effects before checking the application's outcome:
        # an escaped helper must fail even if it also prevents startup.
        observed = []
        while True:
            try:
                stream, _ = receiver.accept()
            except BlockingIOError:
                break
            with stream, stream.makefile("rb") as body:
                stream.settimeout(2)
                observed.append(body.read().hex())
        observation = {
            "helper": name, "prefix": prefix, "host_executed": marker.exists(),
            "receiver_bodies_hex": observed, "returncode": completed.returncode,
            "worker_sha256": hashlib.sha256(Path(binary).read_bytes()).hexdigest(),
            "canary_sha256": hashlib.sha256(private.read_bytes()).hexdigest(),
            "public_controls": 1,
        }
        record = tmp_path / "preparation-observation.json"
        record.write_text(json.dumps(observation))
        assert not observed, "unapproved receiver obtained host data: " + str(observed)
        assert not marker.exists(), completed.stderr
        if prefix == "only":
            assert completed.returncode != 0 and not completed.stdout
        else:
            assert completed.returncode == 0, completed.stderr
            result = json.loads(completed.stdout)["result"]
            assert result["success"] and "public-tools-usable" in result["stdout"], result
        control()
        observation["public_controls"] = 2
        record.write_text(json.dumps(observation))
