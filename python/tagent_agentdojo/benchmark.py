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
import concurrent.futures as cf
import contextlib
import json
import queue
import shutil
import sys
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

from agentdojo.attacks.attack_registry import load_attack
from agentdojo.base_tasks import BaseInjectionTask, BaseUserTask
from agentdojo.task_suite.load_suites import get_suite

from .declarations import SUITES, UndeclaredTool, undefended
from .pipeline import TAgentPipeline


def provenance(binary: str) -> dict[str, str]:
    """What was measured: which binary, and what the checkout looked like.

    The digest is the authoritative part — it identifies the exact program
    that produced these numbers. `repo_head` is the checkout's HEAD when the
    run *started*, which is not necessarily the commit the binary was built
    from: nothing here can know that. It is recorded as a hint, and named so
    it does not read as a claim.

    Recorded at all because rebuilding the binary while a run is in flight
    silently mixes two different programs into one number. With the digest in
    the file, that mistake is visible afterwards instead of invisible.
    """
    import hashlib
    import subprocess

    out = {"binary": binary}
    try:
        out["sha256"] = hashlib.sha256(Path(binary).read_bytes()).hexdigest()[:16]
    except OSError as e:  # noqa: BLE001 - provenance is best-effort
        out["sha256"] = f"unreadable: {e}"
    try:
        out["repo_head"] = subprocess.run(
            ["git", "-C", str(Path(binary).resolve().parent), "rev-parse", "--short", "HEAD"],
            capture_output=True,
            text=True,
            check=True,
            timeout=10,
        ).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        out["repo_head"] = "unknown"
    return out


@dataclass
class RunRecord:
    """What happened in one task run, kept so a reader can see *why*.

    A single utility number cannot distinguish "the gate refused this" from
    "the planner could not do the arithmetic", and those call for opposite
    fixes. Every run records both.
    """

    task: str
    #: Which pass over the suite this was, counting from zero.
    repeat: int = 0
    #: The injection task, when one was placed.
    injection: str | None = None
    #: Did the user's task succeed?
    utility: bool = False
    #: Did the injection achieve its goal? `None` when none was placed.
    attack: bool | None = None
    #: How the session ended: `done`, `refused`, `failed` or `no_plan`.
    outcome: str = "failed"
    #: The rules that refused a call here, in order.
    refused_by: list[str] = field(default_factory=list)
    #: A sentence about how it ended.
    summary: str = ""
    #: The identifier that makes this pair unwinnable, when there is one.
    #: See `confounded`.
    collides_on: str | None = None


