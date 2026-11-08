//! Exact evidence correspondence and ordered completion of accepted obligations.
//! The host owns subject construction and checker-result attribution. A checked
//! receipt is not a proof of a backend's soundness or a compiler's correctness.

/// Logical strength required by the accepted contract. Kinds are not promoted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceKind {
    /// A specified collection of observed executions, without universal proof.
    Empirical,
    /// The checker and bounds named by the obligation.
    Bounded,
    /// A checked statement under the recorded assumptions.
    Theorem,
    /// Implementation correspondence to the accepted specification.
    Refinement,
}

/// Exact subject identity, including session, contract, source/configuration and
/// artifact binding. The host chooses its canonical encoding; no path is proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    /// Host session identity, not a model-provided label.
    pub session: Vec<u8>,
    /// Accepted intent, specification and assumptions revision.
    pub contract: Vec<u8>,
    /// Complete immutable source/build/checker/artifact binding.
    pub inputs: Vec<u8>,
}

/// One immutable accepted obligation, including checker identity/configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Obligation {
    /// Exact formal statement or explicit empirical/bounded contract.
    pub statement: Vec<u8>,
    /// Evidence kind the host accepted for this obligation.
    pub kind: EvidenceKind,
    /// Exact checker/toolchain/configuration binding, not a reported name.
    pub checker: Vec<u8>,
}

/// The host assigns origins; interfaces never deserialize them from tool text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceOrigin {
    /// Untrusted model, repository, page or tool output.
    Model,
    /// Trusted user acceptance channel.
    Host,
    /// Validated output from the host-selected checker adapter.
    Checker,
    /// Host controller computing a new immutable subject.
    Controller,
}

/// Contract lifecycle. Accepted contracts cannot be rewritten by this kernel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidencePhase {
    /// Unaccepted model proposal.
    Draft,
    /// Exposed through the trusted host review channel.
    Reviewed,
    /// Accepted exact obligations and assumptions.
    Accepted,
}

/// A checker result. This type has no model-facing deserialization/registration.
#[derive(Clone, Debug)]
pub struct Receipt {
    /// Immutable inputs for the issued job and checked result.
    pub subject: Subject,
    /// Which ordered obligation this job was issued for.
    pub position: usize,
    /// Statement, kind and checker must all equal the expected obligation.
    pub obligation: Obligation,
    /// Only a validated success discharges the obligation; other outcomes fail.
    pub passed: bool,
    /// Host admission observed the job active while serialized against cancel.
    pub active: bool,
}

/// Private progress over one immutable contract and one current subject.
#[derive(Clone, Debug)]
pub struct Gate {
    subject: Subject,
    obligations: Vec<Obligation>,
    phase: EvidencePhase,
    completed: usize,
}

impl Gate {
    /// A proposal carries no acceptance or evidence authority.
    pub fn propose(subject: Subject, obligations: Vec<Obligation>) -> Self {
        Self {
            subject,
            obligations,
            phase: EvidencePhase::Draft,
            completed: 0,
        }
    }

    /// Show the exact proposal to the host. Repeated inspection is harmless.
    pub fn review(&mut self, origin: EvidenceOrigin) -> bool {
        if origin != EvidenceOrigin::Host {
            return false;
        }
        if self.phase == EvidencePhase::Draft {
            self.phase = EvidencePhase::Reviewed;
        }
        true
    }

    /// Accept after review. Empty contracts cannot become verified projects.
    pub fn accept(&mut self, origin: EvidenceOrigin) -> bool {
        if origin != EvidenceOrigin::Host
            || self.phase != EvidencePhase::Reviewed
            || self.obligations.is_empty()
        {
            return false;
        }
        self.phase = EvidencePhase::Accepted;
        true
    }

    /// Observe acceptance without duplicating the lifecycle in a caller.
    pub fn is_accepted(&self) -> bool {
        self.phase == EvidencePhase::Accepted
    }

    /// Bind changed source/configuration/artifact inputs and discard progress.
    /// Session/contract substitution requires a new proposal and host acceptance.
    pub fn refresh(&mut self, origin: EvidenceOrigin, subject: Subject) -> bool {
        if origin != EvidenceOrigin::Controller
            || self.subject.session != subject.session
            || self.subject.contract != subject.contract
        {
            return false;
        }
        if self.subject != subject {
            self.subject = subject;
            self.completed = 0;
        }
        true
    }

