"""Validate an admitted Lake query and fingerprint every exact compiled input."""

import hashlib
import json
import os
import pathlib
import stat
import sys
import time
from concurrent.futures import ThreadPoolExecutor

# Lake prints one JSON result per requested facet, in request order. Bound each
# record before decoding; a protocol error or nonzero exit admits no fingerprint.
RESULT_BYTES = 64 * 1024**2
MAX_INPUT_BYTES = 512 * 1024**3
HASH_WORKERS = 4
HASH_BATCH = 256
FINGERPRINT_DOMAIN = b"meden.prepared-inputs.v2\0"


def stamp(status):
    # Metadata detects ordinary concurrent replacement/mutation; it never
    # substitutes for reading and hashing every byte or enters the identity.
    return (
        status.st_dev,
        status.st_ino,
        status.st_size,
        status.st_mtime_ns,
        status.st_ctime_ns,
    )


def file_hash(record):
    path, expected = record
    flags = os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK
    with os.fdopen(os.open(path, flags), "rb") as source:
        before = os.fstat(source.fileno())
        if not stat.S_ISREG(before.st_mode) or stamp(before) != stamp(expected):
            raise RuntimeError("Prepared compiler input changed before validation")
        digest = hashlib.sha256()
        size = 0
        while data := source.read(1024 * 1024):
            size += len(data)
            if size > before.st_size:
                raise RuntimeError("Prepared compiler input grew during validation")
            digest.update(data)
        if size != before.st_size or stamp(os.fstat(source.fileno())) != stamp(before):
            raise RuntimeError("Prepared compiler input changed during validation")
    return path, size, digest.digest()


def hash_inputs(root, paths, digest):
    ordered = sorted(paths)
    digest.update(len(ordered).to_bytes(8, "big"))
    size = 0
    # map preserves input order. Bound queued jobs too: Python 3.11 map would
    # otherwise eagerly submit up to 1,250,000 futures. Only four readers and
    # their 1 MiB chunks run at once; results retain just one SHA-256 per file.
    with ThreadPoolExecutor(max_workers=HASH_WORKERS) as pool:
        for start in range(0, len(ordered), HASH_BATCH):
            records = []
            for path in ordered[start : start + HASH_BATCH]:
                status = path.lstat()
                if not stat.S_ISREG(status.st_mode):
                    raise RuntimeError("Prepared compiler input is not a regular file")
                size += status.st_size
                if size > MAX_INPUT_BYTES:
                    raise RuntimeError("Prepared compiler inputs exceed 512 GiB")
                records.append((path, status))
            for index, (path, length, content) in enumerate(
                pool.map(file_hash, records), start
            ):
                if index and index % 5000 == 0:
                    print(
                        f"Validated {index}/{len(ordered)} files",
                        file=sys.stderr,
                        flush=True,
                    )
                name = str(path.relative_to(root)).encode()
                digest.update(len(name).to_bytes(8, "big"))
                digest.update(name)
                digest.update(length.to_bytes(8, "big"))
                digest.update(content)
    return size


def query(stream, bindings):
    # The host admits complete zero-exit output before dispatching this worker.
    # No project code or subprocess runs in the metadata environment.
    for binding in bindings:
        values = []
        for _ in range(4):
            row = stream.readline(RESULT_BYTES + 1)
            if len(row) > RESULT_BYTES or not row.endswith(b"\n"):
                raise RuntimeError(
                    "Lake query returned an incomplete or oversized result"
                )
            values.append(json.loads(row))
        yield binding, *values[1:]
    if stream.read(1):
        raise RuntimeError("Lake query returned unexpected additional output")


def regular(root, value):
    path = pathlib.Path(value)
    if not path.is_absolute():
        path = root / path
    if not path.is_relative_to(root):
        raise RuntimeError(
            "Prepared compiler input leaves the workspace or uses symlinks"
        )
    # The Linux worker already mounts procfs. Ask the kernel for the opened
    # object's actual name instead of re-lstat'ing every ancestor in Python
    # for every imported artifact. O_PATH opens no content, including FIFOs;
    # O_NOFOLLOW plus fstat rejects a final symlink. Parent aliases change the
    # actual name and are rejected too, even if their target was already seen.
    # The handle is closed here; hash_inputs still performs its original full
    # content read and before/after checks. This is not a pinned snapshot.
    handle = os.open(path, os.O_PATH | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        if os.readlink(f"/proc/self/fd/{handle}") != str(path):
            raise RuntimeError(
                "Prepared compiler input leaves the workspace or uses symlinks"
            )
        if not stat.S_ISREG(os.fstat(handle).st_mode):
            raise RuntimeError("Prepared compiler input is not a regular file")
    finally:
        os.close(handle)
    return path


def fingerprint(root, stream, bindings, mode):
    if mode not in {"prepare", "fingerprint"} or not 1 <= len(bindings) <= 32:
        raise ValueError("Expected prepare/fingerprint and between 1 and 32 roots")

    started = time.monotonic()
    paths = set()
    digest = hashlib.sha256(FINGERPRINT_DOMAIN + len(bindings).to_bytes(8, "big"))
    for binding, resolved, compiled, setup in query(stream, bindings):
        source = regular(root, binding["file"])
        if regular(root, resolved) != source:
            raise RuntimeError(
                "Lake resolved a different source than the selected root"
            )
        paths.add(source)
        if (
            setup["name"] != binding["name"]
            or setup.get("plugins")
            or setup.get("dynlibs")
        ):
            raise RuntimeError("Selected module identity or unsupported native plugins")
        for artifact in setup.get("importArts", {}).values():
            for group in artifact:
                paths.update(regular(root, path) for path in group)
        compiled = regular(root, compiled)
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
                paths.add(regular(root, path))
        # Include all actual compiler options, resolved paths and future fields.
        # Stream setups into the aggregate instead of retaining every root's copy.
        encoded = json.dumps(setup, sort_keys=True, separators=(",", ":")).encode()
        digest.update(len(encoded).to_bytes(8, "big"))
        digest.update(encoded)
    print(
        f"Query decoding and input resolution completed in {time.monotonic() - started:.3f}s.",
        file=sys.stderr,
        flush=True,
    )
    if len(paths) > 1250000:
        raise RuntimeError("Prepared compiler inputs exceed 1250000 files")
    print(f"Validating {len(paths)} compiled input files…", file=sys.stderr, flush=True)
    started = time.monotonic()
    size = hash_inputs(root, paths, digest)
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
                sys.stdin.buffer,
                json.loads(encoded),
                mode,
            )
        )
    )
