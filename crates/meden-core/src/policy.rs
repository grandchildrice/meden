//! Policies and decisions.
//!
//! A policy grants capabilities (optionally scoped by a glob) and may deny
//! capabilities outright. [`check`] compares a requirement set against a
//! policy and returns a decision with one reason per violation. Deny always
//! wins over grant.
//!
//! Specification (`MedenSpec.Policy`):
//!
//! * `covers g r  :=  g.cap = r.cap ∧ (g.scope = none ∨ ∃ s, r.scope = some s ∧ glob g.scope s)`
//! * `violates p r :=  r.cap ∈ p.denies ∨ ¬ ∃ g ∈ p.grants, covers g r`
//! * `allowed p rs :=  ∀ r ∈ rs, ¬ violates p r`

use serde::{Deserialize, Serialize};

use crate::capability::{parse_label, Capability, Requirement, UnknownCapability};
use crate::scope::Scope;

/// A granted capability, optionally restricted to scopes matching a glob.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "GrantRepr", into = "GrantRepr")]
pub struct Grant {
    /// The capability granted.
    pub capability: Capability,
    /// Glob the requirement's scope must match; `None` grants it unscoped.
    pub scope: Option<Scope>,
}

impl Grant {
    /// A grant without scope.
    pub fn new(capability: Capability, scope: Option<&str>) -> Self {
        Self {
            capability,
            scope: scope.map(Scope::from),
        }
    }

    /// Parse `cap` or `cap[glob]`.
    pub fn parse(label: &str) -> Result<Self, UnknownCapability> {
        let (capability, scope) = parse_label(label)?;
        Ok(Self { capability, scope })
    }

    /// Does this grant cover the requirement? See [`covers_ref`].
    pub fn covers(&self, req: &Requirement) -> bool {
        covers_ref(self, req)
    }

    /// `cap` or `cap[glob]`.
    pub fn label(&self) -> String {
        match &self.scope {
            Some(s) => format!("{}[{}]", self.capability, s),
            None => self.capability.to_string(),
        }
    }
}

/// Serialized form: either the label string or an object.
#[derive(Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
enum GrantRepr {
    Label(String),
    Object {
        capability: Capability,
        #[serde(default)]
        scope: Option<Scope>,
    },
}

impl TryFrom<GrantRepr> for Grant {
    type Error = UnknownCapability;

    fn try_from(r: GrantRepr) -> Result<Self, Self::Error> {
        match r {
            GrantRepr::Object { capability, scope } => Ok(Grant { capability, scope }),
            GrantRepr::Label(s) => Grant::parse(&s),
        }
    }
}

impl From<Grant> for GrantRepr {
    fn from(g: Grant) -> Self {
        GrantRepr::Label(g.label())
    }
}

/// Does the grant cover the requirement? (`MedenSpec.Grant.covers`)
///
/// An unscoped grant covers every scope. A scoped grant covers only scoped
/// requirements whose scope matches the glob: an unscoped requirement asks
/// for the whole capability and needs an unscoped grant.
pub fn covers_ref(g: &Grant, r: &Requirement) -> bool {
    if g.capability != r.capability {
        return false;
    }
    match (&g.scope, &r.scope) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(pat), Some(s)) => pat.matches(s),
    }
}

/// Is the requirement violated: denied, or covered by no grant?
/// (`MedenSpec.Policy.violates`)
pub fn violates_ref(denies: &[Capability], grants: &[Grant], r: &Requirement) -> bool {
    refusal_ref(denies, grants, r).is_some()
}

/// Are all requirements allowed? (`MedenSpec.Policy.allowed`)
pub fn allowed_ref(denies: &[Capability], grants: &[Grant], reqs: &[Requirement]) -> bool {
    evaluate_ref(denies, grants, reqs).allowed
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    Denied,
    NotGranted,
}

fn refusal_ref(denies: &[Capability], grants: &[Grant], req: &Requirement) -> Option<Refusal> {
    if denies.contains(&req.capability) {
        Some(Refusal::Denied)
    } else if !grants_ref(grants, req) {
        Some(Refusal::NotGranted)
    } else {
        None
    }
}

// PolicyRefinement.grants_loop_refines proves index and increment bounds.
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
fn grants_ref(grants: &[Grant], req: &Requirement) -> bool {
    let mut index = 0;
    while index < grants.len() {
        if covers_ref(&grants[index], req) {
            return true;
        }
        index += 1;
    }
    false
}

