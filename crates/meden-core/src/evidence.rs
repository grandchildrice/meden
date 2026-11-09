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

/// Validated checker output, before the controller establishes job activity.
/// This type has no model-facing deserialization/registration.
#[derive(Clone, Debug)]
pub struct CheckResult {
    /// Exact checked output subject, including its artifact binding.
    pub subject: Subject,
    /// Ordered obligation checked by the host-selected adapter.
    pub position: usize,
    /// Statement, kind and checker reported by that trusted adapter.
    pub obligation: Obligation,
    /// Only validated success can discharge the obligation.
    pub passed: bool,
}

/// Internal evidence; only issued-job admission can record it in production.
#[derive(Clone, Debug)]
struct Receipt {
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
    fn record(&mut self, origin: EvidenceOrigin, current: &Subject, receipt: &Receipt) -> bool {
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

    /// Consume the issued check and record its exact output in one transition.
    /// Callers cannot manufacture an active receipt or record one separately.
    /// A rejected result never restores a consumed job.
    pub fn complete(
        &mut self,
        gate: &mut Gate,
        origin: EvidenceOrigin,
        ticket: &CheckTicket,
        current: &Subject,
        result: CheckResult,
    ) -> bool {
        let active = self.finish(origin, gate, ticket, current);
        if !active {
            return false;
        }
        if !gate.refresh(EvidenceOrigin::Controller, result.subject.clone()) {
            return false;
        }
        let receipt = Receipt {
            subject: result.subject,
            position: result.position,
            obligation: result.obligation,
            passed: result.passed,
            active,
        };
        gate.record(origin, &receipt.subject, &receipt)
    }

    fn matches(&self, ticket: &CheckTicket) -> bool {
        match &self.active {
            Some(active) => active == ticket,
            None => false,
        }
    }
}

/// Read-only-by-copy view of one exact proposal. Changing a returned value
/// cannot change the controller's accepted meaning or obligations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractView {
    /// Fresh proposal generation within the immutable session.
    pub revision: u64,
    /// Exact session, contract and current input binding.
    pub subject: Subject,
    /// Required statement/kind/checker identities.
    pub obligations: Vec<Obligation>,
}

#[derive(Debug)]
struct VersionedGate {
    revision: u64,
    gate: Gate,
}

impl VersionedGate {
    fn view(&self) -> ContractView {
        ContractView {
            revision: self.revision,
            subject: self.gate.subject.clone(),
            obligations: self.gate.obligations.clone(),
        }
    }
}

/// Contract selection, host review and issued evidence jobs share one state.
/// A model cannot replace accepted meaning, expose a mutable gate or reuse
/// a review from a previous generation, even after identical bytes return.
#[derive(Debug)]
pub struct Contracts {
    session: Vec<u8>,
    sequence: u64,
    accepted: Option<VersionedGate>,
    draft: Option<VersionedGate>,
    jobs: CheckJobs,
}

impl Contracts {
    /// Assign the session once. This controller and its jobs are not cloneable.
    pub fn new(session: Vec<u8>) -> Self {
        Self {
            jobs: CheckJobs::new(session.clone()),
            session,
            sequence: 0,
            accepted: None,
            draft: None,
        }
    }

    /// Stage already validated contract data without granting acceptance.
    /// Identical current proposals are idempotent; rejection followed by
    /// reproposal requires a fresh generation and a new review.
    #[allow(clippy::question_mark)] // Explicit extracted overflow branch.
    pub fn propose(
        &mut self,
        origin: EvidenceOrigin,
        contract: Vec<u8>,
        obligations: Vec<Obligation>,
    ) -> bool {
        if origin != EvidenceOrigin::Controller || contract.is_empty() || obligations.is_empty() {
            return false;
        }
        match &self.draft {
            Some(draft) => {
                if draft.gate.subject.contract == contract && draft.gate.obligations == obligations
                {
                    return true;
                }
            }
            None => {
                if let Some(accepted) = &self.accepted {
                    if accepted.gate.subject.contract == contract
                        && accepted.gate.obligations == obligations
                    {
                        return true;
                    }
                }
            }
        }
        let Some(next) = self.sequence.checked_add(1) else {
            return false;
        };
        let subject = Subject {
            session: self.session.clone(),
            contract,
            inputs: Vec::new(),
        };
        self.draft = Some(VersionedGate {
            revision: next,
            gate: Gate::propose(subject, obligations),
        });
        self.sequence = next;
        self.jobs.invalidate(EvidenceOrigin::Controller);
        true
    }

