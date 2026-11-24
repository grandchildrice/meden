"""Portable history and actual offline Lake checks through the public CLI."""

import hashlib
import json
import os
import shlex
import shutil
import signal
import subprocess
import time
from dataclasses import dataclass, field
from pathlib import Path

import pytest

from meden_checks import find_meden


@dataclass
class Project:
    root: Path
    env: dict[str, str] = field(repr=False)
    binary: str
    source: Path

    @property
    def history(self):
        return self.root / ".meden/workspace.json"

    def command(self, *arguments):
        return [self.binary, "work", "--workspace", str(self.root), *arguments]

    def run(self, *arguments, success=True):
        result = subprocess.run(
            self.command(*arguments), env=self.env, text=True, capture_output=True, timeout=120
        )
        if success:
            assert result.returncode == 0, result.stdout + result.stderr
        else:
            assert result.returncode != 0, result.stdout + result.stderr
        return result

    def document(self):
        return json.loads(self.history.read_text())

    def checks(self):
        return [event["check"] for event in self.document()["events"] if event["check"]]


@pytest.fixture
def project(tmp_path):
    binary = find_meden()
    if not binary:
        pytest.skip("a built Meden executable is required")
    candidates = [
        os.environ.get("MEDEN_PROJECT_LEAN"),
        os.environ.get("MEDEN_ATLAS_LEAN"),
        str(Path.home() / ".elan/toolchains/leanprover--lean4---v4.36.0"),
        str(Path.home() / ".elan/toolchains/leanprover--lean4---v4.35.0"),
    ]
    lean = next(
        (Path(path) for path in candidates if path and (Path(path) / "bin/lean").is_file()),
        None,
    )
    if lean is None:
        pytest.skip("actual installed Lean/Lake is required")
    root = tmp_path / "project"
    root.mkdir()
    version = lean.name.removeprefix("leanprover--lean4---")
    if not version.startswith("v"):
        version = "v4.35.0"
    (root / "lean-toolchain").write_text(f"leanprover/lean4:{version}\n")
    (root / "lakefile.toml").write_text('name = "portable_check"\n\n[[lean_lib]]\nname = "Proof"\n')
    source = root / "Proof.lean"
    source.write_text("import Lean\n\ntheorem portable_identity (n : Nat) : n = n := rfl\n")
    env = {
        **os.environ,
        "XDG_DATA_HOME": str(tmp_path / "data"),
        "MEDEN_PROJECT_LEAN": str(lean.resolve()),
    }
    item = Project(root, env, binary, source)
    item.run("init", "--name", "Portable actual Lean checks")
    item.run(
        "add",
        "--id",
        "proof",
        "--title",
        "Identity",
        "--requirement",
        "Every natural number equals itself.",
        "--module",
        "Proof",
        "--theorem",
        "portable_identity",
        "--next",
        "Inspect the retained theorem and continue.",
    )
    return item


def _wait_for_file(path, process, timeout=30):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if path.exists():
            return
        if process.poll() is not None:
            stdout, stderr = process.communicate()
            pytest.fail(f"check ended before the observed source step: {stdout}\n{stderr}")
        time.sleep(0.02)
    pytest.fail("actual Lean did not reach the source observation before the deadline")


def _slow_source():
    return (
        "import Lean\n\n#eval do\n"
        '  IO.FS.writeFile "/workspace/check-started" "started"\n'
        "  IO.sleep 1500\n"
        '  IO.FS.writeFile "/workspace/check-finished" "finished"\n\n'
        "theorem portable_identity (n : Nat) : n = n := rfl\n"
    )


def test_portable_clone_retains_real_proof_and_failure_history(project, tmp_path):
    project.run("note", "proof", "--text", "Check the unchanged requirement.")
    project.run("check", "proof")
    first = project.checks()[-1]
    assert first["success"] is True
    assert first["source_revision"] == first["after_revision"]
    assert first["theorems"] == ["portable_identity"]
    assert "portable_identity" in first["inspection"]
    project.source.write_text("import Lean\n\ntheorem portable_identity : False := by rfl\n")
    project.run("check", "proof", success=False)
    assert project.checks()[-1]["success"] is False
    assert project.checks()[-1]["stdout"] or project.checks()[-1]["stderr"]

    subprocess.run(["git", "init", "-q", str(project.root)], check=True)
    subprocess.run(
        [
            "git",
            "-C",
            str(project.root),
            "add",
            ".meden",
            "Proof.lean",
            "lakefile.toml",
            "lean-toolchain",
            "lake-manifest.json",
        ],
        check=True,
    )
    subprocess.run(
        [
            "git",
            "-C",
            str(project.root),
            "-c",
            "user.name=Meden test",
            "-c",
            "user.email=meden@example.invalid",
            "commit",
            "-qm",
            "Save portable work",
        ],
        check=True,
    )
    clone = tmp_path / "another-machine"
    subprocess.run(["git", "clone", "-q", str(project.root), str(clone)], check=True)
    moved = Project(
        clone,
        {**project.env, "XDG_DATA_HOME": str(tmp_path / "other-data")},
        project.binary,
        clone / "Proof.lean",
    )
    view = json.loads(moved.run("show").stdout)
    assert view["error"] is None
    assert view["document"] == project.document()
    assert not (clone / ".lake").exists()
    moved.source.write_text("import Lean\n\ntheorem portable_identity (n : Nat) : n = n := rfl\n")
    moved.run("check", "proof")
    records = moved.checks()
    assert [record["success"] for record in records] == [True, False, True]
    assert records[0] == first
    assert len({event["id"] for event in moved.document()["events"]}) == len(
        moved.document()["events"]
    )


