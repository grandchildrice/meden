import pytest

from t_agent.attestation import (
    Artifact,
    Attestation,
    Proof,
    Property,
    Result,
    Tier,
    Verifier,
    sha256_of_text,
)

H = sha256_of_text("x")


def _att(
    result: Result, tcb: tuple[str, ...], backend: str = "lean4", proof: bool = True
) -> Attestation:
    return Attestation(
        artifact=Artifact(sha256=H, kind="skill"),
        property=Property(id="inv.v0.no-exfiltration", statement_sha256=H),
        verifier=Verifier(name="test", version="0", backend=backend),
        result=result,
        tcb=tcb,
        proof=Proof(sha256=H, format="lean-olean") if proof else None,
    )


def test_lean_only_proof_is_verified():
    assert _att(Result.PROVED, ("lean4", "lake")).tier() is Tier.VERIFIED


def test_h5i_style_lean_via_aeneas_is_verified():
    a = _att(Result.PROVED, ("lean4", "aeneas", "charon", "rustc"), backend="lean4+aeneas")
    assert a.tier() is Tier.VERIFIED


def test_verus_proof_needs_human_approved_mapping():
    a = _att(Result.PROVED, ("verus", "z3", "rustc"), backend="verus+z3")
    assert a.tier() is Tier.NOT_VERIFIED
    assert a.tier(approved_backends=frozenset({"verus+z3"})) is Tier.VERIFIED_WITH_TCB


def test_tests_and_scans_never_verify():
    assert _att(Result.TESTED, ("pytest",), backend="pytest").tier() is Tier.NOT_VERIFIED
    assert _att(Result.SCANNED, ("virustotal",), backend="vt").tier() is Tier.NOT_VERIFIED


def test_proved_requires_proof_hash_and_tcb():
    with pytest.raises(ValueError):
        _att(Result.PROVED, ("lean4",), proof=False).validate()
    with pytest.raises(ValueError):
        _att(Result.PROVED, ()).validate()


def test_json_round_trip():
    a = _att(Result.PROVED, ("lean4",))
    b = Attestation.from_json(a.to_json())
    assert b == a


def test_rejects_bad_hash():
    with pytest.raises(ValueError):
        Attestation.from_dict(
            {
                **_att(Result.TESTED, ("pytest",)).to_dict(),
                "artifact": {"sha256": "abc", "kind": "skill"},
            }
        )
