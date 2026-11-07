"""Attestations: the only thing the admission gate ever reads.

A verifier plugin (Lean 4, h5i, Verus/PDD Kit, a test runner, a scanner)
turns an artifact and a property into an attestation. The gate classifies
attestations into three tiers:

* ``VERIFIED``: result is ``proved`` and the TCB contains nothing outside
  ``LEAN_ONLY_TCB``. Accepted mechanically.
* ``VERIFIED_WITH_TCB``: result is ``proved`` but the TCB names another
  prover (Verus, Z3, ...). Accepted only when the property mapping for that
  verifier has been approved by a human; the TCB is surfaced to users.
* ``NOT_VERIFIED``: ``tested`` / ``scanned`` / ``failed``. Never confers
  *verified*; usable as supporting evidence only.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import asdict, dataclass, field
from enum import StrEnum
from typing import Any

SHA256_HEX_LEN = 64

LEAN_ONLY_TCB: frozenset[str] = frozenset({"lean4", "lake", "mathlib", "aeneas", "charon", "rustc"})


class Result(StrEnum):
    PROVED = "proved"
    TESTED = "tested"
    SCANNED = "scanned"
    FAILED = "failed"


class Tier(StrEnum):
    VERIFIED = "verified"
    VERIFIED_WITH_TCB = "verified-with-tcb"
    NOT_VERIFIED = "not-verified"


@dataclass(frozen=True)
class Artifact:
    sha256: str
    kind: str  # "skill" | "kernel" | "crate" | ...


@dataclass(frozen=True)
class Property:
    id: str  # e.g. "inv.v0.no-exfiltration"
    statement_sha256: str  # hash of the Lean statement this property refers to


@dataclass(frozen=True)
class Verifier:
    name: str
    version: str
    backend: str  # e.g. "lean4", "lean4+aeneas", "verus+z3", "pytest"


@dataclass(frozen=True)
class Proof:
    sha256: str
    format: str  # "lean-olean" | "verus-log" | "junit" | ...


@dataclass
class Attestation:
    artifact: Artifact
    property: Property
    verifier: Verifier
    result: Result
    tcb: tuple[str, ...]
    assumptions: tuple[str, ...] = ()
    proof: Proof | None = None
    extra: dict[str, Any] = field(default_factory=dict)

    # -- (de)serialisation -------------------------------------------------

    def to_dict(self) -> dict[str, Any]:
        d = asdict(self)
        d["result"] = self.result.value
        d["tcb"] = list(self.tcb)
        d["assumptions"] = list(self.assumptions)
        return d

    def to_json(self) -> str:
        return json.dumps(self.to_dict(), sort_keys=True, ensure_ascii=False, indent=2)

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> Attestation:
        proof = d.get("proof")
        att = cls(
            artifact=Artifact(**d["artifact"]),
            property=Property(**d["property"]),
            verifier=Verifier(**d["verifier"]),
            result=Result(d["result"]),
            tcb=tuple(d.get("tcb", ())),
            assumptions=tuple(d.get("assumptions", ())),
            proof=Proof(**proof) if proof else None,
            extra=dict(d.get("extra", {})),
        )
        att.validate()
        return att

    @classmethod
    def from_json(cls, s: str) -> Attestation:
        return cls.from_dict(json.loads(s))

    # -- validation and classification ------------------------------------

    def validate(self) -> None:
        for label, h in (
            ("artifact.sha256", self.artifact.sha256),
            ("property.statement_sha256", self.property.statement_sha256),
        ):
            _require_sha256(label, h)
        if self.proof is not None:
            _require_sha256("proof.sha256", self.proof.sha256)
        if self.result is Result.PROVED and self.proof is None:
            raise ValueError("a 'proved' attestation must carry a proof hash")
        if self.result is Result.PROVED and not self.tcb:
            raise ValueError("a 'proved' attestation must declare its TCB")

    def tier(self, *, approved_backends: frozenset[str] = frozenset()) -> Tier:
        """Classify this attestation.

        ``approved_backends`` lists verifier backends whose property mapping a
        human has approved (e.g. ``{"verus+z3"}``). Without approval, evidence
        from a non-Lean TCB is not *verified*.
        """
        if self.result is not Result.PROVED:
            return Tier.NOT_VERIFIED
        foreign = set(self.tcb) - LEAN_ONLY_TCB
        if not foreign:
            return Tier.VERIFIED
        if self.verifier.backend in approved_backends:
            return Tier.VERIFIED_WITH_TCB
        return Tier.NOT_VERIFIED


def _require_sha256(label: str, value: str) -> None:
    if len(value) != SHA256_HEX_LEN or any(c not in "0123456789abcdef" for c in value):
        raise ValueError(f"{label} must be a lowercase hex sha256, got {value!r}")


def sha256_of_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_of_text(text: str) -> str:
    return sha256_of_bytes(text.encode("utf-8"))
