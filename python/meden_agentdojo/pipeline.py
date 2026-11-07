"""Driving the runtime from inside an AgentDojo pipeline.

AgentDojo owns the tools and the world they act on. The planning, the labels
and the gate stay in the Rust binary. This element is the seam: it describes
the suite's tools, starts `meden run`, executes the calls the runtime asks
for, and reports what the gate decided.

The messages it builds are the ones AgentDojo scores: an assistant message per
tool call, a tool message per result, and a final assistant message carrying
the answer. A call the gate refused never reaches `run_function`, so it leaves
no trace in the environment, which is exactly what the benchmark measures.
"""

from __future__ import annotations

import json
import subprocess
from collections.abc import Sequence
from dataclasses import dataclass, field
from typing import Any

from agentdojo.agent_pipeline.base_pipeline_element import BasePipelineElement
from agentdojo.functions_runtime import EmptyEnv, FunctionCall, FunctionsRuntime
from agentdojo.types import (
    ChatAssistantMessage,
    ChatToolResultMessage,
    text_content_block_from_string,
)

from .declarations import PERMISSIVE_POLICY, tool_specs


@dataclass
class GateReport:
    """What the gate did during one task."""

    #: One entry per call it ruled on: `(tool, allowed, rule)`.
    decisions: list[tuple[str, bool, str | None]] = field(default_factory=list)
    #: How the session ended: `done`, `refused`, `failed`, `no_plan`, `incomplete`.
    outcome: str = "failed"
    #: A sentence about how it ended.
    summary: str = ""
    #: What the planner wrote, one entry per attempt.
    plans: list[Any] = field(default_factory=list)
    #: Executed calls and their results, for diagnosing utility failures.
    calls: list[dict[str, Any]] = field(default_factory=list)
    #: Final answer, kept separately from the runtime's status sentence.
    answer: Any = None
    #: Exact API history in native conversation mode.
    messages: list[dict[str, Any]] = field(default_factory=list)

    @property
    def refusals(self) -> list[tuple[str, str]]:
        """The calls that were refused, as `(tool, rule)`."""
        return [(t, r or "unknown") for t, ok, r in self.decisions if not ok]

    @property
    def refused_for_provenance(self) -> bool:
        """Did the integrity rule refuse anything?"""
        return any(rule == "untrusted_control" for _, rule in self.refusals)


def _to_jsonable(value: Any) -> Any:
    """Plain JSON for whatever a tool returned."""
    if hasattr(value, "model_dump"):
        return value.model_dump(mode="json")
    if isinstance(value, (list, tuple)):
        return [_to_jsonable(v) for v in value]
    if isinstance(value, dict):
        return {str(k): _to_jsonable(v) for k, v in value.items()}
    if isinstance(value, (str, int, float, bool)) or value is None:
        return value
    return str(value)


#: How to spell the planner so AgentDojo's attacks address it correctly.
#:
#: The `important_instructions` attack writes "You are <model>" into the
#: injected text, and takes <model> from the pipeline's name. An injection
#: addressed to the wrong model is a weaker injection, so getting this right
#: makes the attack stronger, not the defence better. The keys are substrings
#: AgentDojo looks for; the fallback is `local`, which it reads as "Local
#: model" and which is what an open model served over an OpenAI-compatible
#: endpoint is.
_VICTIM_NAMES = {
    "claude": "claude-3-5-sonnet-20241022",
    "anthropic": "claude-3-5-sonnet-20241022",
    "gpt": "gpt-4o-2024-05-13",
    "openai": "gpt-4o-2024-05-13",
    "codex": "gpt-4o-2024-05-13",
    "gemini": "gemini-2.0-flash-001",
}


def victim_name(provider: str | None, model: str | None) -> str:
    """What to call the planner, in the vocabulary AgentDojo's attacks use."""
    haystack = f"{provider or ''} {model or ''}".lower()
    for key, spelling in _VICTIM_NAMES.items():
        if key in haystack:
            return spelling
    return "local"