    /// Observe the displayed proposal, or the accepted contract when no draft exists.
    pub fn displayed(&self) -> Option<ContractView> {
        match &self.draft {
            Some(draft) => Some(draft.view()),
            None => self.accepted(),
        }
    }

    /// Observe retained accepted meaning, independently of a replacement draft.
    #[allow(clippy::manual_map)] // Keep the extracted ownership branches explicit.
    pub fn accepted(&self) -> Option<ContractView> {
        match &self.accepted {
            Some(accepted) => Some(accepted.view()),
            None => None,
        }
    }

    /// A pending replacement is never reported accepted or eligible.
    pub fn pending(&self) -> bool {
        self.draft.is_some()
    }

    /// Acceptance applies only when the current displayed meaning is accepted.
    pub fn is_accepted(&self) -> bool {
        if self.draft.is_some() {
            return false;
        }
        match &self.accepted {
            Some(accepted) => accepted.gate.is_accepted(),
            None => false,
        }
    }

    /// Host inspection names the exact displayed session and generation.
    pub fn review(&mut self, origin: EvidenceOrigin, session: &[u8], revision: u64) -> bool {
        let expected: &[u8] = &self.session;
        if expected != session {
            return false;
        }
        match &mut self.draft {
            Some(draft) => draft.revision == revision && draft.gate.review(origin),
            None => match &mut self.accepted {
                Some(accepted) => accepted.revision == revision && accepted.gate.review(origin),
                None => false,
            },
        }
    }

    /// Only a host-reviewed exact draft can replace accepted meaning.
    pub fn accept(&mut self, origin: EvidenceOrigin, session: &[u8], revision: u64) -> bool {
        let expected: &[u8] = &self.session;
        if expected != session {
            return false;
        }
        let accepted = match &mut self.draft {
            Some(draft) => {
                if draft.revision != revision || !draft.gate.accept(origin) {
                    return false;
                }
                VersionedGate {
                    revision: draft.revision,
                    gate: draft.gate.clone(),
                }
            }
            None => return false,
        };
        self.accepted = Some(accepted);
        self.draft = None;
        self.jobs.invalidate(EvidenceOrigin::Controller);
        true
    }

    /// Reject only that exact draft; retained accepted meaning/evidence survives.
    pub fn reject(&mut self, origin: EvidenceOrigin, session: &[u8], revision: u64) -> bool {
        let expected: &[u8] = &self.session;
        if origin != EvidenceOrigin::Host || expected != session {
            return false;
        }
        match &self.draft {
            Some(draft) => {
                if draft.revision != revision {
                    return false;
                }
            }
            None => return false,
        }
        self.draft = None;
        true
    }

    /// Observe changed inputs without changing accepted meaning or obligations.
    pub fn refresh(&mut self, origin: EvidenceOrigin, revision: u64, inputs: Vec<u8>) -> bool {
        match &mut self.accepted {
            Some(accepted) => {
                if accepted.revision != revision {
                    return false;
                }
                let subject = Subject {
                    inputs,
                    ..accepted.gate.subject.clone()
                };
                accepted.gate.refresh(origin, subject)
            }
            None => false,
        }
    }

    /// Issue a prepared job only for the current accepted generation.
    pub fn start(
        &mut self,
        origin: EvidenceOrigin,
        revision: u64,
        inputs: Vec<u8>,
    ) -> Option<CheckTicket> {
        if origin != EvidenceOrigin::Controller || self.draft.is_some() {
            return None;
        }
        match &mut self.accepted {
            Some(accepted) => {
                if accepted.revision != revision {
                    return None;
                }
                self.jobs.invalidate(EvidenceOrigin::Controller);
                let subject = Subject {
                    inputs,
                    ..accepted.gate.subject.clone()
                };
                if !accepted.gate.refresh(origin, subject.clone()) {
                    return None;
                }
                self.jobs.start(origin, &accepted.gate, &subject)
            }
            None => None,
        }
    }