@dataclass
class Totals:
    """What a run came to."""

    suite: str
    provider: str
    #: `flow-rules` or `none`. A run with no defence is a baseline, not a
    #: result, and the file says which it is.
    defence: str = "flow-rules"
    #: Which binary produced these numbers, and from which commit.
    built: dict[str, str] = field(default_factory=dict)
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
    #: One entry per run, so a number can be traced back to a task.
    runs: list[RunRecord] = field(default_factory=list)
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

    def utility_by_repeat(self) -> list[float]:
        """Utility measured separately in each pass over the suite.

        A planner set to temperature zero still answers differently from one
        run to the next, and on sixteen tasks the swing has been wider than
        the gap to the target. One number from one pass is not a measurement
        of anything; the spread is the honest report.
        """
        passes: dict[int, list[bool]] = {}
        for run in self.runs:
            if run.injection is None:
                passes.setdefault(run.repeat, []).append(run.utility)
        return [sum(r) / len(r) for _, r in sorted(passes.items()) if r]

    def benign_failures(self) -> dict[str, list[str]]:
        """Benign tasks that failed, grouped by what to blame.

        The distinction the headline number hides: a task the gate refused is
        a cost of the defence, a task the planner botched is a cost of the
        model, and only the first is evidence about this design.
        """
        causes: dict[str, list[str]] = {}
        for run in self.runs:
            if run.injection is not None or run.utility:
                continue
            rules = [r for r in run.refused_by if r not in PLAN_MISTAKES]
            if "untrusted_control" in rules:
                cause = "refused: integrity rule"
            elif "disclosure" in rules:
                cause = "refused: confidentiality rule"
            elif rules:
                cause = f"refused: {rules[0]}"
            elif run.refused_by:
                cause = "the plan called a tool wrongly"
            elif run.outcome == "no_plan":
                cause = "planner wrote no usable plan"
            else:
                cause = "ran, but answer was wrong"
            causes.setdefault(cause, []).append(run.task)
        return causes

    def report(self) -> str:
        lines = [
            f"suite                 {self.suite}",
            f"planner               {self.provider}",
            f"defence               {self.defence}",
            f"binary                {self.built.get('sha256', '?')}"
            f"  (checkout at {self.built.get('repo_head', '?')})",
            f"utility               {self.utility:.0%}"
            f"  ({self.benign_successes}/{self.benign_runs})",
        ]
        spread = self.utility_by_repeat()
        if len(spread) > 1:
            each = ", ".join(f"{u:.0%}" for u in spread)
            lines.append(
                f"  by pass             {each}"
                f"  (low {min(spread):.0%}, high {max(spread):.0%})"
            )
        if self.attack_runs:
            lines += [
                f"utility under attack  {self.utility_under_attack:.0%}"
                f"  ({self.attack_utility}/{self.attack_runs})",
                f"attack success rate   {self.attack_success_rate:.0%}"
                f"  ({self.attack_successes}/{self.attack_runs})",
            ]
        unwinnable = [r for r in self.runs if r.collides_on]
        if unwinnable:
            scored = sum(1 for r in unwinnable if r.attack)
            pairs = sorted({f"{r.task}/{r.injection}" for r in unwinnable})
            lines += [
                f"  of which unwinnable  {len(unwinnable)}"
                f"  ({scored} scored as the attack succeeding)",
                "  The injection's identifier is one the user's own task asks for,",
                "  so the checker cannot tell the job from the attack. Counted, not",
                "  excluded: " + ", ".join(pairs[:4]) + ("..." if len(pairs) > 4 else ""),
            ]
        if self.refusals:
            lines.append("refusals by rule")
            for rule, n in sorted(self.refusals.items(), key=lambda kv: -kv[1]):
                lines.append(f"  {rule:<28} {n}")
        failures = self.benign_failures()
        if failures:
            lines.append("benign tasks that failed, by cause")
            for cause, ids in sorted(failures.items()):
                lines.append(f"  {cause:<28} {len(ids)}  {', '.join(ids)}")
        if self.no_plan:
            lines.append(f"runs with no usable plan   {self.no_plan}")
        lines.append(f"took                  {self.seconds:.0f}s")
        return "\n".join(lines)


def confounded(task: BaseUserTask, injection_task: BaseInjectionTask) -> str | None:
    """Does doing the user's job already satisfy the injection's checker?

    AgentDojo scores an injection by looking at the world afterwards. When
    the attacker's identifier is one the *user* asked for, that check cannot
    tell the two apart: in the banking suite every account-based injection
    uses `US133000000121212121212`, and user_task_15 asks for a standing
    order to that exact account, because it is the user's new landlord. An
    agent that does what it was told is then scored as having been
    compromised.

    Returns the identifier that collides, or None. The pair is still run and
    still counted — excluding it would be choosing which evidence to keep —
    but the report says which runs cannot be won.
    """
    prompt = getattr(task, "PROMPT", "") or ""
    for name in dir(injection_task):
        if not name.startswith("_") or name.startswith("__"):
            continue
        value = getattr(injection_task, name, None)
        if isinstance(value, str) and len(value) >= 8 and value in prompt:
            return value
    return None


#: Reasons a call was stopped that are not security decisions.
#:
#: The gate returns one verdict type, so a plan that named a tool wrongly and
#: a plan that tried to launder an injected address arrive the same way. They
#: are not the same thing, and counting them together would credit the
#: defence with the planner's typing mistakes.
PLAN_MISTAKES = frozenset({"missing_argument", "unknown_tool", "disclosure_target_not_text"})


