"""Synthetic, already-populated Lake workspace for actual installed-Lean tests."""

import json
from pathlib import Path

ROOT_CONFIG = """import Lake
open Lake DSL

def sources : System.FilePath := "src"
package shop where
  srcDir := sources
require logic from "vendor/logic"
require cached from git "https://example.invalid/never-fetch.git" @ "deadbeef" / "sub"
lean_lib Store where
  srcDir := "lib"
"""
APP = """import Domain
import Extra
namespace Store
theorem permitted (b s : Nat) : Domain.eligible b s ↔ (150 ≤ b ∧ 0 < s) := Iff.rfl
def reported : Nat := Extra.tag
end Store
"""


def make_project(root: Path):
    files = {
        "lakefile.lean": ROOT_CONFIG,
        "src/lib/Store.lean": APP,
        "vendor/logic/lakefile.toml": 'name = "logic"\nsrcDir = "lean"\n'
        '[[lean_lib]]\nname = "Domain"\n[[require]]\nname = "base"\npath = "../base"\n',
        "vendor/logic/lean/Domain.lean": "import Base\nnamespace Domain\n"
        "def eligible (b s : Nat) : Prop := Base.threshold ≤ b ∧ 0 < s\nend Domain\n",
        "vendor/base/lakefile.toml": 'name = "base"\nsrcDir = "source"\n'
        '[[lean_lib]]\nname = "Base"\n',
        "vendor/base/source/Base.lean": "namespace Base\ndef threshold : Nat := 150\nend Base\n",
        ".lake/packages/cached/sub/config/lakefile.toml": 'name = "cached"\nsrcDir = "code"\n'
        '[[lean_lib]]\nname = "Extra"\n',
        ".lake/packages/cached/sub/code/Extra.lean": (
            "namespace Extra\ndef tag : Nat := 7\nend Extra\n"
        ),
    }
    entries = [
        dict(
            type="path",
            name="logic",
            dir="vendor/logic",
            inherited=False,
            configFile="lakefile.toml",
        ),
        dict(
            type="path", name="base", dir="vendor/base", inherited=True, configFile="lakefile.toml"
        ),
        dict(
            type="git",
            name="cached",
            url="https://example.invalid/never-fetch.git",
            rev="deadbeef",
            inputRev="deadbeef",
            subDir="sub",
            inherited=False,
            configFile="config/lakefile.toml",
        ),
    ]
    files["lake-manifest.json"] = json.dumps(
        dict(
            version="1.4.0",
            name="shop",
            lakeDir=".lake",
            packagesDir=".lake/packages",
            packages=entries,
        )
    )
    for name, content in files.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
    return files
