"""Exercise real mixed system/Nix layouts in an outer Linux mount namespace."""

import hashlib
import json
import os
import shlex
import shutil
import socket
import subprocess
from pathlib import Path

import pytest

from meden_checks import find_meden

ROOT = Path(__file__).resolve().parents[2]


def test_prepared_checker_keeps_its_own_absolute_rust_library_paths(tmp_path):
    """Probe the real offline profile; fixture tools deliberately admit no proof."""
    binary = find_meden()
    assert binary
    public = tmp_path / "prepared"
    rust, lean = public / "rust", public / "lean"
    for root in [rust, lean]:
        (root / "bin").mkdir(parents=True)
        (root / "lib").mkdir()
    secret = public / "host-secret"
    secret.write_text("private adjacent host data")
    resource = rust / "lib" / "prepared-resource"
    resource.write_text("original public resource")

    def executable(path, script):
        path.write_text("#!/tools/bash\nset -eu\n" + script + "\n")
        path.chmod(0o755)

    executable(rust / "bin/rustc", "printf 'rustc 1.100.0-nightly (923c95cdf probe)\\n'")
    executable(lean / "bin/lean", "printf 'Lean (version 4.35.0, profile probe)\\n'")
    executable(public / "charon", "printf '8cae8fd1905946de61147a65ecdd7122cc6b491b\\n'")
    executable(public / "charon-driver", "exit 69")
    executable(public / "aeneas", "printf 'c5eb4a2\\n'")
    # An installed wrapper may refer to its own sysroot through an absolute
    # host path. Only the prepared executables/lib may appear at that path.
    executable(rust / "bin/cargo", f"""
test -x {shlex.quote(str(rust / 'bin/rustc'))}
test "$(cat {shlex.quote(str(resource))})" = 'original public resource'
test ! -e {shlex.quote(str(secret))}
if (printf changed > {shlex.quote(str(resource))}) 2>/dev/null; then exit 91; fi
printf 'meden-checker-origin-probe-passed\\n'
exit 69
""")
    manifest = public / "bundle.json"
    manifest.write_text(json.dumps({
        "version": 1, "rust": str(rust), "lean": str(lean),
        "charon": str(public / "charon"), "driver": str(public / "charon-driver"),
        "aeneas": str(public / "aeneas"), "libraries": [],
    }))
    output = tmp_path / "artifacts"
    completed = subprocess.run([
        binary, "verify-kernel", "--contract", ROOT / "verification/examples/counter.contract.json",
        "--source", ROOT / "verification/examples/counter.rs", "--bundle", manifest,
        "--output", output,
    ], capture_output=True, text=True, timeout=60)
    assert completed.returncode != 0 and not output.exists()
    assert "meden-check: tests" in completed.stderr, completed.stderr
    assert "meden-checker-origin-probe-passed" in completed.stderr, completed.stderr
    assert resource.read_text() == "original public resource"


def nix_tool(name):
    for directory in os.environ["PATH"].split(os.pathsep):
        candidate = (Path(directory) / name).resolve()
        if candidate.is_file() and candidate.is_relative_to("/nix/store"):
            return candidate
    return None


