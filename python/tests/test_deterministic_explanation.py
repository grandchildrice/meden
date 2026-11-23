"""Actual Lean/Plausible compiler failure retains meaning and permits the next node."""

import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest


def test_compiler_budget_failure_retains_english_and_next_observation(tmp_path):
    lean = Path(
        os.environ.get(
            "MEDEN_ATLAS_LEAN",
            str(Path.home() / ".elan/toolchains/leanprover--lean4---v4.35.0"),
        )
    )
    if not (lean / "bin/lean").is_file():
        pytest.skip("actual installed Lean is required")
    source = Path(__file__).resolve().parents[2] / "crates/meden-app/src/atlas"
    env = {**os.environ, "LEAN_SYSROOT": str(lean), "LEAN_PATH": str(tmp_path)}

    def run(*args):
        result = subprocess.run(
            [str(lean / "bin/lean"), *args],
            cwd=tmp_path,
            env=env,
            capture_output=True,
            text=True,
            timeout=120,
        )
        assert result.returncode == 0, result.stdout + result.stderr
        return result.stdout

    modules = [
        f"MedenPlausible/{name}"
        for name in (
            "Random",
            "Gen",
            "Shrinkable",
            "Arbitrary",
            "Sampleable",
            "Testable",
        )
    ] + ["MedenSemantic", "MedenSearch"]
    for module in modules:
        path = tmp_path / f"{module}.lean"
        path.parent.mkdir(exist_ok=True)
        original = (
            source / ("vendor/" if module.startswith("MedenPlausible/") else "") / f"{module}.lean"
        )
        shutil.copyfile(original, path)
        run("-o", f"{module}.olean", f"{module}.lean")
    (tmp_path / "Fixture.lean").write_text("axiom broken : ∀ n : Nat, n + 1 < n\n")
    run("-o", "Fixture.olean", "Fixture.lean")
    (tmp_path / "Run.lean").write_text("""import MedenSemantic
import MedenSearch
open Lean
unsafe def main : IO Unit := do
  Lean.initSearchPath (← Lean.findSysroot)
  Lean.enableInitializersExecution
  let imports := #[{module := `Fixture}, {module := `MedenSemantic}, {module := `MedenSearch}]
  let env ← Lean.importModules imports {}
    (trustLevel := 0) (loadExts := true)
  for options in [({} : Options).set `maxHeartbeats (1 : Nat), {}] do
    let context : PPContext := {env, opts := options}
    let ci := (env.find? `broken).get!
    let checker ← context.runMetaM <| MedenSearch.analyze ci #[`Fixture]
    let result ← context.runMetaM <| MedenSemantic.describe ci checker
    IO.println result.compress
""")
    failed, next_node = map(json.loads, run("--run", "Run.lean").splitlines())
    assert failed["checker"]["status"] in {"unavailable", "budget_exhausted"}
    assert failed["checker"]["candidates"] == []
    assert failed["sentence"] == next_node["sentence"]
    assert "for every n" in failed["sentence"]
    assert next_node["checker"]["status"] == "counterexample"
    assert "n := 0" in next_node["checker"]["candidates"]
