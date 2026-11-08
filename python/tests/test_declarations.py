"""The declarations must cover every tool of every suite they claim.

These run without a model and without the binary, so a declaration that has
drifted from AgentDojo is caught in CI rather than two hours into a run.
"""


def test_every_suite_is_fully_declared():
    """Each suite's tools are all declared, with parameters that exist.

    A missing declaration is caught at run time by `UndeclaredTool`, which is
    the right behaviour but a poor way to find out — halfway through a
    two-hour benchmark. This finds it in a second.
    """
    import types

    from agentdojo.task_suite.load_suites import get_suite

    from meden_agentdojo.declarations import SUITES, tool_specs

    for name, declarations in SUITES.items():
        suite = get_suite("v1", name)
        runtime = types.SimpleNamespace(functions={t.name: t for t in suite.tools})
        specs = tool_specs(runtime, declarations)
        assert len(specs) == len(suite.tools), name
        assert all(spec["description"] for spec in specs), (
            f"{name}: a tool reached the planner with no description, "
            "which is how it cannot tell two tools apart"
        )
        # The ordinary tool-calling baseline gets these schemas. Dropping
        # types/defaults here forces the planner to guess valid arguments.
        for spec, tool in zip(specs, suite.tools, strict=True):
            assert spec["parameters_schema"] == tool.parameters.model_json_schema()


def test_a_tool_nobody_declared_is_refused_not_defaulted():
    import types

    import pytest

    from meden_agentdojo.declarations import UndeclaredTool, tool_specs

    class Param:
        model_fields = {"x": None}

    tool = types.SimpleNamespace(name="mystery", description="d", parameters=Param)
    runtime = types.SimpleNamespace(functions={"mystery": tool})
    with pytest.raises(UndeclaredTool):
        tool_specs(runtime, {})


def test_the_banking_collision_is_detected():
    """user_task_15 asks for the account every account-based injection uses.

    Doing what the user asked then satisfies the injection's checker. The
    benchmark must say so rather than report it as a breach.
    """
    from agentdojo.task_suite.load_suites import get_suite

    from meden_agentdojo.benchmark import confounded

    suite = get_suite("v1", "banking")
    task = suite.user_tasks["user_task_15"]
    collides = confounded(task, suite.injection_tasks["injection_task_4"])
    assert collides == "US133000000121212121212", collides

    # And a pair with no such collision is not flagged.
    other = suite.user_tasks["user_task_1"]
    assert confounded(other, suite.injection_tasks["injection_task_4"]) is None