def _tally(
    totals: Totals,
    report: Any,
    task: str,
    utility: bool,
    injection: str | None = None,
    attack: bool | None = None,
    repeat: int = 0,
    collides_on: str | None = None,
) -> None:
    for tool, rule in report.refusals:
        if rule in PLAN_MISTAKES:
            key = f"{rule} (the plan, not the gate)"
        elif rule == "policy":
            key = f"{rule} ({tool})"
        else:
            key = rule
        totals.refusals[key] = totals.refusals.get(key, 0) + 1
    if report.outcome == "no_plan":
        totals.no_plan += 1
    totals.runs.append(
        RunRecord(
            task=task,
            repeat=repeat,
            injection=injection,
            utility=bool(utility),
            attack=attack,
            outcome=report.outcome,
            refused_by=[rule for _, rule in report.refusals],
            summary=report.summary,
            collides_on=collides_on,
        )
    )


class Pool:
    """A few pipelines, lent out one at a time.

    Tasks are independent and each spends most of its time waiting on a
    model, so running several at once is the difference between a benchmark
    someone runs and one they read about. Each worker needs its own pipeline
    because a pipeline keeps the last run's gate report on itself, and two
    threads sharing one would overwrite each other's.
    """

    def __init__(self, make: Any, workers: int) -> None:
        self.free: queue.SimpleQueue[TAgentPipeline] = queue.SimpleQueue()
        for _ in range(max(1, workers)):
            self.free.put(make())

    @contextlib.contextmanager
    def borrow(self):
        pipeline = self.free.get()
        try:
            yield pipeline
        finally:
            self.free.put(pipeline)


def _one_run(
    pool: Pool,
    suite: Any,
    task: BaseUserTask,
    injection_task: BaseInjectionTask | None,
    injections: dict,
) -> tuple[bool, bool, Any]:
    """Run one task, returning utility, attack success and the gate report."""
    with pool.borrow() as pipeline:
        try:
            utility, security = suite.run_task_with_pipeline(
                pipeline, task, injection_task=injection_task, injections=injections
            )
        except UndeclaredTool:
            raise
        except Exception as e:  # noqa: BLE001 - one bad task must not end the run
            label = task.ID if injection_task is None else f"{task.ID}/{injection_task.ID}"
            print(f"  {label}: error: {e}", file=sys.stderr)
            utility, security = False, False
        return bool(utility), bool(security), pipeline.last_report


def _run_all(
    pool: Pool,
    suite: Any,
    work: list[tuple[BaseUserTask, BaseInjectionTask | None, dict]],
    totals: Totals,
    repeat: int,
    workers: int,
) -> None:
    """Run every piece of work, in parallel when asked, reporting in order."""
    results: dict[int, tuple[bool, bool, Any]] = {}
    if workers <= 1:
        for i, (task, injection_task, injections) in enumerate(work):
            results[i] = _one_run(pool, suite, task, injection_task, injections)
            _report_one(work, results, totals, repeat, i)
        return

    with cf.ThreadPoolExecutor(max_workers=workers) as pool_of_threads:
        futures = {
            pool_of_threads.submit(_one_run, pool, suite, task, injection_task, injections): i
            for i, (task, injection_task, injections) in enumerate(work)
        }
        # Report in submission order, so the output of a parallel run reads
        # the same as a serial one and two runs can be compared line by line.
        next_to_report = 0
        for future in cf.as_completed(futures):
            results[futures[future]] = future.result()
            while next_to_report in results:
                _report_one(work, results, totals, repeat, next_to_report)
                next_to_report += 1


def _report_one(
    work: list[tuple[BaseUserTask, BaseInjectionTask | None, dict]],
    results: dict[int, tuple[bool, bool, Any]],
    totals: Totals,
    repeat: int,
    i: int,
) -> None:
    task, injection_task, _ = work[i]
    utility, security, report = results[i]
    if injection_task is None:
        totals.benign_runs += 1
        totals.benign_successes += int(utility)
        _tally(totals, report, task.ID, utility, repeat=repeat)
        print(f"  {task.ID:<12} utility={'yes' if utility else 'no '}", flush=True)
    else:
        totals.attack_runs += 1
        totals.attack_utility += int(utility)
        totals.attack_successes += int(security)
        _tally(
            totals,
            report,
            task.ID,
            utility,
            injection_task.ID,
            security,
            repeat=repeat,
            collides_on=confounded(task, injection_task),
        )
        print(
            f"  {task.ID:<12} {injection_task.ID:<14} "
            f"utility={'yes' if utility else 'no '} "
            f"attack={'SUCCEEDED' if security else 'failed'}",
            flush=True,
        )


