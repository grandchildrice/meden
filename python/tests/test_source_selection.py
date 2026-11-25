"""Actual frozen-layout selection and a deterministic worker-start budget."""

import hashlib
import json
import os
import subprocess
from pathlib import Path

import pytest
from test_lake_analysis import installed_lean
from test_tui_setup import environment

from meden_checks import find_meden


def selection_profile():
    if selected := os.environ.get("MEDEN_SOURCE_SELECTION_PROFILE"):
        return str(Path(selected).resolve())
    repo = Path(__file__).resolve().parents[2]
    built = subprocess.run(
        ["cargo", "build", "--offline", "-p", "meden-app", "--example", "profile-source-selection"],
        cwd=repo, text=True, capture_output=True, timeout=180,
    )
    assert built.returncode == 0, built.stdout + built.stderr
    metadata = subprocess.run(
        ["cargo", "metadata", "--offline", "--no-deps", "--format-version", "1"],
        cwd=repo, text=True, capture_output=True, timeout=30, check=True,
    )
    target = Path(json.loads(metadata.stdout)["target_directory"])
    return str(target / "debug/examples/profile-source-selection")


def sibling_project(tmp_path, count=10):
    root = tmp_path / "project"
    root.mkdir()
    original = {
        "Entry.lean": "theorem original : True := by trivial\n",
        "lakefile.toml": 'name = "frontier"\n[[lean_lib]]\nname = "Entry"\n',
    }
    packages = []
    for i in range(count):
        name = f"dependency{i}"
        directory = f"dependencies/{name}"
        original["lakefile.toml"] += f'[[require]]\nname = "{name}"\npath = "{directory}"\n'
        original[f"{directory}/lakefile.toml"] = f'name = "{name}"\n'
        packages.append(dict(type="path", name=name, dir=directory, inherited=False,
                             configFile="lakefile.toml"))
    original["lake-manifest.json"] = json.dumps(dict(
        version="1.4.0", name="frontier", lakeDir=".lake", packagesDir=".lake/packages",
        packages=packages,
    ))
    for name, text in original.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
    return root, original, packages


def test_layout_selection_keeps_complete_inputs_with_bounded_worker_starts(tmp_path):
    lean = installed_lean()
    root, original, packages = sibling_project(tmp_path)
    env = {**environment(tmp_path), "MEDEN_ATLAS_LEAN": str(lean)}
    result = subprocess.run(
        [selection_profile(), str(root), find_meden(), '["Entry.lean"]', "2"],
        env=env, capture_output=True, text=True, timeout=180,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    report = json.loads(result.stdout)
    first, second = report["samples"]
    files = {"Entry.lean": original["Entry.lean"]}
    encoded_files = json.dumps(files, ensure_ascii=False, separators=(",", ":")).encode()
    metadata = {name: text for name, text in original.items() if name != "Entry.lean"}
    metadata["lakefile.lean"] = None
    expected = dict(
        revision="sha256:" + hashlib.sha256(encoded_files).hexdigest(), files=files,
        collection=dict(
            roots=["Entry.lean"], external_modules=[],
            modules=[dict(name="Entry", file="Entry.lean", root="")],
            packages=[dict(name="frontier", directory="", source_roots=[""])] + [
                dict(name=p["name"], directory=p["dir"], source_roots=[p["dir"]])
                for p in packages
            ],
            metadata=dict(files=dict(sorted(metadata.items())), directories={}),
        ),
    )
    encoded = json.dumps(expected, ensure_ascii=False, separators=(",", ":")).encode()
    assert first["selection_sha256"] == "sha256:" + hashlib.sha256(encoded).hexdigest()
    assert first["selection_sha256"] == second["selection_sha256"]
    assert first["canonical_bytes"] == second["canonical_bytes"]
    assert first["configuration_bytes"] == sum(map(len, (
        text for name, text in original.items() if name != "Entry.lean"
    )))
    assert second["source_files"] == 1 and second["packages"] == 11
    assert second["configuration_observations"] == 13
    trace = next(r["context"]["trace"] for r in reversed(report["metrics"]["records"])
                 if r["name"] == "analysis.select_roots")
    records = [r for r in report["metrics"]["records"] if r["context"]["trace"] == trace]
    assert sum(r["name"] == "atlas.lake_configuration.reuse" for r in records) == 12
    starts = sum(r["name"] == "process.run" for r in records)
    assert starts <= 16, f"Source selection started {starts} workers for ten sibling packages"
    assert report["metrics"]["evicted"] == 0
    assert all((root / name).read_text() == text for name, text in original.items())
    assert not (root / ".lake").exists(), "Frozen configuration must not write to the project"
    (tmp_path / "source-selection.json").write_text(json.dumps(report, indent=2))


@pytest.mark.parametrize("fault", ["utf8", "file-limit", "total-limit"])
def test_unavailable_upcoming_configuration_never_publishes_partial_selection(tmp_path, fault):
    lean = installed_lean()
    root, original, _ = sibling_project(tmp_path, 17 if fault == "total-limit" else 10)
    if fault == "utf8":
        (root / "dependencies/dependency9/lakefile.toml").write_bytes(b"\xff\xfe")
    elif fault == "file-limit":
        (root / "dependencies/dependency9/lakefile.toml").write_bytes(b"#" + b"x" * 65_536)
    else:
        for i in range(17):
            path = root / f"dependencies/dependency{i}/lakefile.toml"
            prefix = path.read_bytes() + b"#"
            path.write_bytes(prefix + b"x" * (65_536 - len(prefix)))
    before = {name: (root / name).read_bytes() for name in original}
    result = subprocess.run(
        [selection_profile(), str(root), find_meden(), '["Entry.lean"]', "1"],
        env={**environment(tmp_path), "MEDEN_ATLAS_LEAN": str(lean)},
        capture_output=True, text=True, timeout=180,
    )
    assert result.returncode != 0, result.stdout + result.stderr
    assert not result.stdout.strip(), "No complete or partial source selection may be published"
    assert "Completed source-selection" not in result.stderr
    assert all((root / name).read_bytes() == contents for name, contents in before.items())
    assert not (root / ".lake").exists()
