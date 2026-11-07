"""AgentDojo adapter: mount the capability gate inside an AgentDojo pipeline.

AgentDojo evaluates an agent as a chain of ``BasePipelineElement`` objects.
``CapabilityGateElement`` sits between the LLM and the ``ToolsExecutor``: it
inspects the tool calls in the last assistant message and replaces blocked
ones with a synthetic tool result that explains the refusal, so the model can
recover but the effect never happens.

This module imports ``agentdojo`` lazily so the core package does not depend
on it. Install with ``uv sync --extra agentdojo``.
"""

from __future__ import annotations

from collections.abc import Sequence
from typing import Any

from .hook import CapabilityGate, ToolCall


def _agentdojo():
    try:
        from agentdojo.agent_pipeline.base_pipeline_element import BasePipelineElement
        from agentdojo.functions_runtime import EmptyEnv, FunctionsRuntime
        from agentdojo.types import ChatAssistantMessage, ChatToolResultMessage
    except ImportError as exc:  # pragma: no cover - exercised only without the extra
        raise ImportError(
            "the AgentDojo adapter needs the 'agentdojo' extra: uv sync --extra agentdojo"
        ) from exc
    return (
        BasePipelineElement,
        EmptyEnv,
        FunctionsRuntime,
        ChatAssistantMessage,
        ChatToolResultMessage,
    )


def make_gate_element(gate: CapabilityGate):
    """Build the pipeline element class at call time (agentdojo is optional)."""
    BasePipelineElement, EmptyEnv, _runtime, ChatAssistantMessage, ChatToolResultMessage = (
        _agentdojo()
    )

    class CapabilityGateElement(BasePipelineElement):  # type: ignore[misc,valid-type]
        name = "t_agent.CapabilityGateElement"

        def __init__(self, gate: CapabilityGate) -> None:
            self.gate = gate

        def query(
            self,
            query: str,
            runtime: Any,
            env: Any = None,
            messages: Sequence[Any] = (),
            extra_args: dict[str, Any] | None = None,
        ):
            env = env if env is not None else EmptyEnv()
            extra_args = extra_args or {}
            messages = list(messages)
            if not messages or messages[-1]["role"] != "assistant":
                return query, runtime, env, messages, extra_args
            last = messages[-1]
            tool_calls = last.get("tool_calls") or []
            kept, refusals = [], []
            for tc in tool_calls:
                call = ToolCall(name=tc.function, arguments=dict(tc.args), call_id=tc.id)
                result = self.gate(call)
                if result.block:
                    refusals.append(
                        ChatToolResultMessage(
                            role="tool",
                            content=[
                                {
                                    "type": "text",
                                    "content": f"[blocked by capability gate] {result.reason}",
                                }
                            ],
                            tool_call_id=tc.id,
                            tool_call=tc,
                            error=result.reason,
                        )
                    )
                else:
                    kept.append(tc)
            if refusals:
                messages[-1] = ChatAssistantMessage(
                    role="assistant", content=last.get("content"), tool_calls=kept or None
                )
                messages.extend(refusals)
            return query, runtime, env, messages, extra_args

    return CapabilityGateElement(gate)