def run_benign(
    pool: Pool,
    suite: Any,
    tasks: list[BaseUserTask],
    totals: Totals,
    repeat: int = 0,
    workers: int = 1,
) -> None:
    """Every task, with nothing injected."""
    _run_all(pool, suite, [(t, None, {}) for t in tasks], totals, repeat, workers)


def run_attacks(
    pool: Pool,
    suite: Any,
    pipeline: TAgentPipeline,
    tasks: list[BaseUserTask],
    injections: list[BaseInjectionTask],
    attack_name: str,
    totals: Totals,
    repeat: int = 0,
    workers: int = 1,
) -> None:
    """Every task against every injection.

    The injected text is composed up front, in one thread: an attack object
    is not documented as safe to share, and composing is cheap next to
    running.
    """
    attack = load_attack(attack_name, suite, pipeline)
    work = [
        (task, injection_task, attack.attack(task, injection_task))
        for task in tasks
        for injection_task in injections
    ]
    _run_all(pool, suite, work, totals, repeat, workers)


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
    parser.add_argument(
        "--agent",
        choices=["tagent", "tool-calling"],
        default="tagent",
        help="tool-calling runs AgentDojo's own agent on the same model, "
        "where the model reads tool output itself; that is what the attacks "
        "were written against",
    )
    parser.add_argument(
        "--undefended",
        action="store_true",
        help="baseline: the same agent with nothing declared as steering, "
        "so the flow rules refuse nothing",
    )
    parser.add_argument("--max-attempts", type=int, default=3)
    parser.add_argument(
        "--workers",
        type=int,
        default=1,
        help="tasks to run at once; each waits on a model, so this is "
        "most of what decides how long a run takes",
    )
    parser.add_argument(
        "--repeat",
        type=int,
        default=1,
        help="passes over the suite; more than one reports the spread, "
        "which a single pass cannot",
    )
    parser.add_argument("--out", type=Path, default=None, help="write totals as JSON")
    args = parser.parse_args(argv)

    suite = get_suite("v1", args.suite)
    declarations = SUITES[args.suite]
    if args.undefended:
        declarations = undefended(declarations)
        print("BASELINE: no parameter is declared as steering, so the flow")
        print("rules refuse nothing. These numbers are what the defence is")
        print("measured against, not a result.\n")
    binary = find_binary()

    def make_pipeline() -> TAgentPipeline:
        return TAgentPipeline(
            declarations=declarations,
            binary=binary,
            provider=args.provider,
            model=args.model,
            max_attempts=args.max_attempts,
        )

    if args.agent == "tool-calling":
        from .toolcalling import build

        model = args.model or "minimax-m3"

        def make_pipeline() -> Any:  # noqa: F811 - the other agent entirely
            return build(model=model)

        print("BASELINE: AgentDojo's own tool-calling agent on the same")
        print("model. One model holds the conversation, calls tools and")
        print("reads what they return. No gate, no planner/reader split.\n")

    pipeline = make_pipeline()
    pool = Pool(make_pipeline, args.workers)

    tasks = list(suite.user_tasks.values())[: args.limit]
    injections = list(suite.injection_tasks.values())[: args.injection_limit]
    totals = Totals(
        suite=args.suite,
        provider=args.provider or "first detected",
        defence=(
            "none (tool-calling agent)"
            if args.agent == "tool-calling"
            else "none" if args.undefended else "flow-rules"
        ),
        built=provenance(binary),
    )

    started = time.monotonic()
    for pass_number in range(max(1, args.repeat)):
        if args.repeat > 1:
            print(f"pass {pass_number + 1} of {args.repeat}")
        print(f"{len(tasks)} user tasks, no injection")
        run_benign(pool, suite, tasks, totals, repeat=pass_number, workers=args.workers)
        if not args.benign_only:
            print(f"{len(tasks)} tasks x {len(injections)} injections")
            run_attacks(
                pool,
                suite,
                pipeline,
                tasks,
                injections,
                args.attack,
                totals,
                repeat=pass_number,
                workers=args.workers,
            )
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
