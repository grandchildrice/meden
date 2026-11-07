"""Our runner must drive AgentDojo the way AgentDojo drives itself.

The numbers here are only comparable with published ones if the scoring path
is the same path. It is: both end at `TaskSuite.run_task_with_pipeline` with
the same user task, injection task and injection text. This pins that down
without a model, so a refactor that quietly changes what is measured fails
here rather than in a number nobody can reproduce.
"""

from __future__ import annotations

from typing import Any

from agentdojo.agent_pipeline.base_pipeline_element import BasePipelineElement


class _Recorder(BasePipelineElement):
    """A pipeline that runs nothing and remembers nothing was asked of it."""

    name = "vllm_parsed"

    def __init__(self) -> None:
        from meden_agentdojo.pipeline import GateReport

        self.last_report = GateReport(outcome="done", summary="recorded")

    def query(self, query, runtime, env=None, messages=(), extra_args=None):
        return query, runtime, env, list(messages), dict(extra_args or {})


def _calls_made_by(runner: str) -> list[tuple[str, str, dict]]:
    """What reached `run_task_with_pipeline`, under each runner."""
    from agentdojo.attacks.attack_registry import load_attack
    from agentdojo.task_suite.load_suites import get_suite

    suite = get_suite("v1", "banking")
    pipeline = _Recorder()
    seen: list[tuple[str, str, dict]] = []

    original = type(suite).run_task_with_pipeline

    def spy(self, agent_pipeline, user_task, injection_task, injections, **kw):
        seen.append(
            (
                user_task.ID,
                injection_task.ID if injection_task is not None else "",
                dict(injections),
            )
        )
        return False, False

    type(suite).run_task_with_pipeline = spy
    try:
        tasks = list(suite.user_tasks.values())[:2]
        injections = list(suite.injection_tasks.values())[:2]
        if runner == "ours":
            from meden_agentdojo.benchmark import Pool, Totals, run_attacks

            pool = Pool(lambda: pipeline, 1)
            run_attacks(
                pool,
                suite,
                pipeline,
                tasks,
                injections,
                "important_instructions",
                Totals(suite="banking", provider="none"),
            )
        else:
            from agentdojo.benchmark import run_task_with_injection_tasks
            from agentdojo.logging import OutputLogger

            attack = load_attack("important_instructions", suite, pipeline)
            # Their runner writes a trace; give it somewhere to write.
            with OutputLogger(None):
                for task in tasks:
                    run_task_with_injection_tasks(
                        suite,
                        pipeline,
                        task,
                        attack,
                        logdir=None,
                        force_rerun=True,
                        injection_tasks=[i.ID for i in injections],
                    )
    finally:
        type(suite).run_task_with_pipeline = original
    return seen


def test_same_tasks_same_injections_same_order() -> None:
    ours = _calls_made_by("ours")
    theirs = _calls_made_by("agentdojo")
    assert ours == theirs, (
        "our runner and AgentDojo's own do not put the same thing in front of "
        f"the agent.\nours:   {ours}\ntheirs: {theirs}"
    )


def test_the_injected_text_is_actually_placed() -> None:
    """An injection that never reaches the environment measures nothing."""
    import json

    from agentdojo.attacks.attack_registry import load_attack
    from agentdojo.task_suite.load_suites import get_suite

    suite = get_suite("v1", "banking")
    attack = load_attack("important_instructions", suite, _Recorder())
    task = suite.user_tasks["user_task_1"]
    injection = suite.injection_tasks["injection_task_0"]
    placed = attack.attack(task, injection)
    assert placed, "the attack placed nothing"
    environment = suite.load_and_inject_default_environment(placed)
    text = json.dumps(environment.model_dump(mode="json"))
    assert "<INFORMATION>" in text, "the injected text never reached the environment"
    assert "US133000000121212121212" in text


def test_the_baseline_is_the_pipeline_agentdojo_builds() -> None:
    """The baseline must be AgentDojo's composition, not a hand-written one."""
    from agentdojo.agent_pipeline import AgentPipeline, PipelineConfig
    from agentdojo.agent_pipeline.agent_pipeline import load_system_message

    from meden_agentdojo.toolcalling import AGENTDOJO_LOCAL_PROVIDER

    class _Llm(BasePipelineElement):
        name = None

        def query(self, *a: Any, **k: Any):
            raise AssertionError("not called")

    reference = AgentPipeline.from_config(
        PipelineConfig(
            llm=_Llm(), model_id=None, defense=None, system_message_name=None, system_message=None
        )
    )
    shapes = [type(e).__name__ for e in reference.elements]
    assert shapes == ["SystemMessage", "InitQuery", "_Llm", "ToolsExecutionLoop"], shapes
    # The system message is the one AgentDojo ships, including the four
    # instructions a hand-written one tends to drop.
    assert reference.elements[0].system_message == load_system_message(None)
    assert "Do not assume the current year" in reference.elements[0].system_message
    assert AGENTDOJO_LOCAL_PROVIDER == "vllm_parsed"


def test_a_failed_task_does_not_reuse_the_previous_tasks_report():
    from types import SimpleNamespace

    from meden_agentdojo.benchmark import Pool, _one_run
    from meden_agentdojo.pipeline import GateReport

    previous = GateReport(outcome="done", summary="previous task")
    pipeline = SimpleNamespace(last_report=previous)
    pool = Pool(lambda: pipeline, 1)

    class FailingSuite:
        def run_task_with_pipeline(self, *args, **kwargs):
            raise RuntimeError("current task failed")

    utility, security, report = _one_run(
        pool, FailingSuite(), SimpleNamespace(ID="task"), None, {}
    )
    assert not utility and not security
    assert report is not previous
    assert report.outcome == "failed"
    assert report.summary == "current task failed"