struct Evaluation {
    allowed: bool,
    refusals: Vec<Option<Refusal>>,
}

// The authority bit and per-input refusals come from the same evaluation.
// PolicyRefinement.evaluate_loop_refines proves index/increment/vector bounds.
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
fn evaluate_ref(denies: &[Capability], grants: &[Grant], reqs: &[Requirement]) -> Evaluation {
    let mut allowed = true;
    let mut refusals = Vec::new();
    let mut index = 0;
    while index < reqs.len() {
        let refusal = refusal_ref(denies, grants, &reqs[index]);
        if refusal.is_some() {
            allowed = false;
        }
        refusals.push(refusal);
        index += 1;
    }
    Evaluation { allowed, refusals }
}

/// A named set of grants and denies.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// Human-readable name, echoed in reasons.
    pub name: String,
    /// Granted capabilities.
    #[serde(default)]
    pub grants: Vec<Grant>,
    /// Denied capabilities. A denied capability is never allowed, whatever
    /// the grants say.
    #[serde(default)]
    pub denies: Vec<Capability>,
}

impl Policy {
    /// Build a policy from grant labels and denied capability names.
    pub fn from_labels(
        name: &str,
        grants: &[&str],
        denies: &[&str],
    ) -> Result<Self, UnknownCapability> {
        Ok(Self {
            name: name.to_owned(),
            grants: grants
                .iter()
                .map(|g| Grant::parse(g))
                .collect::<Result<_, _>>()?,
            denies: denies.iter().map(|d| d.parse()).collect::<Result<_, _>>()?,
        })
    }

    /// Is the capability denied?
    pub fn denies(&self, cap: Capability) -> bool {
        self.denies.contains(&cap)
    }

    /// Does some grant cover the requirement?
    pub fn grants(&self, req: &Requirement) -> bool {
        grants_ref(&self.grants, req)
    }
}

/// The outcome of checking requirements against a policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    /// `true` when every requirement is granted and none is denied.
    pub allowed: bool,
    /// One line per violated requirement, in label order. Empty when allowed.
    #[serde(default)]
    pub reasons: Vec<String>,
}

impl Decision {
    /// The allowing decision.
    pub const ALLOW: Decision = Decision {
        allowed: true,
        reasons: Vec::new(),
    };

    /// `!allowed`.
    pub fn denied(&self) -> bool {
        !self.allowed
    }
}

/// Is the requirement violated under the policy (denied, or not granted)?
///
/// This is the whole decision rule; [`check`] only adds the human-readable
/// reasons. Mirrors `MedenSpec.Policy.violates`.
pub fn violates(rules: &Policy, req: &Requirement) -> bool {
    violates_ref(&rules.denies, &rules.grants, req)
}

/// Are all requirements allowed? Mirrors `MedenSpec.Policy.allowed`.
pub fn allowed(requirements: &[Requirement], rules: &Policy) -> bool {
    evaluate(requirements, rules).allowed
}

fn evaluate(requirements: &[Requirement], rules: &Policy) -> Evaluation {
    evaluate_ref(&rules.denies, &rules.grants, requirements)
}

