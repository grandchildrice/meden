//! One owned project-check transaction, from accepted meaning to admitted output.
//!
//! Input encoding, checker validation and physical persistence are adapters.
//! These values preserve their original host data and admit an output only
//! through the shared contract/job controller. They are not deserializable.

use crate::evidence::{CheckResult, CheckTicket, ContractView, Contracts, EvidenceOrigin, Subject};

/// Frozen agreement and host data before a single-obligation check is issued.
pub struct Preparation<T> {
    agreement: ContractView,
    data: T,
}

/// An issued check, retaining its original agreement, ticket and host data.
///
/// ```compile_fail
/// use meden_core::check::Running;
/// let forged = Running { agreement: panic!(), ticket: panic!(), data: () };
/// ```
///
/// ```compile_fail
/// use meden_core::{check::Running, evidence::Contracts};
/// fn replay<T>(running: Running<T>, state: &mut Contracts) {
///     let _ = running.complete(state, vec![], vec![], vec![], ());
///     let _ = running.complete(state, vec![], vec![], vec![], ());
/// }
/// ```
pub struct Running<T> {
    agreement: ContractView,
    ticket: CheckTicket,
    data: T,
}

impl<T> Preparation<T> {
    /// Consume the exact current accepted view and invalidate earlier evidence.
    pub fn begin(current: &mut Contracts, agreement: ContractView, data: T) -> Option<Self> {
        if !current.is_accepted() || agreement.obligations.len() != 1 {
            return None;
        }
        match current.accepted() {
            Some(view) if view == agreement => {}
            _ => return None,
        }
        current.edit(EvidenceOrigin::Controller);
        Some(Self { agreement, data })
    }

    /// Original host data, without a replacement argument or mutable accessor.
    pub fn data(&self) -> &T {
        &self.data
    }

    /// Issue only under the original meaning, session, generation and obligations.
    /// Read-only freshness observations may have changed the old input binding;
    /// this call binds the newly prepared inputs independently.
    // The pinned extractor supports explicit Option branches and clone assignment.
    #[allow(clippy::question_mark, clippy::assigning_clones)]
    pub fn issue(self, current: &mut Contracts, inputs: Vec<u8>) -> Option<Running<T>> {
        let Some(mut view) = current.accepted() else {
            return None;
        };
        view.subject.inputs = self.agreement.subject.inputs.clone();
        if view != self.agreement || !current.is_accepted() {
            return None;
        }
        let Some(ticket) =
            current.start(EvidenceOrigin::Controller, self.agreement.revision, inputs)
        else {
            return None;
        };
        Some(Running {
            agreement: self.agreement,
            ticket,
            data: self.data,
        })
    }
}

impl<T> Running<T> {
    /// Original host data used by checker and freshness adapters.
    pub fn data(&self) -> &T {
        &self.data
    }

    /// The original ticket, for matching error cleanup and cancellation only.
    pub fn ticket(&self) -> &CheckTicket {
        &self.ticket
    }

