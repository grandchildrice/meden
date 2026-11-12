"""Verifier host configuration fails before a confidential working copy exists."""

import os
import subprocess

from meden_checks import find_meden


def test_invalid_explicit_verifier_stops_before_import(tmp_path):
    project = tmp_path / "project"
    project.mkdir()
    (project / "private.txt").write_text("project fixture must not enter setup")
    data = tmp_path / "data"
    invalid = tmp_path / "invalid-bundle.json"
    invalid.write_text('{"version": 999}')
    result = subprocess.run(
        [find_meden(), "tui", "--workspace", str(project), "--provider", "configured"],
        env={**os.environ, "XDG_DATA_HOME": str(data),
             "MEDEN_VERIFIER_BUNDLE": str(invalid),
             "MEDEN_LLM_BASE_URL": "http://127.0.0.1:1/v1", "MEDEN_LLM_MODEL": "fixture"},
        capture_output=True, text=True, timeout=15,
    )
    assert result.returncode != 0
    assert "explicitly selected verifier" in result.stderr
    assert not (data / "meden/workspaces").exists()
    assert not (data / "meden/verifier").exists(), "invalid override triggered installation"
    assert "project fixture must not enter setup" not in result.stderr
