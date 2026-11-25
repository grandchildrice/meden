"""Live public-TUI Lake discovery/meaning/failure/restart on synthetic source data."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import tempfile
import time
from pathlib import Path

from meden_checks.lake_fixture import make_project
from meden_checks.meaning_smoke import Terminal

REQUIREMENT = (
    "すべての自然数の残高と在庫について、残高100以上かつ在庫が正のとき、"
    "かつそのときだけ購入を許可する。"
)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--provider", required=True)
    parser.add_argument("--model", required=True)
    parser.add_argument("--judge-model", required=True)
    parser.add_argument("--lean", type=Path, required=True)
    parser.add_argument("--verifier-bundle", type=Path, required=True)
    parser.add_argument("--timeout", type=float, default=300)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.out.exists():
        parser.error("--out must be a new file")
    binary = args.binary.resolve()
    report = dict(
        success=False,
        binary=str(binary),
        binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
        provider=args.provider,
        translator=args.model,
        judge=args.judge_model,
        requirement=REQUIREMENT,
        screens=[],
        scope="Synthetic prepopulated Lake packages, actual installed Lean/Lake and approved "
        "cloud inference; no Git commit attestation, native proof, local inference "
        "or cold install.",
    )
    started = time.monotonic()
    terminal = None
    with tempfile.TemporaryDirectory(prefix="meden-lake-smoke-") as directory:
        root = Path(directory)
        project = root / "project"
        original = make_project(project)
        env = {
            **os.environ,
            "XDG_DATA_HOME": str(root / "data"),
            "TERM": "xterm-256color",
            "MEDEN_ATLAS_LEAN": str(args.lean.resolve()),
            "MEDEN_VERIFIER_BUNDLE": str(args.verifier_bundle.resolve()),
        }
        command = [
            str(binary),
            "tui",
            "--workspace",
            str(project),
            "--provider",
            args.provider,
            "--model",
            args.model,
            "--judge-model",
            args.judge_model,
        ]

        def milestone(name):
            report["screens"].append(dict(step=name, screen=terminal.view()))
            print(name, flush=True)

        def history():
            files = list((root / "data/meden/projects").glob("*/meaning-analysis.json"))
            return json.loads(files[0].read_text())["entries"] if files else []

        def atlas():
            terminal.command("/graph")
            terminal.wait(lambda view: "Reading sources / preparing local compiler" in view)
            terminal.wait(
                lambda view: (
                    "Source graph ready: 4 files" in view
                    and "Domain.eligible · vendor/logic/lean/Domain.lean:3" in view
                    and "Reading sources / preparing local compiler" not in view
                    and "shift+enter" in view
                )
            )

        try:
            terminal = Terminal(command, env, args.timeout)
            terminal.wait(
                lambda view: (
                    f"Judge model: {args.judge_model}" in view and "Allow for this session" in view
                )
            )
            milestone("both processor bindings reviewed")
            terminal.send("y")
            terminal.wait(lambda view: "shift+enter" in view)
            terminal.command("/lean-prepare")
            terminal.wait(lambda view: "Atlas is ready" in view and "shift+enter" in view)
            atlas()
            milestone("Lake roots and four cross-package source paths")
            terminal.command(f"/analyze Domain.eligible -- {REQUIREMENT}")
            terminal.wait(
                lambda view: (
                    "shift+enter" in view and ("Candidate " in view or "Analysis stopped" in view)
                )
            )
            report["history"] = history()
            record = report["history"][-1]
            if record["state"] != "complete" or record["assessment"]["verdict"] != "mismatch":
                raise RuntimeError(
                    "Live Lake analysis did not admit the expected mismatch candidate"
                )
            if record["judge"]["model"] != args.judge_model:
                raise RuntimeError("Unexpected judge binding")
            artifacts = list((root / "data/meden/projects").glob("*/analysis-sources/*.json"))
            report["source_artifacts"] = [json.loads(path.read_text()) for path in artifacts]
            milestone("candidate mismatch from transitive threshold definition")
            terminal.command("/graph missing.lean")
            terminal.wait(lambda view: "Analysis stopped" in view and "shift+enter" in view)
            terminal.command("/analyze Domain.eligible -- refuse an older graph")
            terminal.wait(lambda view: "Read the source graph first" in view)
            milestone("missing root refuses the older graph")
            atlas()
            milestone("Lake recovery through the public command")
            report["first_terminal_restored"] = terminal.quit()
            terminal.close()
            terminal = Terminal(command, env, args.timeout)
            terminal.wait(
                lambda view: (
                    f"Judge model: {args.judge_model}" in view and "Allow for this session" in view
                )
            )
            milestone("fresh processor review after restart")
            terminal.send("n")
            terminal.wait(lambda view: "Choose a model connection" in view)
            terminal.send("\x1b")
            terminal.wait(lambda view: "shift+enter" in view)
            terminal.command("/analysis")
            terminal.wait(lambda view: "Historical" in view and "shift+enter" in view)
            milestone("historical result without renewed approval")
            report["terminal_restored"] = terminal.quit()
            report["history_after_restart"] = history()
            report["unchanged_sources"] = all(
                (project / name).read_text() == content for name, content in original.items()
            )
            report["success"] = (
                report["unchanged_sources"]
                and report["first_terminal_restored"]
                and report["terminal_restored"]
                and len(report["history_after_restart"]) == 1
                and not (project / ".lake/config").exists()
            )
        except Exception as error:
            report["error"] = f"{type(error).__name__}: {error}"
            report["history"] = history()
            if terminal:
                milestone("failure")
        finally:
            if terminal:
                terminal.close()
    report["seconds"] = round(time.monotonic() - started, 3)
    args.out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return int(not report["success"])


if __name__ == "__main__":
    raise SystemExit(main())
