"""Locate the Rust coding agent for integration and live-model checks."""

import os
import shutil
from pathlib import Path


def find_meden() -> str | None:
    """Locate the `meden` binary: `$MEDEN_BIN`, then PATH, then a local build."""
    env = os.environ.get("MEDEN_BIN")
    if env and Path(env).is_file():
        return env
    on_path = shutil.which("meden")
    if on_path:
        return on_path
    here = Path(__file__).resolve()
    for root in here.parents:
        for profile in ("debug", "release"):
            cand = root / "target" / profile / "meden"
            if cand.is_file():
                return str(cand)
    return None
