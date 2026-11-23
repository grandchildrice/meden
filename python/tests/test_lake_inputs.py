"""Worker-protocol fixtures; actual Lake builds are covered by GUI/TUI journeys."""

import hashlib
import importlib.util
import json
import os
import sys
import threading
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import pytest

SCRIPT = Path(__file__).parents[2] / "crates/meden-app/src/atlas/project-atlas.py"


@pytest.fixture
def worker():
    spec = importlib.util.spec_from_file_location("project_atlas_inputs", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


@pytest.fixture
def inputs(tmp_path):
    root = tmp_path / "workspace"
    root.mkdir()
    source = root / "A.lean"
    source.write_text("def a := 1\n")
    compiled = root / "output/A.olean"
    compiled.parent.mkdir()
    compiled.write_bytes(b"COMPILED_ROOT")
    private = root / "output/A.olean.private"
    private.write_bytes(b"PRIVATE_ROOT")
    imported = root / "imports/B.olean"
    imported.parent.mkdir()
    imported.write_bytes(b"COMPILED_IMPORT")
    setup = {
        "name": "A",
        "plugins": [],
        "dynlibs": [],
        "importArts": {"B": [[str(imported)]]},
        "options": {"maxRecDepth": 1000},
        "future": {"complete": True},
    }
    return root, [{"name": "A", "file": "A.lean"}], [str(source), str(compiled), setup]


def lake(tmp_path, payload, code=0):
    path, calls = tmp_path / "lake", tmp_path / "calls.jsonl"
    path.write_text(
        f"#!{sys.executable}\nimport json, os, sys\n"
        f"with open({str(calls)!r}, 'a') as output: output.write(json.dumps(sys.argv[1:])+'\\n')\n"
        f"os.write(1, {payload!r})\nsys.exit({code})\n"
    )
    path.chmod(0o700)
    return path, calls


def encode(values):
    rows = []
    for index, value in enumerate(values):
        if index % 3 == 0:
            rows.append(b"null\n")  # The anonymous build target has no query payload.
        rows.append(json.dumps(value).encode() + b"\n")
    return b"".join(rows)


def test_one_shared_query_builds_roots_and_hashes_all_bytes_and_setup(worker, inputs, tmp_path):
    root, bindings, rows = inputs
    # Two roots share one import: retain both original setup records but hash
    # each physical input path once, in deterministic order.
    (root / "C.lean").write_text("def c := 2\n")
    (root / "output/C.olean").write_bytes(b"ROOT_C")
    setup_c = {**rows[2], "name": "C"}
    rows.extend([str(root / "C.lean"), str(root / "output/C.olean"), setup_c])
    bindings.append({"name": "C", "file": "C.lean"})
    executable, calls = lake(tmp_path, encode(rows))
    result = worker.fingerprint(root, executable, bindings, "prepare")
    expected = hashlib.sha256(b"meden.prepared-inputs.v2\0" + (2).to_bytes(8, "big"))
    for setup in [rows[2], setup_c]:
        data = json.dumps(setup, sort_keys=True, separators=(",", ":")).encode()
        expected.update(len(data).to_bytes(8, "big"))
        expected.update(data)
    files = [
        root / name
        for name in [
            "A.lean",
            "C.lean",
            "imports/B.olean",
            "output/A.olean",
            "output/A.olean.private",
            "output/C.olean",
        ]
    ]
    expected.update(len(files).to_bytes(8, "big"))
    size = 0
    for path in sorted(files):
        name, data = str(path.relative_to(root)).encode(), path.read_bytes()
        expected.update(len(name).to_bytes(8, "big"))
        expected.update(name)
        expected.update(len(data).to_bytes(8, "big"))
        expected.update(hashlib.sha256(data).digest())
        size += len(data)
    assert result == {"fingerprint": expected.hexdigest(), "files": 6, "bytes": size}
    assert [json.loads(line) for line in calls.read_text().splitlines()] == [
        [
            "--no-ansi",
            "--json",
            "query",
            "+A",
            "+A:lean",
            "+A:olean",
            "+A:setup",
            "+C",
            "+C:lean",
            "+C:olean",
            "+C:setup",
        ]
    ]


def test_same_size_timestamp_restored_edit_and_setup_change_invalidate(worker, inputs, tmp_path):
    root, bindings, rows = inputs
    executable, _ = lake(tmp_path, encode(rows))
    original = worker.fingerprint(root, executable, bindings, "fingerprint")
    path = root / "imports/B.olean"
    before = path.stat()
    path.write_bytes(b"COMPILED_TAMPER")
    assert path.stat().st_size == before.st_size
    os.utime(path, ns=(before.st_atime_ns, before.st_mtime_ns))
    changed = worker.fingerprint(root, executable, bindings, "fingerprint")
    assert changed["fingerprint"] != original["fingerprint"]
    rows[2]["future"]["complete"] = False
    lake(tmp_path, encode(rows))
    assert (
        worker.fingerprint(root, executable, bindings, "fingerprint")["fingerprint"]
        != changed["fingerprint"]
    )


@pytest.mark.parametrize(
    "fault", ["truncated", "missing", "extra", "nonzero", "nonzero_empty", "oversized"]
)
def test_query_never_admits_partial_extra_failed_or_oversized_output(
    worker, inputs, tmp_path, fault
):
    root, bindings, rows = inputs
    payload = encode(rows)
    code = 0
    if fault == "truncated":
        payload = payload[:-1]
    elif fault == "missing":
        payload = encode(rows[:2])
    elif fault == "extra":
        payload += b"{}\n"
    elif fault in {"nonzero", "nonzero_empty"}:
        code = 17
        if fault == "nonzero_empty":
            payload = b""
    else:
        worker.RESULT_BYTES = 128
    executable, _ = lake(tmp_path, payload, code)
    with pytest.raises(
        RuntimeError, match="compilation failed" if fault.startswith("nonzero") else ".*"
    ):
        worker.fingerprint(root, executable, bindings, "prepare")


@pytest.mark.parametrize(
    "fault", ["source", "module", "plugin", "dynlib", "missing", "symlink", "external"]
)
def test_original_identity_and_complete_canonical_inputs_remain_required(
    worker, inputs, tmp_path, fault
):
    root, bindings, rows = inputs
    if fault == "source":
        rows[0] = str(root / "imports/B.olean")
    elif fault == "module":
        rows[2]["name"] = "Other"
    elif fault in {"plugin", "dynlib"}:
        rows[2]["plugins" if fault == "plugin" else "dynlibs"] = ["UNSUPPORTED"]
    elif fault == "missing":
        (root / "imports/B.olean").unlink()
    else:
        outside = tmp_path / "external.olean"
        outside.write_bytes(b"EXTERNAL")
        if fault == "symlink":
            (root / "imports/B.olean").unlink()
            (root / "imports/B.olean").symlink_to(outside)
        else:
            rows[2]["importArts"]["B"] = [[str(outside)]]
    executable, _ = lake(tmp_path, encode(rows))
    with pytest.raises((RuntimeError, FileNotFoundError)):
        worker.fingerprint(root, executable, bindings, "prepare")


def test_parallel_completion_and_batch_boundaries_preserve_exact_order(
    worker, tmp_path, monkeypatch
):
    # More than one bounded batch, Unicode names, empty files and a multi-chunk
    # file. Build the reference from original bytes, independently of worker I/O.
    originals = {}
    for index in range(260):
        path = tmp_path / f"input-{index:04d}-λ.olean"
        content = bytes([index % 256]) * (index * 257 % 8193)
        if index == 0:
            content = b"\0\xff" * (1024**2 + 17)
        path.write_bytes(content)
        originals[path] = content
    expected = hashlib.sha256(len(originals).to_bytes(8, "big"))
    for path, content in sorted(originals.items()):
        name = path.name.encode()
        expected.update(len(name).to_bytes(8, "big") + name)
        expected.update(len(content).to_bytes(8, "big") + hashlib.sha256(content).digest())

    def observe():
        digest = hashlib.sha256()
        size = worker.hash_inputs(tmp_path, reversed(originals), digest)
        assert size == sum(map(len, originals.values()))
        assert digest.digest() == expected.digest()

    monkeypatch.setattr(worker, "HASH_WORKERS", 1)
    observe()
    monkeypatch.setattr(worker, "HASH_WORKERS", 4)
    original_hash = worker.file_hash
    second_finished = threading.Event()
    first, second, *_ = sorted(originals)

    def reordered(record):
        if record[0] == first:
            assert second_finished.wait(5), "Independent reads did not run concurrently"
        result = original_hash(record)
        if record[0] == second:
            second_finished.set()
        return result

    monkeypatch.setattr(worker, "file_hash", reordered)
    observe()


@pytest.mark.parametrize("change", ["edit", "truncate", "grow"])
def test_concurrent_file_changes_are_refused_even_if_timestamp_is_restored(
    worker, tmp_path, monkeypatch, change
):
    path = tmp_path / "input.olean"
    path.write_bytes(b"A" * (2 * 1024**2 + 1))
    before = path.stat()
    started, resume = threading.Event(), threading.Event()
    original_hash = hashlib.sha256

    class HeldHash:
        def __init__(self):
            self.digestor = original_hash()

        def update(self, data):
            self.digestor.update(data)
            if not started.is_set():
                started.set()
                assert resume.wait(5)

        def digest(self):
            return self.digestor.digest()

    monkeypatch.setattr(worker.hashlib, "sha256", HeldHash)
    with ThreadPoolExecutor(max_workers=1) as pool:
        result = pool.submit(worker.file_hash, (path, before))
        try:
            assert started.wait(5)
            with path.open("r+b") as source:
                if change == "edit":
                    source.seek(1024**2)
                    source.write(b"B")
                elif change == "truncate":
                    source.truncate(1024**2)
                else:
                    source.seek(0, os.SEEK_END)
                    source.write(b"B")
            os.utime(path, ns=(before.st_atime_ns, before.st_mtime_ns))
        finally:
            resume.set()
        with pytest.raises(RuntimeError, match="during validation"):
            result.result(timeout=5)


@pytest.mark.parametrize("change", ["replace", "symlink", "directory"])
def test_changed_prepared_path_cannot_be_admitted(worker, tmp_path, change):
    path = tmp_path / "input.olean"
    path.write_bytes(b"INPUT")
    original = path.stat()
    if change == "replace":
        replacement = tmp_path / "replacement.olean"
        replacement.write_bytes(b"OTHER")
        os.utime(replacement, ns=(original.st_atime_ns, original.st_mtime_ns))
        replacement.replace(path)
    elif change == "symlink":
        path.unlink()
        target = tmp_path / "target.olean"
        target.write_bytes(b"INPUT")
        path.symlink_to(target)
    else:
        path.unlink()
        path.mkdir()
    with pytest.raises((OSError, RuntimeError)):
        worker.file_hash((path, original))


def test_byte_limit_is_checked_before_scheduling_reads(worker, inputs, tmp_path, monkeypatch):
    root, bindings, rows = inputs
    executable, _ = lake(tmp_path, encode(rows))
    monkeypatch.setattr(worker, "MAX_INPUT_BYTES", 1)
    monkeypatch.setattr(worker, "file_hash", lambda _: pytest.fail("Read past the byte budget"))
    with pytest.raises(RuntimeError, match="exceed 512 GiB"):
        worker.fingerprint(root, executable, bindings, "prepare")
