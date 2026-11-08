"""AgentDojo tools hosted in Python, conversation and policy checks in Rust."""

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
    #: How the session ended: `done`, `failed`, `incomplete`, `cancelled`.
    outcome: str = "failed"
    #: A sentence about how it ended.
    summary: str = ""
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


class MedenPipeline(BasePipelineElement):
    """An AgentDojo pipeline element backed by the capability-typed runtime."""

    def __init__(
        self,
        declarations: dict[str, dict[str, Any]],
        binary: str,
        provider: str | None = None,
        model: str | None = None,
        approve_processor: str | None = None,
        policy: dict[str, Any] | None = None,
        timeout: float = 600.0,
    ) -> None:
        self.declarations = declarations
        self.binary = binary
        self.provider = provider
        self.model = model
        self.approve_processor = approve_processor
        self.policy = policy or PERMISSIVE_POLICY
        self.timeout = timeout
        self.name = "vllm_parsed"
        #: The report from the most recent `query`, for a caller collecting
        #: statistics.
        self.last_report = GateReport()

    def _command(self) -> list[str]:
        if not self.approve_processor:
            raise ValueError("set approve_processor to the exact authorized inference URL")
        command = [self.binary, "run"]
        command += ["--approve-processor", self.approve_processor]
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
        }
        report = GateReport()
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
                    else:
                        from agentdojo.agent_pipeline.tool_execution import tool_result_to_str

                        reply["ok"] = tool_result_to_str(result)
                    process.stdin.write(json.dumps(reply) + "\n")
                    process.stdin.flush()

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
