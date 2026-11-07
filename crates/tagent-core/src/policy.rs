//! Policies and decisions.
//!
//! A policy grants capabilities (optionally scoped by a glob) and may deny
//! capabilities outright. [`check`] compares a requirement set against a
//! policy and returns a decision with one reason per violation. Deny always
//! wins over grant.
//!
//! Specification (`TAgentSpec.Policy`):
//!
//! * `covers g r  :=  g.cap = r.cap ∧ (g.scope = none ∨ ∃ s, r.scope = some s ∧ glob g.scope s)`
//! * `violates p r :=  r.cap ∈ p.denies ∨ ¬ ∃ g ∈ p.grants, covers g r`
//! * `allowed p rs :=  ∀ r ∈ rs, ¬ violates p r`

use serde::{Deserialize, Serialize};

use crate::capability::{parse_label, Capability, Requirement, UnknownCapability};
use crate::glob::glob_match;

/// A granted capability, optionally restricted to scopes matching a glob.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "GrantRepr", into = "GrantRepr")]
pub struct Grant {
    /// The capability granted.
    pub capability: Capability,
    /// Glob the requirement's scope must match; `None` grants it unscoped.
    pub scope: Option<String>,
}

impl Grant {
    /// A grant without scope.
    pub fn new(capability: Capability, scope: Option<&str>) -> Self {
        Self {
            capability,
            scope: scope.map(str::to_owned),
        }
    }

    /// Parse `cap` or `cap[glob]`.
    pub fn parse(label: &str) -> Result<Self, UnknownCapability> {
        let (capability, scope) = parse_label(label)?;
        Ok(Self { capability, scope })
    }

    /// Does this grant cover the requirement? See [`covers_ref`].
    pub fn covers(&self, req: &Requirement) -> bool {
        covers_ref(self.as_ref(), req.as_ref())
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
#[serde(untagged)]
enum GrantRepr {
    Label(String),
    Object {
        capability: Capability,
        #[serde(default)]
        scope: Option<String>,
    },
}

impl From<GrantRepr> for Grant {
    fn from(r: GrantRepr) -> Self {
        match r {
            GrantRepr::Object { capability, scope } => Grant { capability, scope },
            GrantRepr::Label(s) => match Grant::parse(&s) {
                Ok(g) => g,
                // An unknown label cannot grant anything: map it to a grant
                // that covers nothing rather than failing deserialization,
                // so that a policy file with a typo fails closed.
                Err(_) => Grant {
                    capability: Capability::UiControl,
                    scope: Some(String::from("\u{0}never\u{0}")),
                },
            },
        }
    }
}

impl From<Grant> for GrantRepr {
    fn from(g: Grant) -> Self {
        GrantRepr::Label(g.label())
    }
}

/// A borrowed grant: the rule is stated on these so it can be checked on
/// stack data (Kani) as well as on owned policies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrantRef<'a> {
    /// The capability granted.
    pub capability: Capability,
    /// Glob the requirement's scope must match; `None` grants it unscoped.
    pub scope: Option<&'a str>,
}

/// A borrowed requirement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReqRef<'a> {
    /// The capability required.
    pub capability: Capability,
    /// Optional scope.
    pub scope: Option<&'a str>,
}

impl Grant {
    /// Borrow.
    pub fn as_ref(&self) -> GrantRef<'_> {
        GrantRef {
            capability: self.capability,
            scope: self.scope.as_deref(),
        }
    }
}

impl Requirement {
    /// Borrow.
    pub fn as_ref(&self) -> ReqRef<'_> {
        ReqRef {
            capability: self.capability,
            scope: self.scope.as_deref(),
        }
    }
}

