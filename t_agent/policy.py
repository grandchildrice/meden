"""Policies and decisions.

A policy grants capabilities (optionally scoped) and may deny specific
capabilities outright. ``check`` compares a requirement set against a policy
and returns a decision with one reason per violation. Deny always wins over
grant, mirroring how OpenClaw and Hermes resolve tool permissions.
"""

from __future__ import annotations

import fnmatch
from collections.abc import Sequence
from dataclasses import dataclass, field

from .capability import Capability, Requirement


@dataclass(frozen=True)
class Grant:
    capability: Capability
    scope: str | None = None  # glob; None grants the capability unscoped

    def covers(self, req: Requirement) -> bool:
        if req.capability is not self.capability:
            return False
        if self.scope is None:
            return True
        if req.scope is None:
            return False  # unscoped requirement needs an unscoped grant
        return fnmatch.fnmatchcase(req.scope, self.scope)


@dataclass
class Policy:
    name: str
    grants: set[Grant] = field(default_factory=set)
    denies: set[Capability] = field(default_factory=set)

    @classmethod
    def from_strings(cls, name: str, grants: Sequence[str], denies: Sequence[str] = ()) -> Policy:
        parsed: set[Grant] = set()
        for g in grants:
            if "[" in g and g.endswith("]"):
                cap, scope = g[:-1].split("[", 1)
                parsed.add(Grant(Capability.parse(cap), scope))
            else:
                parsed.add(Grant(Capability.parse(g)))
        return cls(name=name, grants=parsed, denies={Capability.parse(d) for d in denies})


@dataclass(frozen=True)
class Decision:
    allowed: bool
    reasons: tuple[str, ...] = ()

    @property
    def denied(self) -> bool:
        return not self.allowed


def check(requirements: set[Requirement], policy: Policy) -> Decision:
    reasons: list[str] = []
    for req in sorted(requirements, key=str):
        if req.capability in policy.denies:
            reasons.append(f"{req} is denied by policy {policy.name!r} ({req.evidence})")
            continue
        if not any(g.covers(req) for g in policy.grants):
            reasons.append(f"{req} is not granted by policy {policy.name!r} ({req.evidence})")
    return Decision(allowed=not reasons, reasons=tuple(reasons))