/// Check a set of requirements against a policy.
///
/// Requirements are considered in the order of their label (then evidence),
/// so the reasons are deterministic whatever order the caller used.
/// `check(rs, p).allowed == allowed(rs, p)` always.
pub fn check(requirements: &[Requirement], policy: &Policy) -> Decision {
    let evaluation = evaluate(requirements, policy);
    let mut rejected: Vec<_> = requirements
        .iter()
        .zip(evaluation.refusals)
        .filter_map(|(req, refusal)| refusal.map(|why| (req, why)))
        .collect();
    rejected.sort_by(|(a, _), (b, _)| {
        a.label()
            .cmp(&b.label())
            .then_with(|| a.evidence.cmp(&b.evidence))
    });
    rejected.dedup();
    let mut reasons = Vec::new();
    for (req, refusal) in rejected {
        if refusal == Refusal::Denied {
            reasons.push(format!(
                "{} is denied by policy {:?} ({})",
                req, policy.name, req.evidence
            ));
        } else {
            reasons.push(format!(
                "{} is not granted by policy {:?} ({})",
                req, policy.name, req.evidence
            ));
        }
    }
    Decision {
        allowed: evaluation.allowed,
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn every_requirement_keeps_its_refusal_and_a_later_allowance_cannot_reset_denial() {
        let rules = Policy {
            name: "mixed".into(),
            grants: vec![
                Grant::new(Capability::FsWrite, Some("日本/*")),
                Grant::new(Capability::EnvRead, Some("PUBLIC_*")),
                Grant::new(Capability::UiControl, None),
                Grant::new(Capability::FsRead, None),
            ],
            denies: vec![Capability::FsRead],
        };
        let mut reqs = vec![
            Requirement::new(Capability::FsWrite, Some("日本/🦀")),
            Requirement::new(Capability::FsRead, None),
            Requirement::new(Capability::SecretRead, Some("日本/🦀")),
            Requirement::new(Capability::FsWrite, None),
            Requirement::new(Capability::EnvRead, Some("PUBLIC_OK")),
            Requirement::new(Capability::Clipboard, None),
            Requirement::new(Capability::UiControl, None),
        ];
        let evaluation = evaluate(&reqs, &rules);
        assert!(!evaluation.allowed);
        assert_eq!(
            evaluation.refusals,
            vec![
                None,
                Some(Refusal::Denied),
                Some(Refusal::NotGranted),
                Some(Refusal::NotGranted),
                None,
                Some(Refusal::NotGranted),
                None,
            ]
        );
        let decision = check(&reqs, &rules);
        assert!(decision.denied());
        assert_eq!(decision.reasons.len(), 4);
        reqs.reverse();
        assert_eq!(check(&reqs, &rules), decision);
    }

    fn policy() -> Policy {
        Policy::from_labels(
            "ws",
            &[
                "fs.read[/workspace/*]",
                "net.egress[*.example.com]",
                "env.read",
            ],
            &["shell.exec"],
        )
        .unwrap_or_default()
    }

    #[test]
    fn unknown_grant_is_a_configuration_error_not_a_hidden_ui_grant() {
        let result =
            serde_json::from_str::<Policy>(r#"{"name":"typo","grants":["fs.raed"],"denies":[]}"#);
        assert!(result.is_err());
    }

    #[test]
    fn misspelled_policy_fields_cannot_broaden_file_authority() {
        for raw in [
            r#"{"name":"typo","grants":[{"capability":"fs.read","scpoe":"/workspace/public/*"}]}"#,
            r#"{"name":"typo","grants":["fs.read"],"denie":["fs.read"]}"#,
        ] {
            let result = serde_json::from_str::<Policy>(raw);
            if let Ok(policy) = &result {
                assert!(
                    !allowed(
                        &[Requirement::new(
                            Capability::FsRead,
                            Some("/workspace/private/key")
                        )],
                        policy
                    ),
                    "malformed policy granted private-file access: {raw}"
                );
            }
            assert!(result.is_err(), "malformed policy accepted: {raw}");
        }
    }

    #[test]
    fn scoped_grant_covers_matching_scope_only() {
        let p = policy();
        assert!(
            check(
                &[Requirement::new(
                    Capability::FsRead,
                    Some("/workspace/a.txt")
                )],
                &p
            )
            .allowed
        );
        assert!(check(
            &[Requirement::new(Capability::FsRead, Some("/etc/passwd"))],
            &p
        )
        .denied());
        // unscoped requirement needs unscoped grant
        assert!(check(&[Requirement::new(Capability::FsRead, None)], &p).denied());
        assert!(check(&[Requirement::new(Capability::EnvRead, Some("HOME"))], &p).allowed);
        assert!(check(&[Requirement::new(Capability::EnvRead, None)], &p).allowed);
    }

    #[test]
    fn deny_wins_and_reasons_are_sorted() {
        let mut p = policy();
        p.grants.push(Grant::new(Capability::ShellExec, None));
        let d = check(
            &[
                Requirement::with_evidence(Capability::ShellExec, None, "setup.sh:2"),
                Requirement::with_evidence(Capability::FsWrite, Some("/x"), "setup.sh:3"),
            ],
            &p,
        );
        assert!(d.denied());
        assert_eq!(d.reasons.len(), 2);
        assert!(d
            .reasons
            .first()
            .is_some_and(|r| r.starts_with("fs.write[/x] is not granted")));
        assert!(d
            .reasons
            .get(1)
            .is_some_and(|r| r.starts_with("shell.exec is denied")));
    }

    #[test]
    fn empty_requirements_are_allowed_by_any_policy() {
        assert_eq!(check(&[], &Policy::default()), Decision::ALLOW);
    }

    #[test]
    fn policy_json_accepts_labels_and_objects() {
        let p: Policy = serde_json::from_str(
            r#"{"name":"p","grants":["fs.read[/a/*]",{"capability":"net.egress","scope":"x"},{"capability":"env.read"}],"denies":["shell.exec"]}"#,
        )
        .unwrap_or_default();
        assert_eq!(p.grants.len(), 3);
        assert_eq!(
            p.grants.first().map(Grant::label),
            Some("fs.read[/a/*]".into())
        );
        assert_eq!(p.grants.get(2).map(Grant::label), Some("env.read".into()));
        assert!(p.denies(Capability::ShellExec));
        // a typo in a grant label grants nothing
        let p: Policy =
            serde_json::from_str(r#"{"name":"p","grants":["fs.raed"]}"#).unwrap_or_default();
        assert!(check(&[Requirement::new(Capability::FsRead, None)], &p).denied());
    }

    fn arb_cap() -> impl Strategy<Value = Capability> {
        (0..Capability::ALL.len()).prop_map(|i| {
            Capability::ALL
                .get(i)
                .copied()
                .unwrap_or(Capability::UiControl)
        })
    }

    proptest! {
        #[test]
        fn allowed_iff_every_requirement_granted_and_not_denied(
            reqs in proptest::collection::vec((arb_cap(), proptest::option::of("[ab/]{0,4}")), 0..4),
            grants in proptest::collection::vec((arb_cap(), proptest::option::of("[ab/*?]{0,4}")), 0..4),
            denies in proptest::collection::vec(arb_cap(), 0..3),
        ) {
            let reqs: Vec<Requirement> = reqs.iter().map(|(c, s)| Requirement::new(*c, s.as_deref())).collect();
            let policy = Policy { name: "p".into(), grants: grants.iter().map(|(c, s)| Grant::new(*c, s.as_deref())).collect(), denies };
            let d = check(&reqs, &policy);
            let expected = reqs.iter().all(|r| !policy.denies(r.capability) && policy.grants(r));
            prop_assert_eq!(d.allowed, expected);
            prop_assert_eq!(d.reasons.is_empty(), d.allowed);
        }

        #[test]
        fn check_agrees_with_allowed(
            reqs in proptest::collection::vec((arb_cap(), proptest::option::of("[ab/]{0,4}")), 0..4),
            grants in proptest::collection::vec((arb_cap(), proptest::option::of("[ab/*?]{0,4}")), 0..4),
            denies in proptest::collection::vec(arb_cap(), 0..3),
        ) {
            let reqs: Vec<Requirement> = reqs.iter().map(|(c, s)| Requirement::new(*c, s.as_deref())).collect();
            let policy = Policy { name: "p".into(), grants: grants.iter().map(|(c, s)| Grant::new(*c, s.as_deref())).collect(), denies };
            prop_assert_eq!(check(&reqs, &policy).allowed, allowed(&reqs, &policy));
        }

        #[test]
        fn adding_grants_never_turns_allow_into_deny(
            reqs in proptest::collection::vec((arb_cap(), proptest::option::of("[ab/]{0,4}")), 0..4),
            grants in proptest::collection::vec((arb_cap(), proptest::option::of("[ab/*?]{0,4}")), 0..4),
            extra in (arb_cap(), proptest::option::of("[ab/*?]{0,4}")),
        ) {
            let reqs: Vec<Requirement> = reqs.iter().map(|(c, s)| Requirement::new(*c, s.as_deref())).collect();
            let mut policy = Policy { name: "p".into(), grants: grants.iter().map(|(c, s)| Grant::new(*c, s.as_deref())).collect(), denies: vec![] };
            let before = check(&reqs, &policy).allowed;
            policy.grants.push(Grant::new(extra.0, extra.1.as_deref()));
            let after = check(&reqs, &policy).allowed;
            prop_assert!(!before || after);
        }
    }
}