@pytest.fixture(params=["bash", "env"])
def mixed_profile(tmp_path, request):
    bash, envbin, ldd = (nix_tool(name) for name in ["bash", "env", "ldd"])
    if not all([bash, envbin, ldd]):
        pytest.skip("mixed-layout namespace lab requires installed public Nix tools")
    binary = find_meden()
    assert binary
    binary = str(Path(binary).resolve())
    bwrap = os.environ.get("MEDEN_BWRAP") or shutil.which("bwrap")
    assert bwrap
    libraries = subprocess.run([str(ldd), binary], capture_output=True, text=True, timeout=10)
    assert libraries.returncode == 0, libraries.stderr
    loaders = sorted({Path(word).resolve() for word in libraries.stdout.split()
                      if word.endswith("/ld-linux-x86-64.so.2")})
    if not loaders:
        pytest.skip("mixed-layout lab requires a dynamic x86_64 glibc worker")
    assert len(loaders) == 1 and loaders[0].is_relative_to("/nix/store")
    source = tmp_path / "source"
    source.mkdir()
    (source / "note").write_text("original")
    private = tmp_path / "host-private"
    private.write_bytes(os.urandom(32))
    public = tmp_path / "system-resource"
    public.write_text("public-system-resource")
    prefix = [str(bwrap), "--die-with-parent", "--unshare-user", "--unshare-pid",
              "--ro-bind", "/", "/", "--proc", "/proc", "--dev", "/dev",
              "--tmpfs", "/usr", "--dir", "/usr/bin", "--dir", "/usr/share",
              "--ro-bind", str(envbin), "/usr/bin/env",
              "--ro-bind", str(public), "/usr/share/meden-public",
              "--tmpfs", "/bin", "--tmpfs", "/lib64",
              "--ro-bind", str(loaders[0]), "/lib64/ld-linux-x86-64.so.2"]
    if request.param == "bash":
        prefix += ["--ro-bind", str(bash), "/usr/bin/bash"]
    prefix += ["--symlink", "/usr/bin/bash" if request.param == "bash" else str(bash),
               "/bin/bash", "--symlink", "/bin/bash", "/bin/sh",
               "--symlink", "/usr/bin/env", "/bin/env",
               "--bind", str(tmp_path), str(tmp_path), "--chdir", str(source), "--"]
    env = {**os.environ, "PATH": "/usr/bin:" + os.environ["PATH"],
           "XDG_DATA_HOME": str(tmp_path / "data"), "TMPDIR": str(tmp_path),
           "MEDEN_BWRAP": str(bwrap), "HOST_PRIVATE": "private-environment"}
    control = subprocess.run(
        prefix + ["/bin/env", "/bin/bash", "--noprofile", "--norc", "-c",
                  "printf system-bash-and-env-work"],
        env=env, capture_output=True, text=True, timeout=10,
    )
    assert control.returncode == 0 and control.stdout == "system-bash-and-env-work", control.stderr
    return {"prefix": prefix, "env": env, "binary": binary, "source": source,
            "private": private, "public": public, "system_tool": request.param,
            "system_loader": str(loaders[0]), "directory": tmp_path,
            "control_stdout": control.stdout}


def run_profile(profile, args, *, input=None, timeout=40):
    return subprocess.run(
        profile["prefix"] + [profile["binary"], *map(str, args)],
        input=input, env=profile["env"], capture_output=True, text=True, timeout=timeout,
    )


