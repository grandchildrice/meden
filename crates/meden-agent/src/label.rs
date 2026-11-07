//! Labels: where a value came from, and who may read it.
//!
//! Every value the interpreter handles carries a [`Label`] made of two
//! independent lattices, following the model in
//! [CaMeL](https://arxiv.org/abs/2503.18813):
//!
//! * **Provenance** ([`Label::sources`]), joined by union. Combining two
//!   values produces data derived from both, so the sources accumulate. This
//!   answers *may this value steer an action?*
//! * **Confidentiality** ([`Label::readers`]), joined by intersection.
//!   Combining two values produces data only those who may read both may
//!   read. This answers *may this value be disclosed to this principal?*
//!
//! The two join in opposite directions, which is why they are separate: data
//! becomes *less* trusted and *less* widely readable as it is combined.
//!
//! A label records facts about where data came from. It never records a
//! decision: whether a given source is trusted for control is a policy
//! question, answered by [`Label::trusted_for_control`] against a set the
//! deployment supplies.
//!
//! Specified in `lean/MedenSpec/Flow.lean`.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Where a piece of data came from.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// The runtime itself: a literal written in a plan, or a value computed
    /// from literals alone.
    Runtime,
    /// The user's own instruction.
    User,
    /// The planning model's own output.
    Assistant,
    /// The output of a tool, by name.
    ///
    /// Whether that tool is trusted is not recorded here. A web fetch and a
    /// password manager both produce `Tool`; the policy says which of them
    /// may steer an action.
    Tool(String),
}

impl Source {
    /// A tool source.
    pub fn tool(name: &str) -> Self {
        Source::Tool(name.to_owned())
    }
}

/// Who may read a value.
///
/// The meet (see [`Readers::meet`]) is intersection, with [`Readers::Public`]
/// as its identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Readers {
    /// Anyone may read this value.
    Public,
    /// Only these principals may read this value.
    Only(BTreeSet<String>),
}

impl Readers {
    /// Only these principals.
    pub fn only<I: IntoIterator<Item = S>, S: Into<String>>(principals: I) -> Self {
        Readers::Only(principals.into_iter().map(Into::into).collect())
    }

    /// The meet: who may read data derived from both.
    ///
    /// `Public` is the identity, so `Public.meet(r) == r`.
    #[must_use]
    pub fn meet(&self, other: &Readers) -> Readers {
        match (self, other) {
            (Readers::Public, r) | (r, Readers::Public) => r.clone(),
            (Readers::Only(a), Readers::Only(b)) => {
                Readers::Only(a.intersection(b).cloned().collect())
            }
        }
    }

    /// May `principal` read this value?
    pub fn allows(&self, principal: &str) -> bool {
        match self {
            Readers::Public => true,
            Readers::Only(s) => s.contains(principal),
        }
    }
}

/// The provenance and confidentiality of one value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    /// Every source that contributed to this value.
    pub sources: BTreeSet<Source>,
    /// Who may read this value.
    pub readers: Readers,
}

impl Label {
    /// A label with the given sources, readable by anyone.
    pub fn public<I: IntoIterator<Item = Source>>(sources: I) -> Self {
        Label {
            sources: sources.into_iter().collect(),
            readers: Readers::Public,
        }
    }

    /// Data the runtime produced itself: a literal in a plan.
    pub fn runtime() -> Self {
        Label::public([Source::Runtime])
    }

    /// Data the user supplied.
    pub fn user() -> Self {
        Label::public([Source::User])
    }

    /// Data the planning model produced.
    pub fn assistant() -> Self {
        Label::public([Source::Assistant])
    }

    /// The join: the label of data derived from both.
    ///
    /// Sources unite and readers intersect, so the result is no more trusted
    /// and no more widely readable than either input.
    #[must_use]
    pub fn join(&self, other: &Label) -> Label {
        Label {
            sources: self.sources.union(&other.sources).cloned().collect(),
            readers: self.readers.meet(&other.readers),
        }
    }

    /// The join of many labels. With none, the result is [`Label::runtime`].
    pub fn join_all<'a, I: IntoIterator<Item = &'a Label>>(labels: I) -> Label {
        labels
            .into_iter()
            .fold(Label::runtime(), |acc, l| acc.join(l))
    }

    /// This label with one more source.
    #[must_use]
    pub fn with_source(&self, source: Source) -> Label {
        let mut sources = self.sources.clone();
        sources.insert(source);
        Label {
            sources,
            readers: self.readers.clone(),
        }
    }

    /// This label restricted to the given readers.
    #[must_use]
    pub fn restrict_to(&self, readers: &Readers) -> Label {
        Label {
            sources: self.sources.clone(),
            readers: self.readers.meet(readers),
        }
    }

    /// May this value steer an action?
    ///
    /// True when every source that contributed is in `trusted`. This is the
    /// rule that stops prompt injection: data fetched from a web page carries
    /// `Tool("fetch")`, so unless the deployment trusts that tool, the value
    /// cannot decide who an email goes to, however convincingly the page
    /// argues that it should.
    pub fn trusted_for_control(&self, trusted: &BTreeSet<Source>) -> bool {
        self.sources.is_subset(trusted)
    }

