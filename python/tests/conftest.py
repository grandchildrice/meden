"""Host fixtures for tests that exercise conversation, without a real checker."""

import json
import os

import pytest

pytest_plugins = ["meden_checks.ux"]


@pytest.fixture
def tui_verifier(tmp_path, monkeypatch):
    """A loadable inert manifest; these tests never request a formal proof.

    Real proof tests supply MEDEN_VERIFIER_BUNDLE themselves. Keeping this
    explicit prevents ordinary PTY fixtures from downloading gigabytes of tools.
    """
    if os.environ.get("MEDEN_VERIFIER_BUNDLE"):
        return os.environ["MEDEN_VERIFIER_BUNDLE"]
    root = tmp_path / "public-verifier-fixture"
    for name in ["rust", "lean"]:
        (root / name / "bin").mkdir(parents=True)
    paths = ["rust/bin/rustc", "rust/bin/cargo", "lean/bin/lean",
             "charon", "charon-driver", "aeneas"]
    for path in paths:
        (root / path).write_text("inert public-tool fixture; never executed")
    manifest = root / "bundle.json"
    manifest.write_text(json.dumps({
        "version": 1, "rust": str(root / "rust"), "lean": str(root / "lean"),
        "charon": str(root / "charon"), "driver": str(root / "charon-driver"),
        "aeneas": str(root / "aeneas"), "libraries": [],
    }))
    monkeypatch.setenv("MEDEN_VERIFIER_BUNDLE", str(manifest))
    return str(manifest)
