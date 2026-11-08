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

/// An issued check for one exact prepared input. Only the controller creates
/// tickets; model interfaces do not deserialize them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckTicket {
    id: u64,
    subject: Subject,
}

/// One session's check lifecycle. Replacing, cancelling or consuming a job
/// never resets the sequence, including when old source bytes are restored.
#[derive(Debug)]
pub struct CheckJobs {
    session: Vec<u8>,
    sequence: u64,
    active: Option<CheckTicket>,
}

impl CheckJobs {
    /// The host assigns the session scope once. The registry is not cloneable.
    pub fn new(session: Vec<u8>) -> Self {
        Self {
            session,
            sequence: 0,
            active: None,
        }
    }

    /// Issue a fresh job only for an accepted contract in this session.
    /// A new successful issue supersedes the previous active job.
    #[allow(clippy::question_mark)] // Keep the overflow branch explicit in the extracted transition.
    pub fn start(
        &mut self,
        origin: EvidenceOrigin,
        gate: &Gate,
        subject: &Subject,
    ) -> Option<CheckTicket> {
        if origin != EvidenceOrigin::Controller
            || !gate.is_accepted()
            || self.session != subject.session
            || gate.subject.session != subject.session
            || gate.subject.contract != subject.contract
        {
            return None;
        }
        let Some(next) = self.sequence.checked_add(1) else {
            return None;
        };
        let ticket = CheckTicket {
            id: next,
            subject: subject.clone(),
        };
        self.sequence = next;
        self.active = Some(ticket.clone());
        Some(ticket)
    }

    /// Only controller-observed invalidation can cancel the pending job.
    pub fn invalidate(&mut self, origin: EvidenceOrigin) -> bool {
        if origin != EvidenceOrigin::Controller {
            return false;
        }
        self.active = None;
        true
    }

    /// A failed old job cannot cancel a newer job which superseded it.
    pub fn abort(&mut self, origin: EvidenceOrigin, ticket: &CheckTicket) -> bool {
        if !self.matches(ticket) {
            return false;
        }
        self.invalidate(origin)
    }

    /// Consume an exact active job once. This establishes correspondence and
    /// freshness, not checker soundness; only a validated checker may call it.
    pub fn finish(
        &mut self,
        origin: EvidenceOrigin,
        gate: &Gate,
        ticket: &CheckTicket,
        current: &Subject,
    ) -> bool {
        if origin != EvidenceOrigin::Checker
            || !gate.is_accepted()
            || !self.matches(ticket)
            || ticket.subject != *current
            || self.session != current.session
            || gate.subject.session != current.session
            || gate.subject.contract != current.contract
        {
            return false;
        }
        self.active = None;
        true
    }

    /// A status observation grants no completion authority.
    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    fn matches(&self, ticket: &CheckTicket) -> bool {
        match &self.active {
            Some(active) => active == ticket,
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issued_jobs_are_session_bound_single_use_and_never_resurrected() -> Result<(), &'static str>
    {
        let request = Subject {
            session: b"one".to_vec(),
            contract: b"accepted".to_vec(),
            inputs: b"prepared source+tools".to_vec(),
        };
        let obligation = Obligation {
            statement: b"contract".to_vec(),
            kind: EvidenceKind::Refinement,
            checker: b"checker".to_vec(),
        };
        let mut gate = Gate::propose(request.clone(), vec![obligation.clone()]);
        let mut jobs = CheckJobs::new(request.session.clone());
        assert!(jobs
            .start(EvidenceOrigin::Controller, &gate, &request)
            .is_none());
        assert!(gate.review(EvidenceOrigin::Host) && gate.accept(EvidenceOrigin::Host));
        for origin in [
            EvidenceOrigin::Model,
            EvidenceOrigin::Host,
            EvidenceOrigin::Checker,
        ] {
            assert!(jobs.start(origin, &gate, &request).is_none());
        }
        let first = jobs
            .start(EvidenceOrigin::Controller, &gate, &request)
            .ok_or("job not issued")?;
        for origin in [
            EvidenceOrigin::Model,
            EvidenceOrigin::Host,
            EvidenceOrigin::Controller,
        ] {
            assert!(!jobs.finish(origin, &gate, &first, &request));
        }
        let mut changed = request.clone();
        changed.inputs.push(1);
        assert!(!jobs.finish(EvidenceOrigin::Checker, &gate, &first, &changed));
        let second = jobs
            .start(EvidenceOrigin::Controller, &gate, &request)
            .ok_or("job not issued")?;
        assert!(second.id > first.id);
        assert!(!jobs.finish(EvidenceOrigin::Checker, &gate, &first, &request));
        assert!(!jobs.abort(EvidenceOrigin::Controller, &first));
        assert!(jobs.is_active());
        assert!(jobs.finish(EvidenceOrigin::Checker, &gate, &second, &request));
        assert!(!jobs.finish(EvidenceOrigin::Checker, &gate, &second, &request));

        let cancelled = jobs
            .start(EvidenceOrigin::Controller, &gate, &request)
            .ok_or("job not issued")?;
        assert!(!jobs.invalidate(EvidenceOrigin::Model));
        assert!(jobs.is_active());
        assert!(jobs.invalidate(EvidenceOrigin::Controller));
        assert!(!jobs.is_active());
        assert!(!jobs.finish(EvidenceOrigin::Checker, &gate, &cancelled, &request));
        let restored = jobs
            .start(EvidenceOrigin::Controller, &gate, &request)
            .ok_or("job not issued")?;
        assert!(restored.id > cancelled.id);
        assert!(!jobs.finish(EvidenceOrigin::Checker, &gate, &cancelled, &request));
        assert!(!jobs.abort(EvidenceOrigin::Model, &restored));
        assert!(jobs.abort(EvidenceOrigin::Controller, &restored));
        assert!(!jobs.finish(EvidenceOrigin::Checker, &gate, &restored, &request));

        let mut other = request.clone();
        other.session = b"two".to_vec();
        let mut other_gate = Gate::propose(other.clone(), vec![obligation]);
        assert!(other_gate.review(EvidenceOrigin::Host) && other_gate.accept(EvidenceOrigin::Host));
        assert!(jobs
            .start(EvidenceOrigin::Controller, &other_gate, &other)
            .is_none());
        jobs.sequence = u64::MAX;
        assert!(jobs
            .start(EvidenceOrigin::Controller, &gate, &request)
            .is_none());
        assert!(!jobs.is_active());
        Ok(())
    }

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
