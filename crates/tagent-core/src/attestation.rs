//! Attestations: the only thing the admission gate ever reads.
//!
//! A verifier plugin (Lean 4, h5i, Verus/PDD Kit, a test runner, a scanner)
//! turns an artifact and a property into an attestation. The gate classifies
//! attestations into three tiers (`TAgentSpec.Attestation`):
//!
//! * [`Tier::Verified`]: result is `proved` and the TCB contains nothing
//!   outside [`LEAN_ONLY_TCB`]. Accepted mechanically.
//! * [`Tier::VerifiedWithTcb`]: result is `proved` but the TCB names another
//!   prover (Verus, Z3, ...). Accepted only when the property mapping for that
//!   verifier backend has been approved by a human; the TCB is surfaced.
//! * [`Tier::NotVerified`]: `tested`, `scanned` or `failed`. Never confers
//!   *verified*; usable as supporting evidence only.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Components a Lean-only attestation may trust.
pub const LEAN_ONLY_TCB: [&str; 6] = ["lean4", "lake", "mathlib", "aeneas", "charon", "rustc"];

/// Is `component` part of the Lean-only trusted computing base?
pub fn in_lean_only_tcb(component: &str) -> bool {
    LEAN_ONLY_TCB.contains(&component)
}

/// What the verifier concluded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Result {
    /// A proof was checked.
    Proved,
    /// Tests passed.
    Tested,
    /// A scanner found nothing.
    Scanned,
    /// The verifier rejected the artifact.
    Failed,
}

/// The gate's classification of an attestation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tier {
    /// Proved with a Lean-only TCB.
    Verified,
    /// Proved with a foreign prover whose mapping a human approved.
    VerifiedWithTcb,
    /// Everything else.
    NotVerified,
}

impl Tier {
    /// Does this tier count as verified for admission?
    pub const fn is_verified(self) -> bool {
        matches!(self, Tier::Verified | Tier::VerifiedWithTcb)
    }
}

/// The thing that was verified.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    /// Lowercase hex SHA-256 of the artifact (see [`crate::digest`]).
    pub sha256: String,
    /// `skill`, `kernel`, `crate`, ...
    pub kind: String,
}

/// The property that was verified about the artifact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Property {
    /// Identifier in the standard invariant catalogue, e.g. `inv.v0.no-exfiltration`.
    pub id: String,
    /// SHA-256 of the formal statement this property refers to.
    pub statement_sha256: String,
}

/// Who verified.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verifier {
    /// Tool name.
    pub name: String,
    /// Tool version.
    pub version: String,
    /// Proof backend, e.g. `lean4`, `lean4+aeneas`, `verus+z3`, `pytest`.
    pub backend: String,
}

/// The proof object.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proof {
    /// SHA-256 of the proof object.
    pub sha256: String,
    /// `lean-olean`, `verus-log`, `junit`, ...
    pub format: String,
}

/// Evidence produced by a verifier plugin.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attestation {
    /// The artifact.
    pub artifact: Artifact,
    /// The property.
    pub property: Property,
    /// The verifier.
    pub verifier: Verifier,
    /// The verdict.
    pub result: Result,
    /// Every component the verdict trusts.
    #[serde(default)]
    pub tcb: Vec<String>,
    /// Stated assumptions, free text.
    #[serde(default)]
    pub assumptions: Vec<String>,
    /// The proof object, required when `result` is `proved`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proof: Option<Proof>,
}

/// Why an attestation is malformed.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AttestationError {
    /// A hash field is not 64 lowercase hex characters.
    #[error("{0} must be a lowercase hex sha256")]
    BadHash(&'static str),
    /// A proved attestation has no proof hash.
    #[error("a 'proved' attestation must carry a proof hash")]
    MissingProof,
    /// A proved attestation has an empty TCB.
    #[error("a 'proved' attestation must declare its TCB")]
    MissingTcb,
}

/// Is `s` a lowercase hex SHA-256?
pub fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl Attestation {
    /// Check well-formedness.
    pub fn validate(&self) -> std::result::Result<(), AttestationError> {
        if !is_sha256_hex(&self.artifact.sha256) {
            return Err(AttestationError::BadHash("artifact.sha256"));
        }
        if !is_sha256_hex(&self.property.statement_sha256) {
            return Err(AttestationError::BadHash("property.statement_sha256"));
        }
        if let Some(p) = &self.proof {
            if !is_sha256_hex(&p.sha256) {
                return Err(AttestationError::BadHash("proof.sha256"));
            }
        }
        if self.result == Result::Proved {
            if self.proof.is_none() {
                return Err(AttestationError::MissingProof);
            }
            if self.tcb.is_empty() {
                return Err(AttestationError::MissingTcb);
            }
        }
        Ok(())
    }

    /// Classify. `approved_backends` lists verifier backends whose property
    /// mapping a human has approved (e.g. `verus+z3`).
    pub fn tier(&self, approved_backends: &[String]) -> Tier {
        tier_of(
            self.result,
            &self.tcb,
            &self.verifier.backend,
            approved_backends,
        )
    }

    /// Parse and validate JSON.
    pub fn from_json(s: &str) -> std::result::Result<Self, String> {
        let att: Attestation = serde_json::from_str(s).map_err(|e| e.to_string())?;
        att.validate().map_err(|e| e.to_string())?;
        Ok(att)
    }
}

