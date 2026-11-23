"""Live approved-model TUI trial with real offline finite verification.

Uses synthetic counter data, exact host-reviewed contract comparison and a
separate exhaustive Python oracle. Installed Z3 is a prerequisite, not a cold
installation claim. No configuration or candidate source is repaired off screen.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import tempfile
import time
import traceback
from pathlib import Path

from meden_checks.meaning_smoke import Terminal


def counter_value(term, state, amount):
    """Interpret only the public finite syntax; expected behavior is separate."""
    op = term["op"]
    if op == "constant":
        return term["value"]
    if op == "state" and term["field"] == "value":
        return state
    if op == "parameter" and term["name"] == "amount":
        return amount
    if op in ("add", "sub"):
        left = counter_value(term["left"], state, amount)
        right = counter_value(term["right"], state, amount)
        return (left + right if op == "add" else left - right) % 16
    if op == "if":
        key = "then" if condition(term["condition"], state, amount) else "else"
        return counter_value(term[key], state, amount)
    raise ValueError(f"Unsupported counter expression: {term}")


def condition(term, state, amount):
    op = term["op"]
    if op in ("true", "false"):
        return op == "true"
    if op == "not":
        return not condition(term["argument"], state, amount)
    if op in ("and", "or"):
        left = condition(term["left"], state, amount)
        right = condition(term["right"], state, amount)
        return left and right if op == "and" else left or right
    left = counter_value(term["left"], state, amount)
    right = counter_value(term["right"], state, amount)
    return {"equal": left == right, "less": left < right, "less_equal": left <= right}[op]


def independent_check(source):
    model = json.loads(source)
    assert model["version"] == 1 and set(model["transition"]) == {"value"}
    for state in range(16):
        for amount in range(16):
            expected = state + amount if state + amount < 16 else state
            actual = counter_value(model["transition"]["value"], state, amount)
            assert actual == expected, (state, amount, actual, expected)
    return 256


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--provider", required=True)
    parser.add_argument("--model", required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--timeout", type=float, default=300)
    args = parser.parse_args(argv)
    if args.out.exists():
        parser.error("--out must be a new file")
    examples = Path(__file__).resolve().parents[2] / "verification/examples"
    original = (examples / "finite-counter.contract.json").read_text()
    expected_contract = json.loads(original)
    binary = args.binary.resolve()
    report = dict(
        success=False,
        binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
        provider=args.provider,
        model=args.model,
        screens=[],
        scope=(
            "Live cloud inference, synthetic counter, installed offline Z3 and 256 independent "
            "Python cases. No local inference, cold installation or unbounded/Rust proof claim."
        ),
    )
    terminal = None
    started = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="meden-live-finite-") as directory:
        root = Path(directory)
        project = root / "project"
        project.mkdir()
        (project / "requirements.contract.json").write_text(original)
        source = project / expected_contract["source_path"]
        source.write_text((examples / "finite-counter-broken.model.json").read_text())
        env = {**os.environ, "XDG_DATA_HOME": str(root / "data"), "TERM": "xterm-256color"}
        env.pop("MEDEN_VERIFIER_BUNDLE", None)
        command = [
            str(binary),
            "tui",
            "--workspace",
            str(project),
            "--verifier",
            "z3.finite.v1",
            "--provider",
            args.provider,
            "--model",
            args.model,
        ]

        def milestone(name):
            report["screens"].append(dict(step=name, screen=terminal.view()))
            print(name, flush=True)

        def journals():
            return list((root / "data/meden/projects").glob("*/contracts.json"))

        try:
            terminal = Terminal(command, env, args.timeout)
            terminal.wait(
                lambda view: "Connection details" in view and "Allow for this session" in view
            )
            milestone("processor reviewed")
            terminal.send("y")
            terminal.wait(lambda view: "shift+enter" in view)
            prompt = (
                "Read requirements.contract.json and counter.model.json. Propose the EXACT "
                "original contract data without modifying it. "
                "Pause for my review. After acceptance, run "
                "verify_project FIRST to observe the existing overflow bug, then repair only "
                "counter.model.json and use verify_project again. Preserve every original "
                "property, field, argument, four-bit bound, and the requirements file. Check all "
                "256 inputs: return value+amount when it fits in 0..15, otherwise retain value. "
                "This is a finite one-step JSON model, not Rust or an unbounded claim."
            )
            report["prompt"] = prompt
            terminal.command(prompt)
            report["proposals"] = []
            for attempt in range(2):
                terminal.wait(
                    lambda view: (
                        "y: Accept specification" in view
                        or ("Stopped:" in view and "shift+enter" in view)
                    )
                )
                if "y: Accept specification" not in terminal.view():
                    raise RuntimeError(terminal.view())
                journal = json.loads(journals()[0].read_text())
                proposed = next(
                    event["contract"]
                    for event in reversed(journal["events"])
                    if event["op"] == "propose"
                )
                report["proposals"].append(proposed)
                if proposed == expected_contract:
                    break
                milestone("changed contract declined without approval")
                assert attempt == 0, "Repeated change to the original contract; no host acceptance"
                terminal.send("n")
                terminal.wait(lambda view: "shift+enter" in view)
                terminal.command(
                    "I rejected that proposal because it changed the original contract data. "
                    "Read requirements.contract.json again and propose its EXACT values, including "
                    "all metadata and every expression. Do not edit the requirements file or "
                    "substitute another specification. Pause for a new review."
                )
            assert proposed == expected_contract, (
                "The model changed the original contract; no host acceptance"
            )
            assert "256 assignments" in terminal.view()
            milestone("exact finite contract reviewed")
            terminal.send("y")
            terminal.wait(
                lambda view: (
                    "shift+enter" in view
                    and ("CHECKED · complete within finite bounds" in view or "Stopped:" in view)
                )
            )
            assert "CHECKED · complete within finite bounds" in terminal.view(), terminal.view()
            milestone("real bounded check completed")
            report["independent_cases"] = independent_check(source.read_text())
            assert (project / "requirements.contract.json").read_text() == original, (
                "The original requirement file changed"
            )
            artifacts = list((root / "data/meden/projects").glob("*/artifacts/checked-*"))
            assert artifacts, "No retained checker artifact"
            # The controller/model can legitimately complete more than one check.
            # Every retained successful result must bind the same correct source.
            report["artifact_count"] = len(artifacts)
            checked_reports = []
            for artifact in artifacts:
                checked = json.loads((artifact / "report.json").read_text())
                assert checked["class"] == "bounded" and checked["contract"] == expected_contract
                assert (
                    checked["registration"]["id"] == "z3.finite.v1"
                    and checked["registration"]["role"] == "verifier"
                )
                assert checked["source_sha256"] == hashlib.sha256(source.read_bytes()).hexdigest()
                assert (artifact / "source.json").read_bytes() == source.read_bytes()
                for name, digest in checked["files"].items():
                    assert hashlib.sha256((artifact / name).read_bytes()).hexdigest() == digest
                checked_reports.append(checked)
            history = list((root / "data/meden/projects").glob("*/conversation.json"))
            report["history"] = [json.loads(path.read_text()) for path in history]
            # Read the transcript, not just final model prose, for the real refusal.
            assert "Finite property failed" in json.dumps(report["history"]), (
                "The initial real counterexample was not observed"
            )
            report["source"] = source.read_text()
            report["reports"] = checked_reports
            retained = json.loads(journals()[0].read_text())
            assert retained["selected"] == "z3.finite.v1"
            binding = next(
                event["binding"]
                for event in reversed(retained["events"])
                if event["op"] == "propose"
            )
            assert all(binding["method"] == checked["registration"] for checked in checked_reports)
            report["original_verifier_binding"] = binding
            report["terminal_restored"] = terminal.quit()
            terminal.close()
            selector = command.index("--verifier")
            command = command[:selector] + command[selector + 2 :]
            terminal = Terminal(command, env, args.timeout)
            terminal.wait(
                lambda view: "Connection details" in view and "Allow for this session" in view
            )
            assert "CHECKED · complete within finite bounds" not in terminal.view()
            milestone("restart requires fresh review and check")
            report["restart_without_verifier_flag"] = True
            report["success"] = report["terminal_restored"]
        except Exception as error:
            report["error"] = f"{type(error).__name__}: {error}"
            report["traceback"] = traceback.format_exc()
            report["source_at_failure"] = source.read_text() if source.exists() else None
            report["requirements_at_failure"] = (project / "requirements.contract.json").read_text()
            report["artifact_reports_at_failure"] = [
                json.loads(path.read_text())
                for path in (root / "data/meden/projects").glob("*/artifacts/checked-*/report.json")
            ]
            if terminal:
                milestone("failure retained")
        finally:
            if terminal:
                terminal.close()
            report["elapsed_seconds"] = time.monotonic() - started
            args.out.parent.mkdir(parents=True, exist_ok=True)
            args.out.write_text(json.dumps(report, indent=2) + "\n")
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
