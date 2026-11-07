"""Running an AgentDojo suite against the capability-typed runtime.

Reports the three numbers that matter and keeps them separate:

* **utility** — tasks completed with no injection present. What the runtime
  costs in capability.
* **utility under attack** — tasks completed with an injection present.
* **attack success rate** — injections that achieved what they set out to do.

It also reports *why* calls were refused, broken down by rule, because
"refused by the integrity rule" and "refused because the policy forbade the
capability" are different claims and only the first is novel.

```bash
cd python
uv run python -m tagent_agentdojo.benchmark --suite banking --limit 3
```
"""

from __future__ import annotations

import argparse
import json
import shutil
import sys
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

from agentdojo.attacks.attack_registry import load_attack
from agentdojo.base_tasks import BaseInjectionTask, BaseUserTask
from agentdojo.task_suite.load_suites import get_suite

from .declarations import SUITES, UndeclaredTool
from .pipeline import TAgentPipeline


@dataclass
class Totals:
    """What a run came to."""

    suite: str
    provider: str
    #: Tasks run with no injection.
    benign_runs: int = 0
    benign_successes: int = 0
    #: Task-and-injection pairs run.
    attack_runs: int = 0
    #: Pairs where the user's task still succeeded.
    attack_utility: int = 0
    #: Pairs where the injection achieved its goal.
    attack_successes: int = 0
    #: Refusals by rule, across every run.
    refusals: dict[str, int] = field(default_factory=dict)
    #: Runs that ended without a usable plan, which is a cost, not a defence.
    no_plan: int = 0
    seconds: float = 0.0

    @property
    def utility(self) -> float:
        return self.benign_successes / self.benign_runs if self.benign_runs else 0.0

    @property
    def utility_under_attack(self) -> float:
        return self.attack_utility / self.attack_runs if self.attack_runs else 0.0

    @property
    def attack_success_rate(self) -> float:
        return self.attack_successes / self.attack_runs if self.attack_runs else 0.0

    def report(self) -> str:
        lines = [
            f"suite                 {self.suite}",
            f"planner               {self.provider}",
            f"utility               {self.utility:.0%}"
            f"  ({self.benign_successes}/{self.benign_runs})",
        ]
        if self.attack_runs:
            lines += [
                f"utility under attack  {self.utility_under_attack:.0%}"
                f"  ({self.attack_utility}/{self.attack_runs})",
                f"attack success rate   {self.attack_success_rate:.0%}"
                f"  ({self.attack_successes}/{self.attack_runs})",
            ]
        if self.refusals:
            lines.append("refusals by rule")
            for rule, n in sorted(self.refusals.items(), key=lambda kv: -kv[1]):
                lines.append(f"  {rule:<28} {n}")
        if self.no_plan:
            lines.append(f"runs with no usable plan   {self.no_plan}")
        lines.append(f"took                  {self.seconds:.0f}s")
        return "\n".join(lines)


def _tally(totals: Totals, report: Any) -> None:
    for tool, rule in report.refusals:
        key = f"{rule} ({tool})" if rule == "policy" else rule
        totals.refusals[key] = totals.refusals.get(key, 0) + 1
    if report.outcome == "no_plan":
        totals.no_plan += 1


def run_benign(
    suite: Any, pipeline: TAgentPipeline, tasks: list[BaseUserTask], totals: Totals
) -> None:
    """Every task, with nothing injected."""
    for task in tasks:
        try:
            utility, _ = suite.run_task_with_pipeline(
                pipeline, task, injection_task=None, injections={}
            )
        except UndeclaredTool:
            raise
        except Exception as e:  # noqa: BLE001 - one bad task must not end the run
            print(f"  {task.ID}: error: {e}", file=sys.stderr)
            utility = False
        totals.benign_runs += 1
        totals.benign_successes += int(utility)
        _tally(totals, pipeline.last_report)
        print(f"  {task.ID:<12} utility={'yes' if utility else 'no '}", flush=True)


def run_attacks(
    suite: Any,
    pipeline: TAgentPipeline,
    tasks: list[BaseUserTask],
    injections: list[BaseInjectionTask],
    attack_name: str,
    totals: Totals,
) -> None:
    """Every task against every injection."""
    attack = load_attack(attack_name, suite, pipeline)
    for task in tasks:
        for injection_task in injections:
            placed = attack.attack(task, injection_task)
            try:
                utility, security = suite.run_task_with_pipeline(
                    pipeline, task, injection_task=injection_task, injections=placed
                )
            except UndeclaredTool:
                raise
            except Exception as e:  # noqa: BLE001
                print(f"  {task.ID}/{injection_task.ID}: error: {e}", file=sys.stderr)
                utility, security = False, False
            totals.attack_runs += 1
            totals.attack_utility += int(utility)
            totals.attack_successes += int(security)
            _tally(totals, pipeline.last_report)
            print(
                f"  {task.ID:<12} {injection_task.ID:<14} "
                f"utility={'yes' if utility else 'no '} "
                f"attack={'SUCCEEDED' if security else 'failed'}",
                flush=True,
            )


def find_binary() -> str:
    """The tagent binary: an explicit path, a build in this checkout, or PATH."""
    import os

    explicit = os.environ.get("TAGENT_BIN")
    if explicit and Path(explicit).is_file():
        return explicit
    here = Path(__file__).resolve()
    for root in here.parents:
        for profile in ("release", "debug"):
            candidate = root / "target" / profile / "tagent"
            if candidate.is_file():
                return str(candidate)
    found = shutil.which("tagent")
    if found:
        return found
    raise SystemExit("no tagent binary; run `cargo build --release -p tagent-cli`")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--suite", default="banking", choices=sorted(SUITES))
    parser.add_argument("--provider", default=None, help="see `tagent providers`")
    parser.add_argument("--model", default=None)
    parser.add_argument("--attack", default="important_instructions")
    parser.add_argument("--limit", type=int, default=None, help="use only the first N user tasks")
    parser.add_argument(
        "--injection-limit", type=int, default=None, help="only the first N injections"
    )
    parser.add_argument("--benign-only", action="store_true", help="measure utility and stop")
    parser.add_argument("--max-attempts", type=int, default=3)
    parser.add_argument("--out", type=Path, default=None, help="write totals as JSON")
    args = parser.parse_args(argv)

    suite = get_suite("v1", args.suite)
    declarations = SUITES[args.suite]
    pipeline = TAgentPipeline(
        declarations=declarations,
        binary=find_binary(),
        provider=args.provider,
        model=args.model,
        max_attempts=args.max_attempts,
    )

    tasks = list(suite.user_tasks.values())[: args.limit]
    injections = list(suite.injection_tasks.values())[: args.injection_limit]
    totals = Totals(suite=args.suite, provider=args.provider or "first detected")

    started = time.monotonic()
    print(f"{len(tasks)} user tasks, no injection")
    run_benign(suite, pipeline, tasks, totals)
    if not args.benign_only:
        print(f"{len(tasks)} tasks x {len(injections)} injections")
        run_attacks(suite, pipeline, tasks, injections, args.attack, totals)
    totals.seconds = time.monotonic() - started

    print()
    print(totals.report())
    if args.out:
        payload = asdict(totals) | {
            "utility": totals.utility,
            "utility_under_attack": totals.utility_under_attack,
            "attack_success_rate": totals.attack_success_rate,
        }
        args.out.write_text(json.dumps(payload, indent=2) + "\n")
        print(f"\nwritten to {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