class MedenPipeline(BasePipelineElement):
    """An AgentDojo pipeline element backed by the capability-typed runtime."""

    def __init__(
        self,
        declarations: dict[str, dict[str, Any]],
        binary: str,
        provider: str | None = None,
        model: str | None = None,
        policy: dict[str, Any] | None = None,
        max_attempts: int = 3,
        timeout: float = 600.0,
        mode: str = "plan",
    ) -> None:
        self.declarations = declarations
        self.binary = binary
        self.provider = provider
        self.model = model
        self.policy = policy or PERMISSIVE_POLICY
        self.max_attempts = max_attempts
        self.timeout = timeout
        if mode not in ("plan", "tool-calling"):
            raise ValueError(f"unsupported meden mode: {mode}")
        self.mode = mode
        #: Read by AgentDojo's attacks to address the injection to the right
        #: model. See `victim_name`.
        self.name = f"meden ({victim_name(provider, model)})"
        #: The report from the most recent `query`, for a caller collecting
        #: statistics.
        self.last_report = GateReport()

    def _command(self) -> list[str]:
        command = [self.binary, "run"]
        if self.mode != "plan":
            command += ["--mode", self.mode]
        if self.provider:
            command += ["--provider", self.provider]
        if self.model:
            command += ["--model", self.model]
        return command

    def query(
        self,
        query: str,
        runtime: FunctionsRuntime,
        env: Any = None,
        messages: Sequence[Any] = (),
        extra_args: dict | None = None,
    ) -> tuple[str, FunctionsRuntime, Any, Sequence[Any], dict]:
        env = EmptyEnv() if env is None else env
        extra_args = dict(extra_args or {})
        start = {
            "instruction": query,
            "tools": tool_specs(runtime, self.declarations),
            "policy": self.policy,
            "max_attempts": self.max_attempts,
        }
        report = GateReport()
        if self.mode == "tool-calling":
            from agentdojo.agent_pipeline.agent_pipeline import load_system_message

            start["system_message"] = load_system_message(None)
            start["max_iterations"] = 15
        new_messages = list(messages)

        process = subprocess.Popen(
            self._command(),
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
        try:
            assert process.stdin is not None and process.stdout is not None
            process.stdin.write(json.dumps(start) + "\n")
            process.stdin.flush()

            while True:
                line = process.stdout.readline()
                if not line:
                    break
                try:
                    message = json.loads(line)
                except json.JSONDecodeError:
                    continue

                kind = message.get("type")
                if kind == "call":
                    call = FunctionCall(
                        function=message["tool"],
                        args=message.get("args") or {},
                        id=str(message["id"]),
                    )
                    result, error = runtime.run_function(
                        env, call.function, call.args, raise_on_error=False
                    )
                    report.calls.append(
                        {
                            "tool": call.function,
                            "args": call.args,
                            "result": _to_jsonable(result),
                            "error": error,
                        }
                    )
                    new_messages.append(
                        ChatAssistantMessage(role="assistant", content=None, tool_calls=[call])
                    )
                    new_messages.append(
                        ChatToolResultMessage(
                            role="tool",
                            content=[text_content_block_from_string(str(result))],
                            tool_call_id=call.id,
                            tool_call=call,
                            error=error,
                        )
                    )
                    reply: dict[str, Any] = {"id": message["id"]}
                    if error:
                        reply["error"] = error
                    elif self.mode == "tool-calling":
                        from agentdojo.agent_pipeline.tool_execution import tool_result_to_str

                        reply["ok"] = tool_result_to_str(result)
                    else:
                        reply["ok"] = _to_jsonable(result)
                    process.stdin.write(json.dumps(reply) + "\n")
                    process.stdin.flush()

                elif kind == "plan":
                    report.plans.append(message.get("plan"))

                elif kind == "conversation":
                    report.messages = message["messages"]

                elif kind == "decision":
                    report.decisions.append(
                        (message["tool"], bool(message["allowed"]), message.get("rule"))
                    )

                elif kind == "done":
                    report.outcome = message.get("outcome", "failed")
                    report.summary = message.get("summary", "")
                    answer = message.get("answer")
                    report.answer = answer
                    text = report.summary if answer is None else str(answer)
                    new_messages.append(
                        ChatAssistantMessage(
                            role="assistant",
                            content=[text_content_block_from_string(text)],
                            tool_calls=None,
                        )
                    )
                    break
        finally:
            if process.stdin and not process.stdin.closed:
                process.stdin.close()
            try:
                process.wait(timeout=self.timeout)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            if process.stderr:
                stderr = process.stderr.read()
                if stderr.strip() and not report.summary:
                    report.summary = stderr.strip().splitlines()[-1]
                process.stderr.close()

        self.last_report = report
        extra_args["meden_report"] = report
        return query, runtime, env, new_messages, extra_args
