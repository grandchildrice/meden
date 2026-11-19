"""Actual Lake configuration and source compilation through the public JSONL host."""

import json
import os
import subprocess
from pathlib import Path

import pytest

from meden_checks import find_meden
from meden_checks.lake_fixture import make_project


def installed_lean():
    lean = Path(
        os.environ.get(
            "MEDEN_ATLAS_LEAN", str(Path.home() / ".elan/toolchains/leanprover--lean4---v4.35.0")
        )
    )
    if not (lean / "bin/lean").is_file():
        pytest.skip("actual installed Lean/Lake is required")
    return lean


@pytest.mark.parametrize("source", ["git-cache", "registry-cache", "registry-path", "path-copy"])
def test_lake_srcdir_transitive_path_and_cached_git_keep_real_source_links(tmp_path, source):
    root = tmp_path / "project"
    original = make_project(root)
    if source != "git-cache":
        config = root / "lakefile.lean"
        old = next(
            line for line in config.read_text().splitlines() if line.startswith("require cached")
        )
        replacement = (
            'require cached from copy "vendor/cached-seed"'
            if source == "path-copy"
            else "require cached"
        )
        config.write_text(config.read_text().replace(old, replacement))
        if source in {"registry-path", "path-copy"}:
            path = root / "lake-manifest.json"
            manifest = json.loads(path.read_text())
            manifest["packages"][-1] = dict(
                type="path",
                name="cached",
                inherited=False,
                dir=(
                    "vendor/cached-seed" if source == "path-copy" else ".lake/packages/cached/sub"
                ),
                copy=source == "path-copy",
                configFile=(
                    "sub/config/lakefile.toml" if source == "path-copy" else "config/lakefile.toml"
                ),
            )
            path.write_text(json.dumps(manifest))
        if source == "path-copy":
            cached_config = root / ".lake/packages/cached/sub/config/lakefile.toml"
            cached_config.write_text(cached_config.read_text().replace('"code"', '"sub/code"'))
        original = {name: (root / name).read_text() for name in original}
    env = {
        **os.environ,
        "XDG_DATA_HOME": str(tmp_path / "data"),
        "MEDEN_ATLAS_LEAN": str(installed_lean()),
    }
    setup = subprocess.run(
        [find_meden(), "setup-atlas"], env=env, capture_output=True, text=True, timeout=180
    )
    assert setup.returncode == 0, setup.stdout + setup.stderr
    result = subprocess.run(
        [
            find_meden(),
            "run",
            "--workspace",
            str(root),
            "--base-url",
            "http://127.0.0.1:1/v1",
            "--model",
            "no-inference",
            "--approve-processor",
            "http://127.0.0.1:1/v1/chat/completions",
        ],
        input=json.dumps({"source_snapshot": {"paths": [], "atlas": True}}) + "\n",
        env=env,
        text=True,
        capture_output=True,
        timeout=180,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    snapshot = json.loads(result.stdout.splitlines()[-1])["snapshot"]
    assert snapshot["collection"]["roots"] == ["src/lib/Store.lean"]
    assert len(snapshot["files"]) == 4
    assert sorted(p["name"] for p in snapshot["collection"]["packages"]) == [
        "base",
        "cached",
        "logic",
        "shop",
    ]
    nodes = {n["id"]: n for n in snapshot["atlas"]["graph"]["nodes"]}
    assert nodes["Store.permitted"]["filePath"] == "src/lib/Store.lean"
    assert nodes["Domain.eligible"]["filePath"] == "vendor/logic/lean/Domain.lean"
    assert nodes["Base.threshold"]["filePath"] == "vendor/base/source/Base.lean"
    assert nodes["Extra.tag"]["filePath"] == ".lake/packages/cached/sub/code/Extra.lean"
    assert all(not n["hasSorry"] for n in nodes.values())
    assert all((root / name).read_text() == content for name, content in original.items())
    assert not (root / ".lake/config").exists(), "Lake state must stay in its private job"
    (tmp_path / "lake-observation.json").write_text(json.dumps(snapshot, indent=2))


@pytest.mark.parametrize("failure", ["missing-cache", "path-escape", "manifest-origin"])
def test_lake_refuses_unavailable_or_misattributed_dependencies(tmp_path, failure):
    root = tmp_path / "project"
    make_project(root)
    if failure == "missing-cache":
        (root / ".lake/packages/cached/sub/config/lakefile.toml").unlink()
    elif failure == "path-escape":
        config = root / "lakefile.lean"
        config.write_text(config.read_text().replace('"vendor/logic"', '"../outside"'))
    else:
        path = root / "lake-manifest.json"
        manifest = json.loads(path.read_text())
        manifest["packages"][-1]["url"] = "https://example.invalid/different.git"
        path.write_text(json.dumps(manifest))
    env = {
        **os.environ,
        "XDG_DATA_HOME": str(tmp_path / "data"),
        "MEDEN_ATLAS_LEAN": str(installed_lean()),
    }
    setup = subprocess.run(
        [find_meden(), "setup-atlas"], env=env, capture_output=True, text=True, timeout=180
    )
    assert setup.returncode == 0, setup.stdout + setup.stderr
    result = subprocess.run(
        [
            find_meden(),
            "run",
            "--workspace",
            str(root),
            "--base-url",
            "http://127.0.0.1:1/v1",
            "--model",
            "no-inference",
            "--approve-processor",
            "http://127.0.0.1:1/v1/chat/completions",
        ],
        input=json.dumps({"source_snapshot": {"paths": [], "atlas": True}}) + "\n",
        env=env,
        text=True,
        capture_output=True,
        timeout=180,
    )
    assert result.returncode != 0
    assert '"type":"source_snapshot"' not in result.stdout
    expected = {
        "missing-cache": "is not available",
        "path-escape": "leaves the selected workspace",
        "manifest-origin": "Manifest Git source differs",
    }
    assert expected[failure] in result.stderr, result.stderr
    assert not (root / ".lake/config").exists()


def test_lean_lake_configuration_stays_offline_and_cannot_observe_host_data(tmp_path):
    import threading
    from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

    hits = []

    class Sink(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_GET(self):
            hits.append(self.path)
            self.send_response(200)
            self.end_headers()

    server = ThreadingHTTPServer(("127.0.0.1", 0), Sink)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    root = tmp_path / "project"
    root.mkdir()
    (root / "host-private.txt").write_text("SYNTHETIC-HOST-CONTENT")
    (tmp_path / "outside-private.txt").write_text("SYNTHETIC-OUTSIDE-CONTENT")
    probe = f"""import os, socket
from pathlib import Path
assert os.getenv('MEDEN_LAKE_TEST_SECRET') is None
assert not Path('/workspace/host-private.txt').exists()
assert not Path('/analysis/sources/host-private.txt').exists()
assert not Path({str(tmp_path / "outside-private.txt")!r}).exists()
for file in ['/analysis/sources/lakefile.lean','/analysis/library/MedenLake.lean']:
 try: Path(file).write_text('tampered')
 except OSError: pass
 else: raise AssertionError('read-only analysis input was writable')
try: socket.create_connection(('127.0.0.1',{server.server_port}),timeout=.3)
except OSError: pass
else: raise AssertionError('host network was reachable')
Path('/workspace/private-job-write').write_text('private')
"""
    config = (
        "import Lake\nopen Lake DSL\n#eval do\n"
        '  let result ← IO.Process.output { cmd := "/tools/python3", args := #["-c", '
        + json.dumps(probe)
        + "] }\n"
        "  if result.exitCode != 0 then throw <| IO.userError result.stderr\n"
        "package isolated\nlean_lib App\n"
    )
    (root / "lakefile.lean").write_text(config)
    (root / "App.lean").write_text("def checked : Nat := 1\n")
    env = {
        **os.environ,
        "XDG_DATA_HOME": str(tmp_path / "data"),
        "MEDEN_ATLAS_LEAN": str(installed_lean()),
        "MEDEN_LAKE_TEST_SECRET": "SYNTHETIC-ENV-CONTENT",
    }
    try:
        setup = subprocess.run(
            [find_meden(), "setup-atlas"], env=env, capture_output=True, text=True, timeout=180
        )
        assert setup.returncode == 0, setup.stdout + setup.stderr
        result = subprocess.run(
            [
                find_meden(),
                "run",
                "--workspace",
                str(root),
                "--base-url",
                "http://127.0.0.1:1/v1",
                "--model",
                "no-inference",
                "--approve-processor",
                "http://127.0.0.1:1/v1/chat/completions",
            ],
            input=json.dumps({"source_snapshot": {"paths": [], "atlas": True}}) + "\n",
            env=env,
            text=True,
            capture_output=True,
            timeout=180,
        )
        assert result.returncode == 0, result.stdout + result.stderr
        assert not hits
        assert not (root / "private-job-write").exists()
        assert (root / "lakefile.lean").read_text() == config
        snapshot = json.loads(result.stdout.splitlines()[-1])["snapshot"]
        assert snapshot["files"] == {"App.lean": "def checked : Nat := 1\n"}
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
