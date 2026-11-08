"""Record public pinned checker resources before starting a confidential session.

No network, credential discovery or candidate-provided configuration is used.
Run from a trusted checkout whose pinned Lean dependencies have been built.
"""

import argparse
import json
import os
import subprocess
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--rust", type=Path, required=True, help="Prepared nightly-2026-09-17 sysroot"
    )
    parser.add_argument("--charon", type=Path, required=True)
    parser.add_argument("--driver", type=Path, required=True)
    parser.add_argument("--aeneas", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent

    def lake(*argv):
        return subprocess.check_output(
            ["lake", "env", *argv], cwd=root / "lean", text=True
        ).strip()

    lean = Path(lake("lean", "--print-prefix")).resolve()
    libraries = []
    for directory in lake("printenv", "LEAN_PATH").split(os.pathsep):
        path = Path(directory).resolve()
        if path.is_dir() and path not in [
            lean / "lib/lean",
            root / "lean/.lake/build/lib/lean",
        ]:
            libraries.append(str(path))
    manifest = {
        "version": 1,
        "rust": str(args.rust.resolve(strict=True)),
        "lean": str(lean),
        "charon": str(args.charon.resolve(strict=True)),
        "driver": str(args.driver.resolve(strict=True)),
        "aeneas": str(args.aeneas.resolve(strict=True)),
        "libraries": libraries,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Prepared host checker manifest: {args.output}")


if __name__ == "__main__":
    main()
