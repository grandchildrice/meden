"""Build and resolve Lake roots once, then fingerprint every exact compiled input."""

import hashlib
import json
import pathlib
import stat
import subprocess
import sys
import time
from contextlib import closing

# Lake prints one JSON result per requested facet, in request order. Bound each
# record before decoding; a protocol error or nonzero exit admits no fingerprint.
RESULT_BYTES = 64 * 1024**2


def query(lake, bindings):
    targets = [
        "+" + binding["name"] + facet
        for binding in bindings
        for facet in ["", ":lean", ":olean", ":setup"]
    ]
    args = [str(lake), "--no-ansi", "--json", "query", *targets]
    print("$ lake --json query " + " ".join(targets), file=sys.stderr, flush=True)
    with subprocess.Popen(
        args, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=sys.stderr
    ) as child:
        try:
            for binding in bindings:
                values = []
                for _ in range(4):
                    row = child.stdout.readline(RESULT_BYTES + 1)
                    if len(row) > RESULT_BYTES:
                        raise RuntimeError(
                            "Lake query returned an incomplete or oversized result"
                        )
                    if not row.endswith(b"\n"):
                        status = child.wait()
                        if status:
                            raise RuntimeError(
                                f"Prepared Lake compilation failed (exit {status})"
                            )
                        raise RuntimeError("Lake query returned an incomplete result")
                    values.append(json.loads(row))
                yield binding, *values[1:]
            if child.stdout.read(1):
                raise RuntimeError("Lake query returned unexpected additional output")
            if child.wait():
                raise RuntimeError("Prepared Lake compilation failed")
        finally:
            if child.poll() is None:
                child.kill()
            child.wait()


def fingerprint(root, lake, bindings, mode):
    if mode not in {"prepare", "fingerprint"} or not 1 <= len(bindings) <= 32:
        raise ValueError("Expected prepare/fingerprint and between 1 and 32 roots")

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

    started = time.monotonic()
    paths = set()
    digest = hashlib.sha256()
    # The anonymous target preserves `lake build +Module`. Lake's one shared
    # build state also resolves sources and the actual compiler setup;
    # no separate build, setup-file or per-root query repeats dependency work.
    with closing(query(lake, bindings)) as records:
        for binding, resolved, compiled, setup in records:
            source = regular(binding["file"])
            if regular(resolved) != source:
                raise RuntimeError(
                    "Lake resolved a different source than the selected root"
                )
            paths.add(source)
            if (
                setup["name"] != binding["name"]
                or setup.get("plugins")
                or setup.get("dynlibs")
            ):
                raise RuntimeError(
                    "Selected module identity or unsupported native plugins"
                )
            for artifact in setup.get("importArts", {}).values():
                for group in artifact:
                    paths.update(regular(path) for path in group)
            compiled = regular(compiled)
            if compiled.suffix != ".olean":
                raise RuntimeError("Lake resolved an invalid compiled root")
            base = compiled.with_suffix("")
            for suffix in [
                ".olean",
                ".olean.private",
                ".olean.server",
                ".ir",
                ".ir.sig",
            ]:
                path = pathlib.Path(str(base) + suffix)
                if suffix == ".olean" or path.exists():
                    paths.add(regular(path))
            # Include all actual compiler options, resolved paths and future fields.
            # Stream setups into the aggregate instead of retaining every root's copy.
            encoded = json.dumps(setup, sort_keys=True, separators=(",", ":")).encode()
            digest.update(len(encoded).to_bytes(8, "big"))
            digest.update(encoded)
    print(
        f"Build/setup and input resolution completed in {time.monotonic() - started:.3f}s.",
        file=sys.stderr,
        flush=True,
    )
    if len(paths) > 1250000:
        raise RuntimeError("Prepared compiler inputs exceed 1250000 files")
    print(f"Validating {len(paths)} compiled input files…", file=sys.stderr, flush=True)
    started = time.monotonic()
    size = 0
    for index, path in enumerate(sorted(paths)):
        if index and index % 5000 == 0:
            print(f"Validated {index}/{len(paths)} files", file=sys.stderr, flush=True)
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
        f"Validated {len(paths)} files / {size} bytes in {time.monotonic() - started:.3f}s.",
        file=sys.stderr,
        flush=True,
    )
    return {"fingerprint": digest.hexdigest(), "files": len(paths), "bytes": size}


if __name__ == "__main__":
    mode, encoded = sys.argv[1:]
    print(
        json.dumps(
            fingerprint(
                pathlib.Path("/workspace"),
                pathlib.Path("/analysis/lean/bin/lake"),
                json.loads(encoded),
                mode,
            )
        )
    )