    /// Admit exactly the next issued obligation for the current exact subject.
    /// A duplicate result cannot discharge a second obligation, even if its
    /// statement happens to be identical. An operational failure is not success.
    pub fn record(&mut self, origin: EvidenceOrigin, current: &Subject, receipt: &Receipt) -> bool {
        if !receipt.active
            || origin != EvidenceOrigin::Checker
            || self.phase != EvidencePhase::Accepted
            || self.subject != *current
            || receipt.subject != *current
            || receipt.position != self.completed
            || !receipt.passed
        {
            return false;
        }
        let Some(expected) = self.obligations.get(self.completed) else {
            return false;
        };
        if *expected != receipt.obligation {
            return false;
        }
        let Some(next) = self.completed.checked_add(1) else {
            return false;
        };
        self.completed = next;
        true
    }

    /// A host-derived fresh subject must still match at the delivery decision.
    pub fn eligible(&self, current: &Subject) -> bool {
        self.phase == EvidencePhase::Accepted
            && !self.obligations.is_empty()
            && self.completed == self.obligations.len()
            && self.subject == *current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_current_evidence_is_required_for_each_obligation() {
        let subject = Subject {
            session: b"session".to_vec(),
            contract: b"intent".to_vec(),
            inputs: b"source+config+artifact".to_vec(),
        };
        let obligation = Obligation {
            statement: b"conservation".to_vec(),
            kind: EvidenceKind::Refinement,
            checker: b"pinned checker".to_vec(),
        };
        let receipt = Receipt {
            subject: subject.clone(),
            position: 0,
            obligation: obligation.clone(),
            passed: true,
            active: true,
        };
        let mut gate = Gate::propose(subject.clone(), vec![obligation.clone(), obligation]);
        assert!(!gate.accept(EvidenceOrigin::Host));
        assert!(!gate.review(EvidenceOrigin::Model));
        assert!(gate.review(EvidenceOrigin::Host));
        assert!(!gate.accept(EvidenceOrigin::Model));
        assert!(gate.accept(EvidenceOrigin::Host));
        assert!(!gate.eligible(&subject));
        for kind in [
            EvidenceKind::Empirical,
            EvidenceKind::Bounded,
            EvidenceKind::Theorem,
        ] {
            let mut weaker = receipt.clone();
            weaker.obligation.kind = kind;
            assert!(!gate.record(EvidenceOrigin::Checker, &subject, &weaker));
        }
        for origin in [
            EvidenceOrigin::Model,
            EvidenceOrigin::Host,
            EvidenceOrigin::Controller,
        ] {
            assert!(!gate.record(origin, &subject, &receipt));
        }
        let mut mismatch = receipt.clone();
        mismatch.subject.inputs.push(0);
        assert!(!gate.record(EvidenceOrigin::Checker, &subject, &mismatch));
        assert!(!gate.record(EvidenceOrigin::Checker, &mismatch.subject, &receipt));
        assert!(!gate.record(EvidenceOrigin::Checker, &mismatch.subject, &mismatch));
        let mut failed = receipt.clone();
        failed.passed = false;
        assert!(!gate.record(EvidenceOrigin::Checker, &subject, &failed));
        let mut cancelled = receipt.clone();
        cancelled.active = false;
        assert!(!gate.record(EvidenceOrigin::Checker, &subject, &cancelled));
        assert!(gate.record(EvidenceOrigin::Checker, &subject, &receipt));
        assert!(!gate.record(EvidenceOrigin::Checker, &subject, &receipt));
        assert!(!gate.eligible(&subject));
        let mut second = receipt.clone();
        second.position = 1;
        assert!(gate.record(EvidenceOrigin::Checker, &subject, &second));
        assert!(gate.eligible(&subject));
        let mut changed = subject.clone();
        changed.inputs.push(0);
        assert!(!gate.eligible(&changed));
        assert!(!gate.refresh(EvidenceOrigin::Model, changed.clone()));
        assert!(gate.refresh(EvidenceOrigin::Controller, changed.clone()));
        assert_eq!(gate.completed, 0);
        assert!(!gate.eligible(&changed));
        assert!(!gate.record(EvidenceOrigin::Checker, &changed, &receipt));
        let mut other_contract = changed.clone();
        other_contract.contract.push(0);
        assert!(!gate.refresh(EvidenceOrigin::Controller, other_contract));
        let mut empty = Gate::propose(subject.clone(), Vec::new());
        assert!(empty.review(EvidenceOrigin::Host));
        assert!(!empty.accept(EvidenceOrigin::Host));
        assert!(!empty.eligible(&subject));
    }
}
