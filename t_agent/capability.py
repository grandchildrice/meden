"""Capability vocabulary.

A capability names one kind of effect an action can have on the world. Tool
calls and skills are *typed* by the set of capabilities they require; a policy
grants a set of capabilities. The vocabulary is deliberately small and coarse
for the prototype. Finer scopes (a path prefix, a host allowlist) are carried
as ``scope`` strings and compared textually for now.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum


class Capability(StrEnum):
    FS_READ = "fs.read"
    FS_WRITE = "fs.write"
    NET_EGRESS = "net.egress"
    NET_LISTEN = "net.listen"
    SHELL_EXEC = "shell.exec"
    ENV_READ = "env.read"
    SECRET_READ = "secret.read"
    PROCESS_SPAWN = "process.spawn"
    PACKAGE_INSTALL = "package.install"
    CLIPBOARD = "clipboard"
    UI_CONTROL = "ui.control"

    @classmethod
    def parse(cls, value: str) -> Capability:
        try:
            return cls(value)
        except ValueError as exc:
            raise ValueError(f"unknown capability: {value!r}") from exc


@dataclass(frozen=True)
class Requirement:
    """One capability a skill or tool call needs, with an optional scope.

    ``scope`` examples: ``"/home/user/project/**"`` for ``fs.write``,
    ``"api.github.com"`` for ``net.egress``, ``"GEMINI_API_KEY"`` for
    ``env.read``. ``None`` means unscoped (the whole capability).
    """

    capability: Capability
    scope: str | None = None
    evidence: str = ""  # where this requirement was found (file:line or field)

    def __str__(self) -> str:
        base = self.capability.value
        return f"{base}[{self.scope}]" if self.scope else base


def requirements_to_strings(reqs: set[Requirement]) -> list[str]:
    return sorted(str(r) for r in reqs)