    /// Invalidate jobs without resetting either monotonic sequence.
    pub fn invalidate(&mut self, origin: EvidenceOrigin) -> bool {
        self.jobs.invalidate(origin)
    }

    /// A permitted development mutation discards proof coverage before its
    /// physical effect. Restoring old bytes cannot revive discarded evidence.
    pub fn edit(&mut self, origin: EvidenceOrigin) -> bool {
        if origin != EvidenceOrigin::Controller {
            return false;
        }
        self.jobs.invalidate(origin);
        if let Some(accepted) = &mut self.accepted {
            accepted.gate.completed = 0;
        }
        true
    }

    /// Abort only the matching job; an old error cannot cancel a new job.
    pub fn abort(&mut self, origin: EvidenceOrigin, ticket: &CheckTicket) -> bool {
        self.jobs.abort(origin, ticket)
    }

    /// Observe managed job activity without granting evidence authority.
    pub fn is_active(&self) -> bool {
        self.jobs.is_active()
    }

    /// Admit an exact issued result only for the still-current accepted generation.
    pub fn complete(
        &mut self,
        origin: EvidenceOrigin,
        revision: u64,
        ticket: &CheckTicket,
        current_inputs: Vec<u8>,
        result: CheckResult,
    ) -> bool {
        if self.draft.is_some() {
            return false;
        }
        match &mut self.accepted {
            Some(accepted) => {
                if accepted.revision != revision {
                    return false;
                }
                let current = Subject {
                    inputs: current_inputs,
                    ..accepted.gate.subject.clone()
                };
                self.jobs
                    .complete(&mut accepted.gate, origin, ticket, &current, result)
            }
            None => false,
        }
    }

    /// Fresh eligibility excludes drafts, active jobs and stale generations.
    pub fn eligible(&self, revision: u64, inputs: Vec<u8>) -> bool {
        if self.draft.is_some() || self.jobs.is_active() {
            return false;
        }
        match &self.accepted {
            Some(accepted) => {
                let current = Subject {
                    inputs,
                    ..accepted.gate.subject.clone()
                };
                accepted.revision == revision && accepted.gate.eligible(&current)
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn requirements() -> Vec<Obligation> {
        vec![Obligation {
            statement: b"exact transition".to_vec(),
            kind: EvidenceKind::Refinement,
            checker: b"fixed checker".to_vec(),
        }]
    }

    fn accept_contract(
        contracts: &mut Contracts,
        meaning: &[u8],
    ) -> Result<ContractView, &'static str> {
        assert!(contracts.propose(EvidenceOrigin::Controller, meaning.to_vec(), requirements()));
        let view = contracts.displayed().ok_or("no draft")?;
        assert!(contracts.review(EvidenceOrigin::Host, b"session", view.revision));
        assert!(contracts.accept(EvidenceOrigin::Host, b"session", view.revision));
        Ok(view)
    }

    fn passed(view: &ContractView, inputs: &[u8]) -> Result<CheckResult, &'static str> {
        Ok(CheckResult {
            subject: Subject {
                inputs: inputs.to_vec(),
                ..view.subject.clone()
            },
            position: 0,
            obligation: view.obligations.first().ok_or("no obligation")?.clone(),
            passed: true,
        })
    }

    #[test]
    fn identical_reproposal_after_rejection_requires_a_fresh_host_review(
    ) -> Result<(), &'static str> {
        let mut contracts = Contracts::new(b"session".to_vec());
        let accepted = accept_contract(&mut contracts, b"original")?;
        assert!(contracts.propose(
            EvidenceOrigin::Controller,
            b"replacement".to_vec(),
            requirements()
        ));
        let old = contracts.displayed().ok_or("no draft")?;
        assert!(contracts.review(EvidenceOrigin::Host, b"session", old.revision));
        assert!(contracts.propose(
            EvidenceOrigin::Controller,
            b"replacement".to_vec(),
            requirements()
        ));
        assert_eq!(contracts.displayed(), Some(old.clone()));
        assert!(contracts.reject(EvidenceOrigin::Host, b"session", old.revision));
        assert_eq!(contracts.accepted(), Some(accepted));
        assert!(contracts.propose(
            EvidenceOrigin::Controller,
            b"replacement".to_vec(),
            requirements()
        ));
        let new = contracts.displayed().ok_or("no draft")?;
        assert_eq!(new.subject, old.subject);
        assert!(new.revision > old.revision);
        assert!(!contracts.review(EvidenceOrigin::Host, b"session", old.revision));
        assert!(!contracts.accept(EvidenceOrigin::Host, b"session", old.revision));
        assert!(!contracts.accept(EvidenceOrigin::Host, b"session", new.revision));
        assert!(contracts.review(EvidenceOrigin::Host, b"session", new.revision));
        assert!(!contracts.accept(EvidenceOrigin::Host, b"session", old.revision));
        assert!(contracts.accept(EvidenceOrigin::Host, b"session", new.revision));
        Ok(())
    }