    /// Consume validated output under the original obligation and exact job.
    /// The output payload is returned unchanged only after current eligibility.
    /// An unsuccessful attempt aborts this ticket, never a later unrelated job.
    // Keep owned observations and the supported indexed lookup in the extracted boundary.
    #[allow(clippy::get_first, clippy::needless_pass_by_value)]
    pub fn complete<U>(
        self,
        current: &mut Contracts,
        inputs: Vec<u8>,
        output_inputs: Vec<u8>,
        checker: Vec<u8>,
        output: U,
    ) -> Option<U> {
        let Some(required) = self.agreement.obligations.get(0) else {
            current.abort(EvidenceOrigin::Controller, &self.ticket);
            return None;
        };
        if checker != required.checker {
            current.abort(EvidenceOrigin::Controller, &self.ticket);
            return None;
        }
        let result = CheckResult {
            subject: Subject {
                session: self.agreement.subject.session,
                contract: self.agreement.subject.contract,
                inputs: output_inputs.clone(),
            },
            position: 0,
            obligation: required.clone(),
            passed: true,
        };
        if !current.complete(
            EvidenceOrigin::Checker,
            self.agreement.revision,
            &self.ticket,
            inputs,
            result,
        ) || !current.eligible(self.agreement.revision, output_inputs)
        {
            current.abort(EvidenceOrigin::Controller, &self.ticket);
            return None;
        }
        Some(output)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::evidence::{EvidenceKind, Obligation};

    fn accepted(session: &[u8], meaning: &[u8]) -> (Contracts, ContractView) {
        let mut current = Contracts::new(session.to_vec());
        assert!(current.propose(
            EvidenceOrigin::Controller,
            meaning.to_vec(),
            vec![Obligation {
                statement: meaning.to_vec(),
                kind: EvidenceKind::Refinement,
                checker: b"fixed checker".to_vec(),
            }],
        ));
        let view = current.displayed().expect("draft");
        assert!(current.review(EvidenceOrigin::Host, session, view.revision));
        assert!(current.accept(EvidenceOrigin::Host, session, view.revision));
        let view = current.accepted().expect("accepted view");
        (current, view)
    }

    #[test]
    fn transaction_preserves_original_data_and_admits_exact_output_after_fresh_observation() {
        let (mut current, view) = accepted(b"session", b"meaning");
        let preparation = Preparation::begin(&mut current, view.clone(), b"source A".to_vec())
            .expect("prepared original");
        assert_eq!(preparation.data(), b"source A");
        assert!(current.refresh(
            EvidenceOrigin::Controller,
            view.revision,
            b"observed".to_vec()
        ));
        let running = preparation
            .issue(&mut current, b"prepared".to_vec())
            .expect("issued");
        assert_eq!(running.data(), b"source A");
        assert!(current.is_active());
        let artifact = running.complete(
            &mut current,
            b"prepared".to_vec(),
            b"artifact binding".to_vec(),
            b"fixed checker".to_vec(),
            b"original checked artifact".to_vec(),
        );
        assert_eq!(
            artifact.as_deref(),
            Some(b"original checked artifact".as_slice())
        );
        assert!(!current.is_active());
        assert!(current.eligible(view.revision, b"artifact binding".to_vec()));
        let retained = current.accepted().expect("retained meaning");
        assert_eq!(retained.subject.session, view.subject.session);
        assert_eq!(retained.subject.contract, view.subject.contract);
        assert_eq!(retained.obligations, view.obligations);
    }

    #[test]
    fn transaction_refuses_substituted_agreements_and_invalid_completion() {
        for (session, meaning) in [
            (b"other".as_slice(), b"meaning".as_slice()),
            (b"session", b"different"),
        ] {
            let (mut original, view) = accepted(b"session", b"meaning");
            let (mut other, _) = accepted(session, meaning);
            assert!(Preparation::begin(&mut other, view.clone(), ()).is_none());
            let preparation = Preparation::begin(&mut original, view, ()).expect("original");
            assert!(preparation
                .issue(&mut other, b"prepared".to_vec())
                .is_none());
            assert!(!other.is_active());
        }
        for case in 0..5 {
            let (mut current, view) = accepted(b"session", b"meaning");
            let running = Preparation::begin(&mut current, view.clone(), ())
                .expect("prepared")
                .issue(&mut current, b"prepared".to_vec())
                .expect("issued");
            match case {
                2 => {
                    current.invalidate(EvidenceOrigin::Controller);
                }
                3 => {
                    current.propose(
                        EvidenceOrigin::Controller,
                        b"replacement".to_vec(),
                        view.obligations.clone(),
                    );
                }
                4 => {
                    current.propose(
                        EvidenceOrigin::Controller,
                        b"replacement".to_vec(),
                        view.obligations.clone(),
                    );
                    let next = current.displayed().expect("replacement");
                    assert!(current.review(EvidenceOrigin::Host, b"session", next.revision));
                    assert!(current.accept(EvidenceOrigin::Host, b"session", next.revision));
                }
                _ => {}
            }
            let inputs = if case == 0 {
                b"changed".to_vec()
            } else {
                b"prepared".to_vec()
            };
            let checker = if case == 1 {
                b"wrong checker".to_vec()
            } else {
                b"fixed checker".to_vec()
            };
            assert!(running
                .complete(&mut current, inputs, b"output".to_vec(), checker, ())
                .is_none());
            assert!(!current.is_active());
            assert!(!current.eligible(view.revision, b"output".to_vec()));
        }
    }

    #[test]
    fn rejected_old_transaction_cannot_abort_the_newer_job() {
        let (mut current, view) = accepted(b"session", b"meaning");
        let old = Preparation::begin(&mut current, view, ())
            .expect("prepared")
            .issue(&mut current, b"same inputs".to_vec())
            .expect("old job");
        let view = current.accepted().expect("current view");
        let new = Preparation::begin(&mut current, view, ())
            .expect("new preparation")
            .issue(&mut current, b"same inputs".to_vec())
            .expect("new job");
        assert_ne!(old.ticket(), new.ticket());
        assert!(old
            .complete(
                &mut current,
                b"same inputs".to_vec(),
                b"old".to_vec(),
                b"wrong checker".to_vec(),
                ()
            )
            .is_none());
        assert!(current.is_active());
        assert!(new
            .complete(
                &mut current,
                b"same inputs".to_vec(),
                b"new".to_vec(),
                b"fixed checker".to_vec(),
                ()
            )
            .is_some());
        assert!(!current.is_active());
    }
}
