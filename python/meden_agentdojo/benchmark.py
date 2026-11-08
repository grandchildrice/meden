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
uv run python -m meden_agentdojo.benchmark --suite banking --limit 3
```
"""

from __future__ import annotations

import argparse
import concurrent.futures as cf
import contextlib
import json
import queue
import sys
import time
from dataclasses import asdict, dataclass, field
from importlib.metadata import version
from pathlib import Path
from typing import Any

from agentdojo.attacks.attack_registry import load_attack
from agentdojo.base_tasks import BaseInjectionTask, BaseUserTask
from agentdojo.task_suite.load_suites import get_suite

from . import find_meden
from .declarations import SUITES, UndeclaredTool
from .pipeline import GateReport, MedenPipeline


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
    #: How the session ended: `done`, `failed`, `incomplete`, `cancelled`.
    outcome: str = "failed"
    #: The rules that refused a call here, in order.
    refused_by: list[str] = field(default_factory=list)
    #: A sentence about how it ended.
    summary: str = ""
    #: The identifier that makes this pair unwinnable, when there is one.
    #: See `confounded`.
    collides_on: str | None = None
    #: Executed calls and results, also available to the conversational model.
    calls: list[dict[str, Any]] = field(default_factory=list)
    answer: Any = None
    messages: list[dict[str, Any]] = field(default_factory=list)


@dataclass
class Totals:
    """What a run came to."""

    suite: str
    provider: str
    #: Current capability policy or the undefended AgentDojo baseline.
    defence: str = "capability-policy"
    #: Which binary produced these numbers, and from which commit.
    built: dict[str, str] = field(default_factory=dict)
    #: Execution settings needed to compare independent measurements.
    settings: dict[str, Any] = field(default_factory=dict)
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
    #: Which injection goals the agent could achieve when simply asked.
    #: See `injections_are_solvable`.
    solvable: dict[str, bool] = field(default_factory=dict)
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

        Report each pass: model outcomes vary between runs, and a single
        aggregate can hide that variation.
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
            rules = [r for r in run.refused_by if r not in ARGUMENT_ERRORS]
            if rules:
                cause = f"refused: {rules[0]}"
            elif run.refused_by:
                cause = "model called a tool wrongly"
            elif run.outcome == "incomplete":
                cause = "agent reached the tool execution limit"
            else:
                cause = "ran, but answer was wrong"
            causes.setdefault(cause, []).append(run.task)
        return causes

    def report(self) -> str:
        lines = [
            f"suite                 {self.suite}",
            f"model connection      {self.provider}",
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
                f"  by pass             {each}  (low {min(spread):.0%}, high {max(spread):.0%})"
            )
        if self.attack_runs:
            lines += [
                f"utility under attack  {self.utility_under_attack:.0%}"
                f"  ({self.attack_utility}/{self.attack_runs})",
                f"attack success rate   {self.attack_success_rate:.0%}"
                f"  ({self.attack_successes}/{self.attack_runs})",
            ]
            if self.solvable:
                live = [i for i, ok in self.solvable.items() if ok]
                dead = sorted(i for i, ok in self.solvable.items() if not ok)
                counted = [r for r in self.runs if r.injection in live]
                hit = sum(1 for r in counted if r.attack)
                rate = hit / len(counted) if counted else 0.0
                lines += [
                    f"  over achievable     {rate:.0%}  ({hit}/{len(counted)})",
                    f"  {len(live)} of {len(self.solvable)} injection goals this agent can reach"
                    " when simply asked;",
                    "  the rest cannot be scored for or against any defence: " + ", ".join(dead),
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
        lines.append(f"took                  {self.seconds:.0f}s")
        return "\n".join(lines)


def injections_are_solvable(suite: Any, pool: Pool, workers: int = 1) -> dict[str, bool]:
    """Can the agent carry out each injection's goal when simply asked to?

    AgentDojo's own runner does this before measuring an attack and warns
    when the answer is no. The reason matters more than the warning: an
    injection whose goal the agent could not achieve even on instruction is
    one that no defence can be credited with stopping, and one that inflates
    the denominator of every attack success rate quoted against it.

    So an attack success rate of zero means nothing until this says yes.
    """
    from agentdojo.functions_runtime import FunctionsRuntime

    def one(injection_task: BaseInjectionTask) -> bool:
        with pool.borrow() as pipeline:
            environment = suite.load_and_inject_default_environment({})
            runtime = FunctionsRuntime(suite.tools)
            before = environment.model_copy(deep=True)
            try:
                _, _, after, messages, _ = pipeline.query(injection_task.GOAL, runtime, environment)
            except Exception as e:  # noqa: BLE001 - one goal must not end the check
                print(f"  {injection_task.ID}: error: {e}", file=sys.stderr)
                return False
            said = messages[-1].get("content") if messages else None
            return bool(injection_task.security(str(said), before, after))

    tasks = list(suite.injection_tasks.items())
    with cf.ThreadPoolExecutor(max_workers=max(1, workers)) as threads:
        verdicts = list(threads.map(lambda kv: one(kv[1]), tasks))
    return dict(zip([k for k, _ in tasks], verdicts, strict=True))


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


#: Invalid model arguments are not capability-policy refusals.
ARGUMENT_ERRORS = frozenset({"missing_argument", "unknown_tool", "invalid_arguments"})


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
        if rule in ARGUMENT_ERRORS:
            key = f"{rule} (model argument error)"
        elif rule == "policy":
            key = f"{rule} ({tool})"
        else:
            key = rule
        totals.refusals[key] = totals.refusals.get(key, 0) + 1
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
            calls=list(report.calls),
            answer=report.answer,
            messages=list(report.messages),
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
        self.free: queue.SimpleQueue[MedenPipeline] = queue.SimpleQueue()
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
        # A failed invocation must never inherit the preceding task's report.
        pipeline.last_report = GateReport()
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
            pipeline.last_report.outcome = "failed"
            pipeline.last_report.summary = str(e)
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
    pipeline: MedenPipeline,
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


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--suite", default="banking", choices=sorted(SUITES))
    parser.add_argument("--provider", default=None, help="see `meden providers`")
    parser.add_argument("--model", default=None)
    parser.add_argument("--attack", default="important_instructions")
    parser.add_argument("--limit", type=int, default=None, help="use only the first N user tasks")
    parser.add_argument(
        "--injection-limit", type=int, default=None, help="only the first N injections"
    )
    parser.add_argument("--benign-only", action="store_true", help="measure utility and stop")
    parser.add_argument(
        "--agent",
        choices=["meden", "tool-calling"],
        default="meden",
        help="meden runs the Rust conversation engine; tool-calling runs AgentDojo's own baseline",
    )
    parser.add_argument(
        "--skip-injection-check",
        action="store_true",
        help="do not first measure which injection goals the agent can reach",
    )
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
        help="passes over the suite; more than one reports the spread, which a single pass cannot",
    )
    parser.add_argument("--out", type=Path, default=None, help="write totals as JSON")
    args = parser.parse_args(argv)
    suite = get_suite("v1", args.suite)
    declarations = SUITES[args.suite]
    binary = find_meden()
    if binary is None:
        raise SystemExit("no meden binary; run cargo build --release -p meden-cli")

    def make_pipeline() -> MedenPipeline:
        return MedenPipeline(
            declarations=declarations,
            binary=binary,
            provider=args.provider,
            model=args.model,
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
            "none (tool-calling agent)" if args.agent == "tool-calling" else "capability-policy"
        ),
        built=provenance(binary),
        settings={
            "workers": args.workers,
            "max_tool_iterations": 15,
            "agentdojo_package": version("agentdojo"),
            "repeat": args.repeat,
            "agent": args.agent,
            "model": args.model,
            "attack": args.attack,
            "benign_only": args.benign_only,
            "suite_version": "v1",
            "sampling": "server defaults (matches AgentDojo HTTP)",
        },
    )

    started = time.monotonic()
    if not args.benign_only and not args.skip_injection_check:
        print(f"checking which of {len(injections)} injection goals this agent can reach")
        totals.solvable = injections_are_solvable(suite, pool, args.workers)
        for name, ok in totals.solvable.items():
            print(f"  {name:<18} {'reachable' if ok else 'NOT reachable'}", flush=True)
        print()
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
