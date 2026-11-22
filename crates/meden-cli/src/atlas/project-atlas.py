"""Prepare selected Lake modules and fingerprint their exact compiled inputs."""

import hashlib
import json
import pathlib
import stat
import subprocess
import sys

mode, encoded = sys.argv[1:]
bindings = json.loads(encoded)
root = pathlib.Path("/workspace")
lake = "/analysis/lean/bin/lake"


def command(args):
    result = subprocess.run(args, capture_output=True, check=False)
    if result.returncode:
        sys.stderr.buffer.write(result.stdout + result.stderr)
        raise RuntimeError("Prepared Lake compilation failed")
    return result.stdout


def regular(value):
    path = pathlib.Path(value)
    if not path.is_absolute():
        path = root / path
    canonical = path.resolve(strict=True)
    if canonical != path or not canonical.is_relative_to(root):
        raise RuntimeError(
            "Prepared compiler input leaves the workspace or uses symlinks"
        )
    if not stat.S_ISREG(path.lstat().st_mode):
        raise RuntimeError("Prepared compiler input is not a regular file")
    return path


if mode == "prepare":
    command([lake, "build", *["+" + b["name"] for b in bindings]])
paths = set()
setups = []
for binding in bindings:
    source = regular(binding["file"])
    resolved = (
        command([lake, "query", "+" + binding["name"] + ":lean"]).decode().strip()
    )
    if regular(resolved) != source:
        raise RuntimeError("Lake resolved a different source than the selected root")
    paths.add(source)
    setup = json.loads(command([lake, "setup-file", str(source)]))
    if setup["name"] != binding["name"] or setup.get("plugins") or setup.get("dynlibs"):
        raise RuntimeError("Selected module identity or unsupported native plugins")
    for artifact in setup.get("importArts", {}).values():
        for group in artifact:
            paths.update(regular(path) for path in group)
    compiled = regular(
        command([lake, "query", "+" + binding["name"] + ":olean"]).decode().strip()
    )
    if compiled.suffix != ".olean":
        raise RuntimeError("Lake resolved an invalid compiled root")
    base = compiled.with_suffix("")
    for suffix in [".olean", ".olean.private", ".olean.server", ".ir", ".ir.sig"]:
        path = pathlib.Path(str(base) + suffix)
        if suffix == ".olean" or path.exists():
            paths.add(regular(path))
    # Include all setup options, resolved paths and any additional future fields.
    paths_json = json.dumps(setup, sort_keys=True, separators=(",", ":")).encode()
    setups.append(paths_json)
if len(paths) > 1250000:
    raise RuntimeError("Prepared compiler inputs exceed 1250000 files")
digest = hashlib.sha256()
for setup in setups:
    digest.update(len(setup).to_bytes(8, "big"))
    digest.update(setup)
size = 0
for path in sorted(paths):
    name = str(path.relative_to(root)).encode()
    digest.update(len(name).to_bytes(8, "big"))
    digest.update(name)
    length = path.stat().st_size
    size += length
    if size > 512 * 1024**3:
        raise RuntimeError("Prepared compiler inputs exceed 512 GiB")
    digest.update(length.to_bytes(8, "big"))
    with path.open("rb") as source:
        while data := source.read(1024 * 1024):
            digest.update(data)
print(
    json.dumps({"fingerprint": digest.hexdigest(), "files": len(paths), "bytes": size})
)