@pytest.mark.parametrize(
    "kind", ["malformed", "invalid-utf8", "oversized", "authority", "task-authority"]
)
def test_portable_rejects_bad_records_without_rewriting_them(project, kind):
    document = project.document()
    if kind == "malformed":
        contents = b'{"tasks": ['
    elif kind == "invalid-utf8":
        contents = json.dumps(document).encode() + b"\xff"
    elif kind == "oversized":
        contents = b" " * (4_194_304 + 1)
    else:
        target = document if kind == "authority" else document["tasks"][0]
        target["approved"] = True
        contents = json.dumps(document).encode()
    project.history.write_bytes(contents)
    before = hashlib.sha256(contents).digest()
    view = json.loads(project.run("show").stdout)
    assert view["document"] is None
    assert view["error"]
    assert hashlib.sha256(project.history.read_bytes()).digest() == before
    project.run("note", "proof", "--text", "Must not overwrite invalid history", success=False)
    assert hashlib.sha256(project.history.read_bytes()).digest() == before


def test_complete_portable_history_at_the_byte_limit_preserves_literal_data_and_edits(
    project, tmp_path
):
    document = project.document()
    literal = 'Literal λ 🧪: \"quotes\" \\ path\n\t\0 {"Err":{"kind":"cancelled"}}\n' * 1000
    document["conversations"] = [
        {"id": "literal", "messages": [{"speaker": "user", "text": literal}]}
    ]
    encoded = json.dumps(document, ensure_ascii=False).encode()
    contents = encoded + b" " * (4_194_304 - len(encoded))
    assert len(contents) == 4_194_304
    project.history.write_bytes(contents)
    before = hashlib.sha256(contents).digest()
    # Observe the actual confined worker's unchanged reply. This trusted host
    # launcher copies bytes verbatim; it neither supplies nor edits a result.
    public = {}
    for name in ("bash", "cat"):
        alias = shutil.which(name)
        assert alias and str(Path(alias).resolve()).startswith(("/nix/store/", "/usr/", "/bin/"))
        public[name] = alias
    bwrap = project.env.get("MEDEN_BWRAP")
    if bwrap is None:
        bwrap = shutil.which("bwrap")
        assert bwrap and str(Path(bwrap).resolve()).startswith(("/nix/store/", "/usr/", "/bin/"))
    assert bwrap, "Actual Bubblewrap is required"
    replies = tmp_path / "worker-replies"
    replies.mkdir()
    launcher = tmp_path / "observed-bwrap"
    launcher.write_text(
        f"#!{public['bash']}\nset -eu\n"
        f"reply={shlex.quote(str(replies))}/\"$BASHPID\"\n"
        "worker_status=0\n"
        f"{shlex.quote(bwrap)} \"$@\" > \"$reply\" || worker_status=$?\n"
        f"{shlex.quote(public['cat'])} \"$reply\"\nexit \"$worker_status\"\n"
    )
    launcher.chmod(0o700)
    result = subprocess.run(
        project.command("show"),
        env={**project.env, "MEDEN_BWRAP": str(launcher)},
        text=True,
        capture_output=True,
        timeout=120,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    reply_bytes = sum(path.stat().st_size for path in replies.iterdir())
    assert 0 < reply_bytes <= 8_388_608, "Complete 4 MiB history exceeds its IPC budget"
    view = json.loads(result.stdout)
    assert view["error"] is None
    assert view["document"] == document
    assert hashlib.sha256(project.history.read_bytes()).digest() == before
    project.run("note", "proof", "--text", "APPEND-AFTER-COMPLETE-READ")
    after = project.document()
    assert after["conversations"] == document["conversations"]
    assert after["tasks"] == document["tasks"]
    assert after["events"][-1]["note"] == "APPEND-AFTER-COMPLETE-READ"
    assert json.loads(project.run("show").stdout)["document"] == after


@pytest.mark.parametrize("link", ["directory", "file"])
def test_portable_external_symlink_cannot_read_or_change_host_history(project, tmp_path, link):
    outside = tmp_path / "outside"
    outside.mkdir()
    contents = json.dumps({**project.document(), "name": "HOST-ONLY-CANARY"})
    secret = outside / "workspace.json"
    secret.write_text(contents)
    if link == "directory":
        shutil.rmtree(project.root / ".meden")
        (project.root / ".meden").symlink_to(outside, target_is_directory=True)
    else:
        project.history.unlink()
        project.history.symlink_to(secret)
    view = project.run("show")
    assert "HOST-ONLY-CANARY" not in view.stdout
    assert json.loads(view.stdout)["document"] is None
    project.run("init", "--name", "Cannot overwrite host history", success=False)
    assert secret.read_text() == contents


@pytest.mark.parametrize("field", ["module", "theorem"])
def test_portable_names_are_data_and_cannot_inject_commands(project, field):
    document = project.history.read_bytes()
    bad = "Proof;touch /workspace/injected"
    result = project.run(
        "add",
        "--id",
        "injection",
        "--title",
        "Refused",
        "--requirement",
        "Keep the boundary.",
        "--module",
        bad if field == "module" else "Proof",
        "--theorem",
        bad if field == "theorem" else "portable_identity",
        success=False,
    )
    assert "Invalid portable work item" in result.stderr
    assert not (project.root / "injected").exists()
    assert project.history.read_bytes() == document


@pytest.mark.parametrize("kind", ["missing", "definition", "axiom"])
def test_portable_inspects_the_named_theorem_and_its_axioms(project, kind):
    if kind == "definition":
        project.source.write_text("import Lean\n\ndef portable_identity : Nat := 1\n")
    elif kind == "axiom":
        project.source.write_text(
            "module\n\npublic import Lean\n\npublic axiom invented : False\n"
            "public theorem portable_identity : False := invented\n"
        )
    else:
        project.source.write_text("import Lean\n\ntheorem unrelated : True := True.intro\n")
    project.run("check", "proof", success=False)
    record = project.checks()[-1]
    assert record["success"] is False
    assert record["inspection"]
    expected = {"definition": "Not a theorem", "axiom": "Unexpected axiom", "missing": "Missing"}
    assert expected[kind] in record["inspection"]


@pytest.mark.parametrize("kind", ["theorem", "definition", "axiom", "unsound-theorem"])
def test_portable_inspects_original_kind_and_axioms_of_module_imports(project, kind):
    (project.root / "lakefile.toml").write_text(
        'name = "portable_check"\n\n[[lean_lib]]\nname = "Proof"\n'
        '\n[[lean_lib]]\nname = "Imported"\n'
    )
    (project.root / "Imported.lean").write_text(
        "module\n\npublic import Lean\n\n"
        "set_option linter.defProp false\n\n"
        "public theorem imported_identity (n : Nat) : n = n := rfl\n"
        "public def imported_definition : True := True.intro\n"
        "public axiom imported_axiom : False\n"
        "public theorem imported_unsound : False := imported_axiom\n"
    )
    project.source.write_text(
        "module\n\npublic import Imported\n\n"
        "public theorem portable_identity (n : Nat) : n = n := imported_identity n\n"
    )
    declaration = {
        "theorem": "imported_identity",
        "definition": "imported_definition",
        "axiom": "imported_axiom",
        "unsound-theorem": "imported_unsound",
    }[kind]
    project.run(
        "add",
        "--id",
        "imported",
        "--title",
        "Imported declaration",
        "--requirement",
        "Inspect the selected imported declaration and transitive axioms.",
        "--module",
        "Proof",
        "--theorem",
        declaration,
        "--next",
        "Read the retained inspection.",
    )
    project.run("check", "imported", success=kind == "theorem")
    record = project.checks()[-1]
    assert record["success"] is (kind == "theorem")
    assert declaration in record["inspection"]
    if kind == "theorem":
        assert "MEDEN_THEOREMS_CHECKED" in record["inspection"]
    elif kind == "unsound-theorem":
        assert "Unexpected axiom" in record["inspection"]
        assert "imported_axiom" in record["inspection"]
    else:
        assert "Not a theorem" in record["inspection"]


@pytest.mark.parametrize("directory", ["build", "cache", "target"])
def test_portable_source_fingerprint_includes_lake_source_directories(project, directory):
    sources = project.root / directory
    sources.mkdir()
    project.source.rename(sources / "Proof.lean")
    project.source = sources / "Proof.lean"
    (project.root / "lakefile.toml").write_text(
        f'name = "portable_check"\nsrcDir = "{directory}"\n\n[[lean_lib]]\nname = "Proof"\n'
    )
    project.run("check", "proof")
    first = project.checks()[-1]
    project.source.write_text(
        "import Lean\n\ntheorem portable_identity (n : Nat) : n = n := Eq.refl n\n"
    )
    project.run("check", "proof")
    second = project.checks()[-1]
    assert first["success"] and second["success"]
    assert first["source_revision"] != second["source_revision"]


def test_portable_source_excluded_from_fingerprint_cannot_be_inspected(project):
    sources = project.root / ".git"
    sources.mkdir()
    project.source.rename(sources / "Proof.lean")
    (project.root / "lakefile.toml").write_text(
        'name = "portable_check"\nsrcDir = ".git"\n\n[[lean_lib]]\nname = "Proof"\n'
    )
    project.run("check", "proof", success=False)
    record = project.checks()[-1]
    assert record["success"] is False
    assert "MEDEN_THEOREMS_CHECKED" not in record["inspection"]


def test_portable_source_change_during_check_is_retained_as_failed(project):
    project.source.write_text(_slow_source())
    process = subprocess.Popen(
        project.command("check", "proof"),
        env=project.env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    try:
        _wait_for_file(project.root / "check-started", process)
        project.source.write_text(
            "import Lean\n\ntheorem portable_identity (n : Nat) : n = n := Eq.refl n\n"
        )
        stdout, stderr = process.communicate(timeout=60)
        assert process.returncode != 0, stdout + stderr
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=5)
    record = project.checks()[-1]
    assert record["success"] is False
    assert record["source_revision"] != record["after_revision"]
    assert record["stdout"] or record["stderr"] or record["inspection"]
    project.run("check", "proof")
    assert project.checks()[-1]["success"] is True


def test_portable_interruption_leaves_unfinished_history_and_stops_worker(project):
    project.source.write_text(_slow_source())
    process = subprocess.Popen(
        project.command("check", "proof"),
        env=project.env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    try:
        _wait_for_file(project.root / "check-started", process)
        process.send_signal(signal.SIGTERM)
        process.communicate(timeout=15)
        assert process.returncode != 0
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=5)
    time.sleep(1.8)
    assert not (project.root / "check-finished").exists()
    assert not any(record["success"] for record in project.checks())
    assert any("Started offline" in event["note"] for event in project.document()["events"])
    project.run("note", "proof", "--text", "Resume after the interrupted check.")
    assert project.document()["events"][-1]["note"] == "Resume after the interrupted check."


def test_portable_zero_exit_cannot_reuse_a_stale_compiled_theorem(project):
    project.run("check", "proof")
    assert project.checks()[-1]["success"] is True
    project.source.write_text(
        "import Lean\n\n#eval do\n"
        '  IO.FS.writeFile "/workspace/compiler-exited" "before zero exit"\n'
        "  (IO.Process.exit 0 : IO Unit)\n\n"
        "theorem portable_identity : False := by rfl\n"
    )
    result = subprocess.run(
        project.command("check", "proof"),
        env=project.env,
        text=True,
        capture_output=True,
        timeout=120,
    )
    assert (project.root / "compiler-exited").is_file(), result.stdout + result.stderr
    assert result.returncode != 0, result.stdout + result.stderr
    record = project.checks()[-1]
    assert record["success"] is False
    assert record["source_revision"] != project.checks()[0]["source_revision"]


def test_portable_restored_artifact_cannot_replace_current_source_inspection(project):
    project.run("check", "proof")
    assert project.checks()[-1]["success"] is True
    saved = project.root / "saved-artifacts"
    saved.mkdir()
    restoration = []
    for directory in ["lib/lean", "ir"]:
        for artifact in sorted((project.root / ".lake/build" / directory).glob("Proof.*")):
            if artifact.is_file():
                relative = artifact.relative_to(project.root)
                backup = saved / "_".join(relative.parts)
                shutil.copyfile(artifact, backup)
                restoration.append(
                    f'  IO.FS.writeBinFile "/workspace/{relative}" '
                    f'(← IO.FS.readBinFile "/workspace/{backup.relative_to(project.root)}")\n'
                )
    assert restoration
    project.source.write_text(
        "import Lean\n\n#eval do\n"
        + "".join(restoration)
        + '  IO.FS.writeFile "/workspace/artifacts-restored" "restored old theorem"\n'
        + "  (IO.Process.exit 0 : IO Unit)\n\n"
        + "theorem portable_identity : False := by rfl\n"
    )
    project.run("check", "proof", success=False)
    assert (project.root / "artifacts-restored").is_file()
    record = project.checks()[-1]
    assert record["success"] is False
    assert "MEDEN_THEOREMS_CHECKED" not in record["inspection"]
