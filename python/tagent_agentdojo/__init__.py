"""AgentDojo adapter for the tagent capability gate.

The policy kernel lives in Rust (`tagent decide`). This module gives Python
callers a `CapabilityGate` with the `before_tool_call` shape used by pi,
Hermes and OpenClaw, backed by that binary, and an AgentDojo pipeline element
that drops tool calls the gate refuses.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from collections.abc import Callable, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

__all__ = [
    "CapabilityGate",
    "Decision",
    "HookResult",
    "Requirement",
    "TagentClient",
    "ToolCall",
    "find_tagent",
    "make_gate_element",
]


def find_tagent() -> str | None:
    """Locate the `tagent` binary: `$TAGENT_BIN`, then PATH, then a local build."""
    env = os.environ.get("TAGENT_BIN")
    if env and Path(env).is_file():
        return env
    on_path = shutil.which("tagent")
    if on_path:
        return on_path
    here = Path(__file__).resolve()
    for root in here.parents:
        for profile in ("debug", "release"):
            cand = root / "target" / profile / "tagent"
            if cand.is_file():
                return str(cand)
    return None


@dataclass(frozen=True)
class Requirement:
    capability: str
    scope: str | None = None
    evidence: str = ""

    def to_json(self) -> dict[str, Any]:
        d: dict[str, Any] = {"capability": self.capability, "evidence": self.evidence}
        if self.scope is not None:
            d["scope"] = self.scope
        return d


@dataclass(frozen=True)
class Decision:
    allowed: bool
    reasons: tuple[str, ...] = ()


@dataclass(frozen=True)
class ToolCall:
    name: str
    arguments: dict[str, Any] = field(default_factory=dict)
    call_id: str | None = None


@dataclass(frozen=True)
class HookResult:
    block: bool = False
    reason: str | None = None


class TagentClient:
    """Thin wrapper over `tagent decide`."""

    def __init__(self, binary: str | None = None) -> None:
        found = binary or find_tagent()
        if found is None:
            raise FileNotFoundError(
                "tagent binary not found; set TAGENT_BIN or run `cargo build -p tagent-cli`"
            )
        self.binary: str = found

    def decide(self, requirements: Sequence[Requirement], policy: dict[str, Any]) -> Decision:
        payload = json.dumps(
            {"requirements": [r.to_json() for r in requirements], "policy": policy}
        )
        proc = subprocess.run(
            [self.binary, "decide"],
            input=payload,
            capture_output=True,
            text=True,
            check=False,
        )
        if proc.returncode not in (0, 2):
            raise RuntimeError(f"tagent decide failed ({proc.returncode}): {proc.stderr.strip()}")
        out = json.loads(proc.stdout)
        return Decision(allowed=bool(out["allowed"]), reasons=tuple(out.get("reasons", ())))


ToolSignature = Callable[[ToolCall], list[Requirement]]


@dataclass
class CapabilityGate:
    """Blocks tool calls whose requirements the policy does not cover.

    Untyped tools (no signature registered) are blocked: an action without a
    type does not run.
    """

    policy: dict[str, Any]
    client: TagentClient = field(default_factory=TagentClient)
    signatures: dict[str, ToolSignature] = field(default_factory=dict)
    audit: list[tuple[ToolCall, Decision]] = field(default_factory=list)

    def register(self, tool_name: str, signature: ToolSignature) -> None:
        self.signatures[tool_name] = signature

    def decide(self, call: ToolCall) -> Decision:
        sig = self.signatures.get(call.name)
        if sig is None:
            decision = Decision(False, (f"tool {call.name!r} has no capability signature",))
        else:
            decision = self.client.decide(sig(call), self.policy)
        self.audit.append((call, decision))
        return decision

    def __call__(self, call: ToolCall) -> HookResult:
        d = self.decide(call)
        return HookResult() if d.allowed else HookResult(block=True, reason="; ".join(d.reasons))


def make_gate_element(gate: CapabilityGate):
    """Build an AgentDojo `BasePipelineElement` that applies the gate.

    Place it between the LLM and the `ToolsExecutor`. Blocked tool calls are
    removed from the assistant message and answered with a synthetic tool
    result that explains the refusal, so the model can recover but the effect
    never happens. Requires the `agentdojo` extra.
    """
    from agentdojo.agent_pipeline.base_pipeline_element import BasePipelineElement
    from agentdojo.functions_runtime import EmptyEnv
    from agentdojo.types import ChatAssistantMessage, ChatToolResultMessage

    class CapabilityGateElement(BasePipelineElement):  # type: ignore[misc,valid-type]
        name = "tagent.CapabilityGateElement"

        def __init__(self, gate: CapabilityGate) -> None:
            self.gate = gate

        def query(self, query, runtime, env=None, messages=(), extra_args=None):
            env = env if env is not None else EmptyEnv()
            extra_args = extra_args or {}
            messages = list(messages)
            if not messages or messages[-1]["role"] != "assistant":
                return query, runtime, env, messages, extra_args
            last = messages[-1]
            kept, refusals = [], []
            for tc in last.get("tool_calls") or []:
                result = self.gate(
                    ToolCall(name=tc.function, arguments=dict(tc.args), call_id=tc.id)
                )
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