    /// The sources that are not in `trusted`, for an error message.
    pub fn untrusted_sources(&self, trusted: &BTreeSet<Source>) -> Vec<Source> {
        self.sources.difference(trusted).cloned().collect()
    }

    /// May `principal` read this value?
    pub fn allows_reader(&self, principal: &str) -> bool {
        self.readers.allows(principal)
    }
}

impl Default for Label {
    fn default() -> Self {
        Label::runtime()
    }
}

/// The set of sources a deployment trusts to steer actions.
///
/// `Runtime`, `User` and `Assistant` are the usual baseline: a plan written
/// by the planning model from the user's instruction may act. Tools are added
/// one by one, deliberately.
pub fn baseline_trust() -> BTreeSet<Source> {
    [Source::Runtime, Source::User, Source::Assistant]
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    // Tests fail by panicking; that is their job.
    #![allow(clippy::panic)]

    use super::*;
    use proptest::prelude::*;

    fn fetch() -> Label {
        Label::public([Source::Runtime, Source::tool("fetch")])
    }

    #[test]
    fn join_accumulates_sources_and_narrows_readers() {
        let a = Label {
            sources: [Source::User].into_iter().collect(),
            readers: Readers::only(["alice", "bob"]),
        };
        let b = Label {
            sources: [Source::tool("fetch")].into_iter().collect(),
            readers: Readers::only(["bob", "carol"]),
        };
        let j = a.join(&b);
        assert_eq!(
            j.sources,
            [Source::User, Source::tool("fetch")].into_iter().collect()
        );
        assert_eq!(j.readers, Readers::only(["bob"]));
    }

    #[test]
    fn public_is_the_identity_for_readers() {
        let r = Readers::only(["alice"]);
        assert_eq!(Readers::Public.meet(&r), r);
        assert_eq!(r.meet(&Readers::Public), r);
        assert_eq!(Readers::Public.meet(&Readers::Public), Readers::Public);
    }

    #[test]
    fn untrusted_tool_output_cannot_steer() {
        let trusted = baseline_trust();
        assert!(Label::user().trusted_for_control(&trusted));
        assert!(!fetch().trusted_for_control(&trusted));
        assert_eq!(
            fetch().untrusted_sources(&trusted),
            vec![Source::tool("fetch")]
        );

        // A deployment may trust a specific tool.
        let mut trusted_fetch = trusted.clone();
        trusted_fetch.insert(Source::tool("fetch"));
        assert!(fetch().trusted_for_control(&trusted_fetch));
    }

    #[test]
    fn trust_cannot_be_laundered_by_combining() {
        let trusted = baseline_trust();
        // Joining trusted data onto untrusted data does not make it trusted.
        let combined = Label::user().join(&fetch());
        assert!(!combined.trusted_for_control(&trusted));
    }

    #[test]
    fn join_of_nothing_is_runtime() {
        assert_eq!(Label::join_all([]), Label::runtime());
        let l = Label::user();
        assert_eq!(
            Label::join_all([&l]).sources,
            Label::runtime().join(&l).sources
        );
    }

    fn arb_source() -> impl Strategy<Value = Source> {
        prop_oneof![
            Just(Source::Runtime),
            Just(Source::User),
            Just(Source::Assistant),
            "[a-c]".prop_map(Source::Tool),
        ]
    }

    fn arb_readers() -> impl Strategy<Value = Readers> {
        prop_oneof![
            Just(Readers::Public),
            proptest::collection::btree_set("[a-c]", 0..3).prop_map(Readers::Only),
        ]
    }

    fn arb_label() -> impl Strategy<Value = Label> {
        (
            proptest::collection::btree_set(arb_source(), 0..3),
            arb_readers(),
        )
            .prop_map(|(sources, readers)| Label { sources, readers })
    }

    proptest! {
        #[test]
        fn join_is_commutative(a in arb_label(), b in arb_label()) {
            prop_assert_eq!(a.join(&b), b.join(&a));
        }

        #[test]
        fn join_is_associative(a in arb_label(), b in arb_label(), c in arb_label()) {
            prop_assert_eq!(a.join(&b).join(&c), a.join(&b.join(&c)));
        }

        #[test]
        fn join_is_idempotent(a in arb_label()) {
            prop_assert_eq!(a.join(&a), a.clone());
        }

        /// Joining never drops a source and never widens the readers: you
        /// cannot launder data by mixing it with something cleaner.
        #[test]
        fn join_is_monotone(a in arb_label(), b in arb_label(), p in "[a-c]") {
            let j = a.join(&b);
            prop_assert!(a.sources.is_subset(&j.sources));
            prop_assert!(b.sources.is_subset(&j.sources));
            prop_assert!(!j.allows_reader(&p) || (a.allows_reader(&p) && b.allows_reader(&p)));
        }

        /// The control rule is downward closed: if the join may steer, so may
        /// each part.
        #[test]
        fn control_trust_is_downward_closed(a in arb_label(), b in arb_label()) {
            let trusted = baseline_trust();
            let j = a.join(&b);
            prop_assert!(
                !j.trusted_for_control(&trusted)
                    || (a.trusted_for_control(&trusted) && b.trusted_for_control(&trusted))
            );
        }
    }
}