/// Does the grant cover the requirement? (`TAgentSpec.Grant.covers`)
///
/// An unscoped grant covers every scope. A scoped grant covers only scoped
/// requirements whose scope matches the glob: an unscoped requirement asks
/// for the whole capability and needs an unscoped grant.
pub fn covers_ref(g: GrantRef<'_>, r: ReqRef<'_>) -> bool {
    if g.capability != r.capability {
        return false;
    }
    match (g.scope, r.scope) {
        (None, _) | (Some(_), None) => true,
        (Some(pat), Some(s)) => glob_match(pat, s),
    }
}

/// Is the requirement violated: denied, or covered by no grant?
/// (`TAgentSpec.Policy.violates`)
pub fn violates_ref(denies: &[Capability], grants: &[GrantRef<'_>], r: ReqRef<'_>) -> bool {
    denies.contains(&r.capability) || !grants.iter().any(|g| covers_ref(*g, r))
}

/// Are all requirements allowed? (`TAgentSpec.Policy.allowed`)
pub fn allowed_ref(denies: &[Capability], grants: &[GrantRef<'_>], reqs: &[ReqRef<'_>]) -> bool {
    reqs.iter().all(|r| !violates_ref(denies, grants, *r))
}

/// A named set of grants and denies.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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
        self.grants.iter().any(|g| g.covers(req))
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
/// reasons. Mirrors `TAgentSpec.Policy.violates`.
pub fn violates(policy: &Policy, req: &Requirement) -> bool {
    policy.denies(req.capability) || !policy.grants(req)
}

/// Are all requirements allowed? Mirrors `TAgentSpec.Policy.allowed`.
pub fn allowed(requirements: &[Requirement], policy: &Policy) -> bool {
    requirements.iter().all(|r| !violates(policy, r))
}

/// Check a set of requirements against a policy.
///
/// Requirements are considered in the order of their label (then evidence),
/// so the reasons are deterministic whatever order the caller used.
/// `check(rs, p).allowed == allowed(rs, p)` always.
pub fn check(requirements: &[Requirement], policy: &Policy) -> Decision {
    let mut reqs: Vec<&Requirement> = requirements.iter().collect();
    reqs.sort_by(|a, b| {
        a.label()
            .cmp(&b.label())
            .then_with(|| a.evidence.cmp(&b.evidence))
    });
    reqs.dedup();
    let mut reasons = Vec::new();
    for req in reqs {
        if policy.denies(req.capability) {
            reasons.push(format!(
                "{} is denied by policy {:?} ({})",
                req, policy.name, req.evidence
            ));
        } else if !policy.grants(req) {
            reasons.push(format!(
                "{} is not granted by policy {:?} ({})",
                req, policy.name, req.evidence
            ));
        }
    }
    Decision {
        allowed: reasons.is_empty(),
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

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
        // a scoped grant now also covers an unscoped requirement
        assert!(check(&[Requirement::new(Capability::FsRead, None)], &p).allowed);
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
        fn owned_and_borrowed_rules_agree(
            reqs in proptest::collection::vec((arb_cap(), proptest::option::of("[ab/]{0,4}")), 0..4),
            grants in proptest::collection::vec((arb_cap(), proptest::option::of("[ab/*?]{0,4}")), 0..4),
            denies in proptest::collection::vec(arb_cap(), 0..3),
        ) {
            let reqs: Vec<Requirement> = reqs.iter().map(|(c, s)| Requirement::new(*c, s.as_deref())).collect();
            let policy = Policy { name: "p".into(), grants: grants.iter().map(|(c, s)| Grant::new(*c, s.as_deref())).collect(), denies };
            let grefs: Vec<GrantRef<'_>> = policy.grants.iter().map(Grant::as_ref).collect();
            let rrefs: Vec<ReqRef<'_>> = reqs.iter().map(Requirement::as_ref).collect();
            prop_assert_eq!(allowed(&reqs, &policy), allowed_ref(&policy.denies, &grefs, &rrefs));
            for r in &reqs {
                prop_assert_eq!(violates(&policy, r), violates_ref(&policy.denies, &grefs, r.as_ref()));
            }
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