/// The tier rule, generic over how the strings are owned.
///
/// This is the whole classification; [`Attestation::tier`] only supplies the
/// fields. Mirrors `TAgentSpec.tierOf`.
pub fn tier_rule<S: AsRef<str>, B: AsRef<str>>(
    result: Result,
    tcb: &[S],
    backend: &str,
    approved_backends: &[B],
) -> Tier {
    if result != Result::Proved {
        return Tier::NotVerified;
    }
    if tcb.iter().all(|c| in_lean_only_tcb(c.as_ref())) {
        return Tier::Verified;
    }
    if approved_backends.iter().any(|b| b.as_ref() == backend) {
        return Tier::VerifiedWithTcb;
    }
    Tier::NotVerified
}

/// [`tier_rule`] on owned strings.
pub fn tier_of(
    result: Result,
    tcb: &[String],
    backend: &str,
    approved_backends: &[String],
) -> Tier {
    tier_rule(result, tcb, backend, approved_backends)
}

/// [`tier_rule`] on borrowed strings, so the model-checking harness can hand
/// it stack data.
pub fn tier_of_ref(
    result: Result,
    tcb: &[&str],
    backend: &str,
    approved_backends: &[&str],
) -> Tier {
    tier_rule(result, tcb, backend, approved_backends)
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: &str = "2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881";

    fn att(result: Result, tcb: &[&str], backend: &str, proof: bool) -> Attestation {
        Attestation {
            artifact: Artifact {
                sha256: H.into(),
                kind: "skill".into(),
            },
            property: Property {
                id: "inv.v0.no-exfiltration".into(),
                statement_sha256: H.into(),
            },
            verifier: Verifier {
                name: "t".into(),
                version: "0".into(),
                backend: backend.into(),
            },
            result,
            tcb: tcb.iter().map(|s| (*s).to_owned()).collect(),
            assumptions: vec![],
            proof: proof.then(|| Proof {
                sha256: H.into(),
                format: "lean-olean".into(),
            }),
        }
    }

    #[test]
    fn tiers() {
        assert_eq!(
            att(Result::Proved, &["lean4", "lake"], "lean4", true).tier(&[]),
            Tier::Verified
        );
        assert_eq!(
            att(
                Result::Proved,
                &["lean4", "aeneas", "charon", "rustc"],
                "lean4+aeneas",
                true
            )
            .tier(&[]),
            Tier::Verified
        );
        let verus = att(Result::Proved, &["verus", "z3", "rustc"], "verus+z3", true);
        assert_eq!(verus.tier(&[]), Tier::NotVerified);
        assert_eq!(verus.tier(&["verus+z3".to_owned()]), Tier::VerifiedWithTcb);
        assert_eq!(
            att(Result::Tested, &["pytest"], "pytest", false).tier(&["pytest".into()]),
            Tier::NotVerified
        );
        assert_eq!(
            att(Result::Failed, &["lean4"], "lean4", true).tier(&[]),
            Tier::NotVerified
        );
    }

    #[test]
    fn owned_and_borrowed_tier_rules_agree() {
        for result in [
            Result::Proved,
            Result::Tested,
            Result::Scanned,
            Result::Failed,
        ] {
            for tcb in [vec!["lean4"], vec!["z3", "verus"], vec![]] {
                for approved in [vec![], vec!["verus+z3"]] {
                    let owned_tcb: Vec<String> = tcb.iter().map(|s| (*s).to_owned()).collect();
                    let owned_ap: Vec<String> = approved.iter().map(|s| (*s).to_owned()).collect();
                    assert_eq!(
                        tier_of(result, &owned_tcb, "verus+z3", &owned_ap),
                        tier_of_ref(result, &tcb, "verus+z3", &approved)
                    );
                }
            }
        }
    }

    #[test]
    fn validation() {
        assert_eq!(
            att(Result::Proved, &["lean4"], "lean4", false).validate(),
            Err(AttestationError::MissingProof)
        );
        assert_eq!(
            att(Result::Proved, &[], "lean4", true).validate(),
            Err(AttestationError::MissingTcb)
        );
        assert_eq!(att(Result::Tested, &[], "pytest", false).validate(), Ok(()));
        let mut bad = att(Result::Tested, &[], "pytest", false);
        bad.artifact.sha256 = "ABC".into();
        assert_eq!(
            bad.validate(),
            Err(AttestationError::BadHash("artifact.sha256"))
        );
    }

    #[test]
    fn json_round_trip() {
        let a = att(Result::Proved, &["lean4"], "lean4", true);
        let s = serde_json::to_string(&a).unwrap_or_default();
        assert!(s.contains("\"result\":\"proved\""));
        assert_eq!(Attestation::from_json(&s), Ok(a));
        assert!(Attestation::from_json("{}").is_err());
    }
}
