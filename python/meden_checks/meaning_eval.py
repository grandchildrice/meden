"""Retained end-to-end synthetic Lean meaning evaluation through public JSONL.

Gold labels/case IDs never enter model messages. Separate runs use new translations;
this is not a causal experiment isolating judge quality or a population benchmark.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import select
import subprocess
import time
from pathlib import Path


def metrics(rows):
    result = dict(
        total=len(rows),
        match_cases=0,
        mismatch_cases=0,
        correct=0,
        false_accept=0,
        false_reject=0,
        unclear=0,
        error=0,
    )
    for row in rows:
        expected, actual = row["expected"], row.get("actual", "error")
        result[f"{expected}_cases"] += 1
        if actual == "error":
            result["error"] += 1
        elif actual == "unclear":
            result["unclear"] += 1
        elif actual == expected:
            result["correct"] += 1
        elif actual == "match":
            result["false_accept"] += 1
        else:
            result["false_reject"] += 1
    result["false_accept_rate"] = (
        result["false_accept"] / result["mismatch_cases"] if result["mismatch_cases"] else None
    )
    result["false_reject_rate"] = (
        result["false_reject"] / result["match_cases"] if result["match_cases"] else None
    )
    result["coverage"] = (result["total"] - result["unclear"] - result["error"]) / max(1, len(rows))
    return result


class Host:
    def __init__(self, command, env, timeout):
        self.process = subprocess.Popen(
            command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env
        )
        self.timeout = timeout
        self.buffer = b""

    def send(self, value):
        self.process.stdin.write(json.dumps(value, ensure_ascii=False).encode() + b"\n")
        self.process.stdin.flush()

    def receive(self):
        deadline = time.monotonic() + self.timeout
        while b"\n" not in self.buffer:
            remaining = deadline - time.monotonic()
            if remaining <= 0 or not select.select([self.process.stdout], [], [], remaining)[0]:
                raise TimeoutError("Public JSONL command exceeded its evaluation budget")
            data = os.read(self.process.stdout.fileno(), 65536)
            if not data:
                raise RuntimeError(self.process.stderr.read().decode(errors="replace"))
            self.buffer += data
            if len(self.buffer) > 8_000_000:
                raise ValueError("Oversized JSONL response")
        line, self.buffer = self.buffer.split(b"\n", 1)
        return json.loads(line)

    def close(self):
        if self.process.poll() is None:
            self.process.kill()
        self.process.wait(timeout=5)
        for pipe in (self.process.stdin, self.process.stdout, self.process.stderr):
            try:
                pipe.close()
            except BrokenPipeError:
                pass


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--dataset", type=Path, default=Path("benchmarks/meaning/cases.json"))
    parser.add_argument("--provider", required=True)
    parser.add_argument("--model", required=True)
    parser.add_argument("--judge-model")
    parser.add_argument("--approve-processor", required=True)
    parser.add_argument("--lean", type=Path, required=True)
    parser.add_argument("--timeout", type=float, default=300)
    parser.add_argument("--offset", type=int, default=0)
    parser.add_argument("--limit", type=int)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.offset < 0 or args.limit is not None and args.limit <= 0:
        parser.error("--offset must be nonnegative and --limit positive")
    binary = args.binary.resolve()
    dataset_bytes = args.dataset.read_bytes()
    dataset = json.loads(dataset_bytes)
    cases = dataset["cases"][args.offset :]
    if args.limit is not None:
        cases = cases[: args.limit]
    if not cases:
        parser.error("selected dataset range is empty")
    args.out.mkdir(parents=True, exist_ok=False)
    env = {
        **os.environ,
        "XDG_DATA_HOME": str(args.out.resolve() / "state"),
        "MEDEN_ATLAS_LEAN": str(args.lean.resolve()),
    }
    setup = subprocess.run(
        [str(binary), "setup-atlas"], env=env, capture_output=True, timeout=args.timeout, text=True
    )
    (args.out / "setup.json").write_text(
        json.dumps(
            dict(exit_code=setup.returncode, stdout=setup.stdout, stderr=setup.stderr), indent=2
        )
    )
    rows = []
    report = dict(
        version=1,
        scope=dataset["scope"],
        label_method=dataset["label_method"],
        comparison_scope=(
            "Each configuration makes fresh translations; stochastic end-to-end "
            "comparison, not a paired judge-only experiment."
        ),
        binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
        dataset_sha256=hashlib.sha256(dataset_bytes).hexdigest(),
        provider=args.provider,
        translator=args.model,
        judge=args.judge_model or args.model,
        endpoint=args.approve_processor,
        prerequisites=(
            "Installed Lean; not cold install, local inference, native proof coverage "
            "or general accuracy"
        ),
        selection=dict(offset=args.offset, count=len(cases), dataset_count=len(dataset["cases"])),
        rows=rows,
    )
    for index, case in enumerate(cases, start=args.offset):
        start = time.monotonic()
        row = {**case, "index": index, "actual": "error"}
        host = None
        try:
            if setup.returncode:
                raise RuntimeError("Atlas preparation failed; see setup.json")
            project = args.out.resolve() / "projects" / str(index)
            project.mkdir(parents=True)
            (project / "Case.lean").write_text(case["source"])
            command = [
                str(binary),
                "run",
                "--workspace",
                str(project),
                "--provider",
                args.provider,
                "--model",
                args.model,
                "--approve-processor",
                args.approve_processor,
            ]
            if args.judge_model:
                command += [
                    "--judge-model",
                    args.judge_model,
                    "--approve-judge",
                    args.approve_processor,
                ]
            host = Host(command, env, args.timeout)
            host.send({"source_snapshot": {"paths": ["Case.lean"], "atlas": True}})
            ready = host.receive()
            assert ready["type"] == "ready", ready
            observation = host.receive()
            assert observation["type"] == "source_snapshot", observation
            snapshot = observation["snapshot"]
            row["source_snapshot"] = snapshot
            graph = snapshot["compilation"]["graph"]
            assert not any(node["hasSorry"] for node in graph["graph"]["nodes"]), (
                "Dataset proofs contain sorry"
            )
            target = next(
                node for node in graph["graph"]["nodes"] if node["id"] == case["target"]
            )
            # Only neutral source, target and requirement cross the public boundary.
            host.send(
                {
                    "analyze_meaning": dict(
                        source_revision=snapshot["revision"],
                        graph_revision=graph["revision"],
                        target=dict(
                            file="Case.lean", line=target["lineStart"], name=case["target"]
                        ),
                        requirement=case["requirement"],
                    )
                }
            )
            row["observations"] = []
            while True:
                event = host.receive()
                row["observations"].append(event)
                if event.get("type") == "analysis" and event.get("record"):
                    record = event["record"]
                    if record["state"] == "complete":
                        row["actual"] = record["assessment"]["verdict"]
                        row["record"] = record
                        break
                    if record["state"] in {"failed", "cancelled", "stale"}:
                        raise RuntimeError(record.get("error"))
        except Exception as error:
            row["error"] = f"{type(error).__name__}: {error}"
        finally:
            if host:
                host.close()
        row["seconds"] = round(time.monotonic() - start, 3)
        rows.append(row)
        (args.out / f"case-{index:02d}.json").write_text(
            json.dumps(row, ensure_ascii=False, indent=2)
        )
        report["metrics"] = metrics(rows)
        (args.out / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2))
        print(
            f"{index + 1}/{args.offset + len(cases)} {case['id']}: {row['actual']} "
            f"expected={case['expected']}",
            flush=True,
        )
    return int(report["metrics"]["error"] > 0)


if __name__ == "__main__":
    raise SystemExit(main())