def test_mixed_system_and_nix_tools_keep_private_offline_workspaces(mixed_profile):
    profile = mixed_profile
    with socket.socket() as receiver:
        receiver.bind(("127.0.0.1", 0))
        receiver.listen()
        receiver.setblocking(False)
        port = receiver.getsockname()[1]
        script = f"""set -e
git init -q local-repo
cargo --version
rustc --version
printf private-copy > note
python3 - <<'PY'
import json, os, pathlib, socket
assert not pathlib.Path({str(profile['private'])!r}).exists()
assert not pathlib.Path({str(profile['source'])!r}).exists()
assert 'HOST_PRIVATE' not in os.environ
public=pathlib.Path('/usr/share/meden-public')
assert public.read_text()=='public-system-resource'
try: public.write_text('changed')
except OSError: pass
else: raise AssertionError('system resource was writable')
tool={profile['system_tool']!r}
assert pathlib.Path('/tools/'+tool).resolve().as_posix()=='/usr/bin/'+tool
assert pathlib.Path('/tools/git').resolve().is_relative_to('/nix/store')
try: stream=socket.create_connection(('127.0.0.1',{port}),timeout=.2)
except OSError: pass
else:
    stream.sendall(b'unapproved-hybrid-egress')
    stream.close()
    raise AssertionError('mixed profile escaped its network namespace')
print(json.dumps({{'net':os.readlink('/proc/self/ns/net'),'system_tool':{profile['system_tool']!r}}}))
PY
"""
        completed = run_profile(profile, ["exec", "--workspace", profile["source"]], input=script)
        record = {
            "worker_sha256": hashlib.sha256(Path(profile["binary"]).read_bytes()).hexdigest(),
            "system_tool": profile["system_tool"], "system_loader": profile["system_loader"],
            "control_stdout": profile["control_stdout"], "returncode": completed.returncode,
            "stdout": completed.stdout, "stderr": completed.stderr,
        }
        observation = profile["directory"] / "mixed-observation.json"
        observation.write_text(json.dumps(record, indent=2))
        assert completed.returncode == 0, completed.stderr
        result = json.loads(completed.stdout)
        assert result["result"]["success"], result
        marker = json.loads(result["result"]["stdout"].splitlines()[-1])
        assert marker["net"] != os.readlink("/proc/self/ns/net")
        assert (Path(result["workspace"]) / "note").read_text() == "private-copy"
        assert (profile["source"] / "note").read_text() == "original"
        assert profile["public"].read_text() == "public-system-resource"
        with pytest.raises(BlockingIOError):
            receiver.accept()
        with socket.create_connection(("127.0.0.1", port), timeout=2) as control:
            control.sendall(b'public-control')
            control.shutdown(socket.SHUT_WR)
            stream, _ = receiver.accept()
            with stream, stream.makefile("rb") as body:
                stream.settimeout(2)
                assert body.read() == b'public-control'
        record.update(marker=marker, result=result, receiver_control=True)
        observation.write_text(json.dumps(record, indent=2))


@pytest.mark.skipif(
    not os.environ.get("MEDEN_VERIFIER_BUNDLE"), reason="requires pinned checker bundle",
)
@pytest.mark.parametrize("fault", [False, True], ids=["proved", "hidden-fault"])
def test_mixed_system_and_nix_tools_check_actual_kernel(mixed_profile, fault):
    from test_project_verification import assert_artifact

    profile = mixed_profile
    directory = profile["directory"]
    contract = json.loads((ROOT / "verification/examples/counter.contract.json").read_text())
    candidate = (ROOT / "verification/examples/counter.rs").read_text()
    if fault:
        candidate = candidate.replace("    match", "    if amount == 4 { return s; }\n    match")
    source, agreement = directory / "candidate.rs", directory / "contract.json"
    output = directory / "artifacts"
    source.write_text(candidate)
    agreement.write_text(json.dumps(contract))
    completed = run_profile(profile, [
        "verify-kernel", "--contract", agreement, "--source", source,
        "--bundle", os.environ["MEDEN_VERIFIER_BUNDLE"], "--output", output,
    ], timeout=360)
    (directory / "mixed-checker-stderr.txt").write_text(completed.stderr)
    (directory / "mixed-checker-observation.json").write_text(json.dumps({
        "worker_sha256": hashlib.sha256(Path(profile["binary"]).read_bytes()).hexdigest(),
        "system_tool": profile["system_tool"], "fault": fault,
        "returncode": completed.returncode, "stdout": completed.stdout,
        "stderr": completed.stderr,
    }, indent=2))
    if fault:
        assert completed.returncode != 0 and not output.exists(), completed.stdout
        assert "test result: ok" in completed.stderr
        assert "meden-check: refinement" in completed.stderr
        assert "Check.lean" in completed.stderr and "error:" in completed.stderr
        for operational in ["unknown identifier", "unknown module", "unknown constant",
                            "stack overflow", "maximum number of heartbeats",
                            "maximum recursion depth", "failed to create thread"]:
            assert operational not in completed.stderr.casefold(), completed.stderr
    else:
        assert completed.returncode == 0, completed.stderr
        artifact = Path(json.loads(completed.stdout)["directory"])
        assert artifact.parent == output
        report = assert_artifact(artifact, candidate, contract)
        digest = hashlib.sha256(Path(profile["binary"]).read_bytes()).hexdigest()
        assert report["worker_sha256"] == digest
