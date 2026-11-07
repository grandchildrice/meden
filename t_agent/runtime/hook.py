"""The ``before_tool_call`` contract and the capability gate.

The contract is modelled on pi's ``beforeToolCall`` (and is close to Hermes'
``pre_tool_call`` and OpenClaw's ``before_tool_call``): the hook receives the
tool call about to run and may let it through, block it with a reason the
model will see, or rewrite its arguments. The same gate object can therefore
be mounted on any of those runtimes with a thin adapter.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field
from typing import Any, Protocol

from ..capability import Requirement
from ..policy import Decision, Policy, check


@dataclass(frozen=True)
class ToolCall:
    name: str
    arguments: dict[str, Any] = field(default_factory=dict)
    call_id: str | None = None


@dataclass(frozen=True)
class HookResult:
    block: bool = False
    reason: str | None = None
    arguments: dict[str, Any] | None = None  # rewritten arguments, if any


class BeforeToolCall(Protocol):
    def __call__(self, call: ToolCall) -> HookResult: ...


# A tool signature maps a concrete call to the capabilities it exercises.
# For example a ``send_email`` tool needs ``net.egress`` scoped to the
# recipient domain; ``read_file`` needs ``fs.read`` scoped to the path.
ToolSignature = Callable[[ToolCall], set[Requirement]]


@dataclass
class CapabilityGate:
    """Blocks tool calls whose requirements are not covered by the policy.

    Untyped tools (no signature registered) are blocked: in this runtime an
    action without a type does not run. That is the whole point.
    """

    policy: Policy
    signatures: dict[str, ToolSignature] = field(default_factory=dict)
    audit: list[tuple[ToolCall, Decision]] = field(default_factory=list)

    def register(self, tool_name: str, signature: ToolSignature) -> None:
        self.signatures[tool_name] = signature

    def decide(self, call: ToolCall) -> Decision:
        sig = self.signatures.get(call.name)
        if sig is None:
            decision = Decision(False, (f"tool {call.name!r} has no capability signature",))
        else:
            decision = check(sig(call), self.policy)
        self.audit.append((call, decision))
        return decision

    def __call__(self, call: ToolCall) -> HookResult:
        decision = self.decide(call)
        if decision.allowed:
            return HookResult()
        return HookResult(block=True, reason="; ".join(decision.reasons))