    #[test]
    fn rejecting_a_replacement_preserves_accepted_evidence_but_never_its_old_job(
    ) -> Result<(), &'static str> {
        let mut contracts = Contracts::new(b"session".to_vec());
        let view = accept_contract(&mut contracts, b"original")?;
        let inputs = b"source and artifact".to_vec();
        let ticket = contracts
            .start(EvidenceOrigin::Controller, view.revision, inputs.clone())
            .ok_or("no job")?;
        let result = passed(&view, &inputs)?;
        assert!(contracts.complete(
            EvidenceOrigin::Checker,
            view.revision,
            &ticket,
            inputs.clone(),
            result.clone()
        ));
        assert!(contracts.eligible(view.revision, inputs.clone()));
        let old_job = contracts
            .start(EvidenceOrigin::Controller, view.revision, inputs.clone())
            .ok_or("no job")?;
        assert!(contracts.propose(
            EvidenceOrigin::Controller,
            b"replacement".to_vec(),
            requirements()
        ));
        let draft = contracts.displayed().ok_or("no draft")?;
        assert!(!contracts.is_accepted() && !contracts.eligible(view.revision, inputs.clone()));
        assert!(contracts
            .start(EvidenceOrigin::Controller, view.revision, inputs.clone())
            .is_none());
        assert!(!contracts.complete(
            EvidenceOrigin::Checker,
            view.revision,
            &old_job,
            inputs.clone(),
            result.clone()
        ));
        assert!(contracts.reject(EvidenceOrigin::Host, b"session", draft.revision));
        assert!(contracts.eligible(view.revision, inputs.clone()));
        assert!(!contracts.complete(
            EvidenceOrigin::Checker,
            view.revision,
            &old_job,
            inputs.clone(),
            result
        ));
        assert!(contracts.eligible(view.revision, inputs));
        Ok(())
    }

    #[test]
    fn editing_then_restoring_inputs_does_not_restore_proof_coverage() -> Result<(), &'static str> {
        let mut contracts = Contracts::new(b"session".to_vec());
        let view = accept_contract(&mut contracts, b"original")?;
        let inputs = b"original source and artifact".to_vec();
        let ticket = contracts
            .start(EvidenceOrigin::Controller, view.revision, inputs.clone())
            .ok_or("no job")?;
        let result = passed(&view, &inputs)?;
        assert!(contracts.complete(
            EvidenceOrigin::Checker,
            view.revision,
            &ticket,
            inputs.clone(),
            result.clone()
        ));
        assert!(contracts.eligible(view.revision, inputs.clone()));
        let pending = contracts
            .start(EvidenceOrigin::Controller, view.revision, inputs.clone())
            .ok_or("no job")?;
        assert!(contracts.edit(EvidenceOrigin::Controller));
        assert!(contracts.refresh(EvidenceOrigin::Controller, view.revision, inputs.clone()));
        assert!(!contracts.eligible(view.revision, inputs.clone()));
        assert!(!contracts.complete(
            EvidenceOrigin::Checker,
            view.revision,
            &pending,
            inputs.clone(),
            result.clone()
        ));
        let next = contracts
            .start(EvidenceOrigin::Controller, view.revision, inputs.clone())
            .ok_or("no job")?;
        assert!(contracts.complete(
            EvidenceOrigin::Checker,
            view.revision,
            &next,
            inputs.clone(),
            result
        ));
        assert!(contracts.eligible(view.revision, inputs));
        Ok(())
    }

    #[test]
    fn untrusted_origins_and_modified_snapshots_cannot_change_owned_meaning(
    ) -> Result<(), &'static str> {
        let mut contracts = Contracts::new(b"session".to_vec());
        let view = accept_contract(&mut contracts, b"original")?;
        let inputs = b"checked source".to_vec();
        let ticket = contracts
            .start(EvidenceOrigin::Controller, view.revision, inputs.clone())
            .ok_or("no job")?;
        assert!(contracts
            .start(
                EvidenceOrigin::Controller,
                view.revision + 1,
                inputs.clone()
            )
            .is_none());
        assert!(!contracts.complete(
            EvidenceOrigin::Checker,
            view.revision + 1,
            &ticket,
            inputs.clone(),
            passed(&view, &inputs)?
        ));
        assert!(contracts.is_active());
        for origin in [
            EvidenceOrigin::Model,
            EvidenceOrigin::Host,
            EvidenceOrigin::Checker,
        ] {
            assert!(!contracts.propose(origin, b"changed".to_vec(), requirements()));
            assert!(contracts
                .start(origin, view.revision, inputs.clone())
                .is_none());
            assert!(!contracts.refresh(origin, view.revision, b"changed".to_vec()));
            assert!(!contracts.edit(origin));
            assert!(!contracts.invalidate(origin));
            assert!(!contracts.abort(origin, &ticket));
            assert!(contracts.is_active());
        }
        let mut copy = contracts.accepted().ok_or("no accepted contract")?;
        copy.subject.contract = b"forged".to_vec();
        copy.obligations.clear();
        copy.revision += 1;
        assert_eq!(
            contracts
                .accepted()
                .ok_or("no accepted contract")?
                .subject
                .contract,
            b"original"
        );
        assert!(contracts.complete(
            EvidenceOrigin::Checker,
            view.revision,
            &ticket,
            inputs.clone(),
            passed(&view, &inputs)?
        ));
        assert!(contracts.eligible(view.revision, inputs));
        assert!(contracts.propose(
            EvidenceOrigin::Controller,
            b"changed".to_vec(),
            requirements()
        ));
        let draft = contracts.displayed().ok_or("no draft")?;
        for origin in [
            EvidenceOrigin::Model,
            EvidenceOrigin::Controller,
            EvidenceOrigin::Checker,
        ] {
            assert!(!contracts.review(origin, b"session", draft.revision));
            assert!(!contracts.accept(origin, b"session", draft.revision));
            assert!(!contracts.reject(origin, b"session", draft.revision));
        }
        assert!(!contracts.review(EvidenceOrigin::Host, b"foreign", draft.revision));
        assert!(!contracts.reject(EvidenceOrigin::Host, b"foreign", draft.revision));
        Ok(())
    }

    #[test]
    fn exhausted_generations_never_wrap_or_preserve_an_old_job_on_reissue(
    ) -> Result<(), &'static str> {
        let mut contracts = Contracts::new(b"session".to_vec());
        let view = accept_contract(&mut contracts, b"original")?;
        let inputs = b"checked source".to_vec();
        let ticket = contracts
            .start(EvidenceOrigin::Controller, view.revision, inputs.clone())
            .ok_or("no job")?;
        contracts.sequence = u64::MAX;
        assert!(!contracts.propose(
            EvidenceOrigin::Controller,
            b"changed".to_vec(),
            requirements()
        ));
        assert!(contracts.is_active());
        assert!(!contracts.pending());
        contracts.jobs.sequence = u64::MAX;
        assert!(contracts
            .start(EvidenceOrigin::Controller, view.revision, inputs.clone())
            .is_none());
        assert!(!contracts.is_active());
        assert!(!contracts.complete(
            EvidenceOrigin::Checker,
            view.revision,
            &ticket,
            inputs.clone(),
            passed(&view, &inputs)?
        ));
        assert!(!contracts.eligible(view.revision, inputs));
        Ok(())
    }

    #[test]
    fn admission_requires_an_issued_job_and_the_exact_accepted_result() -> Result<(), &'static str>
    {
        let current = Subject {
            session: b"session".to_vec(),
            contract: b"accepted contract".to_vec(),
            inputs: b"prepared source and checker".to_vec(),
        };
        let obligation = Obligation {
            statement: b"refinement".to_vec(),
            kind: EvidenceKind::Refinement,
            checker: b"pinned checker".to_vec(),
        };
        let mut gate = Gate::propose(current.clone(), vec![obligation.clone(); 2]);
        let mut checks = CheckJobs::new(current.session.clone());
        assert!(gate.review(EvidenceOrigin::Host) && gate.accept(EvidenceOrigin::Host));
        let result = CheckResult {
            subject: Subject {
                inputs: b"bound output artifact".to_vec(),
                ..current.clone()
            },
            position: 0,
            obligation,
            passed: true,
        };
        let first = checks
            .start(EvidenceOrigin::Controller, &gate, &current)
            .ok_or("no job")?;
        for origin in [
            EvidenceOrigin::Model,
            EvidenceOrigin::Host,
            EvidenceOrigin::Controller,
        ] {
            assert!(!checks.complete(&mut gate, origin, &first, &current, result.clone()));
            assert_eq!(gate.completed, 0);
        }
        let mut changed = current.clone();
        changed.inputs.push(1);
        assert!(!checks.complete(
            &mut gate,
            EvidenceOrigin::Checker,
            &first,
            &changed,
            result.clone()
        ));
        assert_eq!(gate.completed, 0);
        checks.invalidate(EvidenceOrigin::Controller);
        assert!(!checks.complete(
            &mut gate,
            EvidenceOrigin::Checker,
            &first,
            &current,
            result.clone()
        ));
        for fault in 0..7 {
            let mut wrong = result.clone();
            match fault {
                0 => wrong.passed = false,
                1 => wrong.position = 1,
                2 => wrong.obligation.kind = EvidenceKind::Empirical,
                3 => wrong.obligation.statement.push(1),
                4 => wrong.obligation.checker.push(1),
                5 => wrong.subject.session.push(1),
                _ => wrong.subject.contract.push(1),
            }
            let ticket = checks
                .start(EvidenceOrigin::Controller, &gate, &current)
                .ok_or("no job")?;
            assert!(!checks.complete(&mut gate, EvidenceOrigin::Checker, &ticket, &current, wrong));
            assert_eq!(gate.completed, 0);
            assert!(!checks.is_active());
            assert!(!gate.eligible(&result.subject));
        }
        let ticket = checks
            .start(EvidenceOrigin::Controller, &gate, &current)
            .ok_or("no job")?;
        assert!(checks.complete(
            &mut gate,
            EvidenceOrigin::Checker,
            &ticket,
            &current,
            result.clone()
        ));
        assert!(!checks.is_active());
        assert_eq!(gate.completed, 1);
        assert!(!gate.eligible(&result.subject));
        let mut second = result.clone();
        second.position = 1;
        assert!(!checks.complete(
            &mut gate,
            EvidenceOrigin::Checker,
            &ticket,
            &current,
            second.clone()
        ));
        let next = checks
            .start(EvidenceOrigin::Controller, &gate, &current)
            .ok_or("no job")?;
        assert!(checks.complete(&mut gate, EvidenceOrigin::Checker, &next, &current, second));
        assert!(gate.eligible(&result.subject));
        Ok(())
    }

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
