//! Host-reviewed mathematical meaning and its input-bound verified artifact.
//! Models propose drafts and implementations; only host review accepts meaning.

use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use meden_agent::{
    tool::{Args, Tool, ToolError, ToolSig},
    Registry,
};
use meden_core::{
    evidence::{
        CheckJobs, CheckResult, CheckTicket, EvidenceKind, EvidenceOrigin as Origin, Gate,
        Obligation, Subject,
    },
    Capability, Grant, Policy, Requirement,
};
use meden_runtime::{
    process::{self, Control, Limits, Termination},
    verification::{
        self,
        kernel::{self, Artifact, Report},
        Bundle, Contract, Failure, FailureKind,
    },
    workspace::Workspace,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub session: String,
    pub contract: String,
}

#[derive(Clone, Serialize)]
pub struct Summary {
    pub reference: Reference,
    pub proposed: bool,
    /// Acceptance of the displayed contract, never of a pending replacement.
    pub accepted: bool,
    pub verified: bool,
    pub verification_failure: Option<FailureKind>,
    pub contract: Option<Contract>,
    pub accepted_contract: Option<Contract>,
    pub review: String,
    pub source_path: String,
    pub api: String,
    pub artifact: Option<PathBuf>,
    pub source_sha256: Option<String>,
    pub artifact_sha256: Option<String>,
}

struct Agreement {
    contract: Contract,
    scope: Subject,
    obligation: Obligation,
    gate: Gate,
}

impl Agreement {
    fn new(session: &str, contract: Contract) -> Result<Self> {
        let bytes = contract.canonical()?;
        let obligation = Obligation {
            statement: bytes.clone(),
            kind: EvidenceKind::Refinement,
            checker: kernel::checker_id(&contract)?.into_bytes(),
        };
        let scope = Subject {
            session: session.as_bytes().to_vec(),
            contract: bytes,
            inputs: b"pending".to_vec(),
        };
        let gate = Gate::propose(scope.clone(), vec![obligation.clone()]);
        Ok(Self {
            contract,
            scope,
            obligation,
            gate,
        })
    }

    fn subject(&self, inputs: Vec<u8>) -> Subject {
        Subject {
            inputs,
            ..self.scope.clone()
        }
    }

    fn reference(&self, session: &str) -> Reference {
        Reference {
            session: session.to_owned(),
            contract: hash(&self.scope.contract),
        }
    }
}

struct State {
    accepted: Option<Agreement>,
    draft: Option<Agreement>,
    jobs: CheckJobs,
    dirty: bool,
    autorun: bool,
    artifact: Option<Artifact>,
    failure: Option<Failure>,
}

pub struct Project {
    session: String,
    bundle: Option<PathBuf>,
    storage: PathBuf,
    state: Mutex<State>,
}

impl Project {
    pub fn new(root: &Path) -> Self {
        let session =
            root.as_os_str()
                .as_encoded_bytes()
                .iter()
                .fold(String::new(), |mut out, b| {
                    let _ = write!(out, "{b:02x}");
                    out
                });
        let jobs = CheckJobs::new(session.as_bytes().to_vec());
        Self {
            session,
            bundle: std::env::var_os("MEDEN_VERIFIER_BUNDLE").map(PathBuf::from),
            storage: root
                .parent()
                .and_then(Path::parent)
                .unwrap_or(root)
                .join("artifacts"),
            state: Mutex::new(State {
                accepted: None,
                draft: None,
                jobs,
                dirty: false,
                autorun: false,
                artifact: None,
                failure: None,
            }),
        }
    }

    pub fn matches(&self, root: &Path) -> bool {
        self.session == Self::new(root).session
    }

    fn bundle(&self) -> std::result::Result<Bundle, Failure> {
        let path = self.bundle.as_deref().ok_or_else(|| {
            Failure::unavailable(
                "verification needs a host-prepared MEDEN_VERIFIER_BUNDLE before confidential work",
            )
        })?;
        Bundle::load(path).map_err(Failure::unavailable)
    }

    fn binding(agreement: &Agreement, report: &Report, source: &str) -> Result<Subject> {
        anyhow::ensure!(
            agreement.contract == report.contract,
            "artifact belongs to a different contract"
        );
        Ok(agreement.subject(serde_json::to_vec(&json!({"source":hash(source.as_bytes()),"artifact":report.artifact_sha256,
            "bundle":report.bundle_sha256,"checker":report.checker_sha256,"worker":report.worker_sha256}))?))
    }

    pub fn propose(&self, contract: Contract, control: &Control) -> Result<Summary> {
        {
            let state = self
                .state
                .lock()
                .map_err(|_| anyhow::anyhow!("project unavailable"))?;
            if state
                .draft
                .as_ref()
                .or(state.accepted.as_ref())
                .is_some_and(|a| a.contract == contract)
            {
                return Ok(self.summary(&state, false));
            }
        }
        // A valid mathematical draft is not implementation evidence. Model
        // terms are parsed as restricted data before any elaboration.
        let result = self.bundle().and_then(|bundle| {
            kernel::check_contract(
                &contract,
                &bundle,
                &std::env::current_exe().map_err(Failure::unavailable)?,
                control,
            )
        });
        if let Err(failure) = result {
            self.record_failure(failure.clone());
            return Err(failure.into());
        }
        let draft = Agreement::new(&self.session, contract)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        control
            .with_active(|| {
                state.jobs.invalidate(Origin::Controller);
                state.draft = Some(draft);
                state.autorun = false;
                state.failure = None;
                Ok(self.summary(&state, false))
            })
            .map_err(Into::into)
    }

    fn reference(&self, agreement: &Agreement, reference: &Reference) -> Result<()> {
        anyhow::ensure!(
            reference.session == self.session
                && reference.contract == hash(&agreement.scope.contract),
            "contract belongs to another session or is no longer the displayed proposal"
        );
        Ok(())
    }

    pub fn review(&self, reference: &Reference) -> Result<Summary> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        let agreement = if let Some(draft) = &mut state.draft {
            draft
        } else {
            state.accepted.as_mut().context("no contract proposal")?
        };
        self.reference(agreement, reference)?;
        anyhow::ensure!(agreement.gate.review(Origin::Host), "review refused");
        Ok(self.summary(&state, false))
    }

    pub fn accept(&self, reference: &Reference) -> Result<Summary> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        let draft = state.draft.as_mut().context("no pending contract")?;
        self.reference(draft, reference)?;
        anyhow::ensure!(
            draft.gate.accept(Origin::Host),
            "contract must be reviewed before host acceptance"
        );
        state.jobs.invalidate(Origin::Controller);
        state.accepted = state.draft.take();
        state.artifact = None;
        state.failure = None;
        state.dirty = true;
        state.autorun = true;
        Ok(self.summary(&state, false))
    }

    pub fn reject(&self, reference: &Reference) -> Result<Summary> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        self.reference(
            state.draft.as_ref().context("no pending contract")?,
            reference,
        )?;
        state.draft = None;
        // The accepted meaning and its evidence are retained. Source changes
        // still require fresh checking, including after a rejected proposal.
        Ok(self.summary(&state, false))
    }

    pub fn pending(&self) -> bool {
        self.state.lock().is_ok_and(|s| s.draft.is_some())
    }
    pub fn accepted(&self) -> bool {
        self.state.lock().is_ok_and(|s| {
            s.draft.is_none() && s.accepted.as_ref().is_some_and(|a| a.gate.is_accepted())
        })
    }
    pub fn current_reference(&self) -> Result<Reference> {
        Ok(self.stored_status()?.reference)
    }
    pub fn take_autorun(&self) -> bool {
        self.state
            .lock()
            .is_ok_and(|mut s| std::mem::take(&mut s.autorun))
    }

    pub fn continuation(&self) -> Result<String> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        anyhow::ensure!(
            state.draft.is_none(),
            "pending contract requires host review"
        );
        let agreement = state.accepted.as_ref().context("no accepted contract")?;
        Ok(format!("Proceed under the host-accepted contract. Implement {} and use verify_project. Repair failures; report unknown if verification cannot finish. Do not change the contract or checker. Accepted meaning (host-owned):\n{}", agreement.contract.source(), agreement.contract.review_text()))
    }

    pub fn mark_dirty(&self) {
        if let Ok(mut state) = self.state.lock() {
            if let Some(agreement) = &mut state.accepted {
                agreement
                    .gate
                    .refresh(Origin::Controller, agreement.subject(b"editing".to_vec()));
            }
            state.dirty = true;
            state.jobs.invalidate(Origin::Controller);
            if state.failure.as_ref().is_some_and(|f| f.kind.repairable()) {
                state.failure = None;
            }
        }
    }

    pub fn begin_turn(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.failure = None;
        }
    }
    pub fn cancel(&self, control: &Control) {
        if let Ok(mut state) = self.state.lock() {
            state.jobs.invalidate(Origin::Controller);
            control.cancel();
        } else {
            control.cancel();
        }
    }
    pub fn failure(&self) -> Option<Failure> {
        self.state.lock().ok().and_then(|s| s.failure.clone())
    }
    fn record_failure(&self, failure: Failure) {
        if let Ok(mut state) = self.state.lock() {
            state.failure = Some(failure);
        }
    }

    pub fn status(
        &self,
        workspace: &Workspace,
        control: &Control,
    ) -> std::result::Result<Summary, Failure> {
        self.current_status(workspace, control).map_err(|error| {
            let failure = error
                .downcast_ref::<Failure>()
                .cloned()
                .unwrap_or_else(|| Failure::unavailable(error));
            self.record_failure(failure.clone());
            failure
        })
    }

    fn current_status(&self, workspace: &Workspace, control: &Control) -> Result<Summary> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        if state.jobs.is_active() || state.draft.is_some() {
            return Ok(self.summary(&state, false));
        }
        let Some(agreement) = state.accepted.as_ref() else {
            return Ok(self.summary(&state, false));
        };
        let Some(artifact) = state.artifact.as_ref() else {
            return Ok(self.summary(&state, false));
        };
        let current = Self::binding(
            agreement,
            &artifact.report,
            &snapshot(workspace, agreement.contract.source(), control)?,
        )?;
        let verified = state.accepted.as_mut().is_some_and(|a| {
            a.gate.refresh(Origin::Controller, current.clone()) && a.gate.eligible(&current)
        });
        state.dirty = false;
        Ok(self.summary(&state, verified))
    }

    pub fn stored_status(&self) -> Result<Summary> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        Ok(self.summary(&state, false))
    }

    fn summary(&self, state: &State, verified: bool) -> Summary {
        let displayed = state.draft.as_ref().or(state.accepted.as_ref());
        let reference = displayed.map_or_else(
            || Reference {
                session: self.session.clone(),
                contract: String::new(),
            },
            |a| a.reference(&self.session),
        );
        let mut review = displayed.map_or_else(String::new, |a| a.contract.review_text());
        if state.draft.is_some() {
            if let Some(accepted) = &state.accepted {
                review = format!("Previously accepted contract (retained until host accepts a replacement):\n{}\n\nProposed replacement:\n{review}",accepted.contract.review_text());
            }
        }
        let artifact = if state.draft.is_none() {
            state.artifact.as_ref()
        } else {
            None
        };
        Summary {
            reference,
            proposed: displayed.is_some(),
            accepted: state.draft.is_none() && state.accepted.is_some(),
            verified: verified && !state.dirty && state.draft.is_none(),
            verification_failure: state.failure.as_ref().map(|f| f.kind),
            contract: displayed.map(|a| a.contract.clone()),
            accepted_contract: state.accepted.as_ref().map(|a| a.contract.clone()),
            review,
            source_path: displayed.map_or_else(String::new, |a| a.contract.source().to_owned()),
            api: displayed.map_or_else(String::new, |a| a.contract.api()),
            artifact: artifact.map(|a| a.directory.clone()),
            source_sha256: artifact.map(|a| a.report.source_sha256.clone()),
            artifact_sha256: artifact.map(|a| a.report.artifact_sha256.clone()),
        }
    }

    pub fn verify(&self, workspace: &Workspace, control: &Control) -> Result<Summary> {
        let result = self.verify_current(workspace, control);
        if let Err(error) = &result {
            let kind = error
                .downcast_ref::<Failure>()
                .map_or(FailureKind::Inconclusive, |f| f.kind);
            self.record_failure(Failure::new(kind, error));
        }
        result
    }

    fn verify_current(&self, workspace: &Workspace, control: &Control) -> Result<Summary> {
        let contract = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow::anyhow!("project unavailable"))?;
            anyhow::ensure!(
                state.draft.is_none(),
                "pending contract requires host review"
            );
            let agreement = state
                .accepted
                .as_mut()
                .context("host contract acceptance is required before verification")?;
            anyhow::ensure!(
                agreement.gate.is_accepted()
                    && agreement
                        .gate
                        .refresh(Origin::Controller, agreement.subject(b"checking".to_vec())),
                "job binding refused"
            );
            let contract = agreement.contract.clone();
            state.dirty = true;
            state.jobs.invalidate(Origin::Controller);
            state.failure = None;
            contract
        };
        let source = snapshot(workspace, contract.source(), control)?;
        let bundle = self.bundle()?;
        let prepared = kernel::prepare_kernel(
            &source,
            &contract,
            &bundle,
            &std::env::current_exe().map_err(Failure::unavailable)?,
            control,
        )?;
        let ticket = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow::anyhow!("project unavailable"))?;
            control.with_active(|| {
                if state.draft.is_some() {
                    return Err(io::Error::other("contract changed during preparation"));
                }
                let State { accepted, jobs, .. } = &mut *state;
                let agreement = accepted
                    .as_ref()
                    .ok_or_else(|| io::Error::other("no accepted contract"))?;
                if agreement.contract != contract {
                    return Err(io::Error::other("contract changed during preparation"));
                }
                let request = agreement.subject(serde_json::to_vec(prepared.inputs())?);
                jobs.start(Origin::Controller, &agreement.gate, &request)
                    .ok_or_else(|| io::Error::other("job issue refused"))
            })?
        };
        let result = self.run_job(prepared, &ticket, workspace, control);
        if result.is_err() {
            if let Ok(mut state) = self.state.lock() {
                state.jobs.abort(Origin::Controller, &ticket);
            }
        }
        result
    }

    fn run_job(
        &self,
        prepared: kernel::PreparedKernel<'_>,
        ticket: &CheckTicket,
        workspace: &Workspace,
        control: &Control,
    ) -> Result<Summary> {
        let checked = prepared.check(control)?;
        let fresh = snapshot(workspace, checked.report().contract.source(), control)?;
        let mut inputs = checked.inputs();
        inputs.source_sha256 = hash(fresh.as_bytes());
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        control
            .with_active(|| {
                if state.draft.is_some() {
                    return Err(io::Error::other("contract changed during checking"));
                }
                let State { accepted, jobs, .. } = &mut *state;
                let agreement = accepted
                    .as_mut()
                    .ok_or_else(|| io::Error::other("no accepted contract"))?;
                let current = agreement.subject(serde_json::to_vec(&inputs)?);
                let subject =
                    Self::binding(agreement, checked.report(), &fresh).map_err(io::Error::other)?;
                let result = CheckResult {
                    subject: subject.clone(),
                    position: 0,
                    obligation: agreement.obligation.clone(),
                    passed: true,
                };
                if checked.report().checker_sha256.as_bytes() != agreement.obligation.checker
                    || !jobs.complete(
                        &mut agreement.gate,
                        Origin::Checker,
                        ticket,
                        &current,
                        result,
                    )
                    || !agreement.gate.eligible(&subject)
                {
                    return Err(io::Error::other("evidence correspondence refused"));
                }
                state.artifact = Some(checked.persist(&self.storage)?);
                state.dirty = false;
                Ok(self.summary(&state, true))
            })
            .map_err(Into::into)
    }
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn snapshot(workspace: &Workspace, path: &str, control: &Control) -> Result<String> {
    let input = serde_json::to_vec(&json!({"operation":"snapshot","arguments":{"path":path}}))?;
    let output = process::run_with_input(
        &mut workspace.file_command().map_err(Failure::unavailable)?,
        &input,
        &Limits {
            timeout: Duration::from_secs(30),
            capture_bytes: 400_000,
            output_bytes: 1_000_000,
        },
        control,
    )
    .map_err(Failure::unavailable)?;
    if output.truncated || !matches!(output.termination,Termination::Exited(code) if code.success())
    {
        return Err(Failure::new(
            verification::failed_worker(&output.termination, output.truncated),
            format!("cannot snapshot {path}: {}", output.stderr),
        )
        .into());
    }
    let bytes: Vec<u8> = serde_json::from_str(&output.stdout).map_err(Failure::unavailable)?;
    anyhow::ensure!(
        bytes.len() <= 65_536,
        Failure::new(FailureKind::Unverified, "kernel source exceeds 64 KiB")
    );
    Ok(String::from_utf8(bytes).map_err(|e| Failure::new(FailureKind::Unverified, e))?)
}

enum Operation {
    Propose,
    Verify,
    Status,
}
struct ProjectTool {
    project: Arc<Project>,
    workspace: Arc<Workspace>,
    control: Arc<Control>,
    sig: ToolSig,
    operation: Operation,
}

pub fn register(
    registry: &mut Registry,
    policy: &mut Policy,
    workspace: &Arc<Workspace>,
    control: &Arc<Control>,
    project: &Arc<Project>,
) -> Result<()> {
    anyhow::ensure!(
        project.matches(workspace.path()),
        "project belongs to another workspace"
    );
    for (name,operation,description) in [
        ("propose_contract",Operation::Propose,"Propose a pure Rust transition's mathematical contract for host review. Only metadata and restricted mathematical terms are accepted, never Lean commands, proof scripts or checker paths. State has one to eight u64 fields and at most eight u64 parameters; specification uses mathematical Nat values and must return the next state. Each property is a Prop in s, parameters and out. Descriptions/assumptions are reviewed meaning, not proved equivalence to natural language. After a successful proposal pause for host review. Rust contains only the declared public struct/function: finite pure arithmetic, branches and checked/saturating operations; no imports, attributes, other calls, macros or loops."),
        ("verify_project",Operation::Verify,"Independently check the accepted contract's exact Rust source using the fixed offline Rust-to-Lean backend. No model-supplied checker/path/receipt/result. Tests alone do not suffice. Success retains source, proof and native library outside the writable coding copy; changed source needs fresh evidence."),
        ("project_status",Operation::Status,"Read the current host-derived proposal, accepted meaning and evidence freshness. Model prose or project-local proof/report files never make this status verified.")
    ] {
        let parameters = if matches!(operation,Operation::Propose) {
            json!({"type":"object","properties":{"contract":Contract::schema()},"required":["contract"],"additionalProperties":false})
        } else {json!({"type":"object","properties":{},"additionalProperties":false})};
        registry.register(Box::new(ProjectTool{project:Arc::clone(project),workspace:Arc::clone(workspace),control:Arc::clone(control),sig:ToolSig::new(name,description,parameters),operation}));
    }
    policy.grants.push(Grant::parse("process.spawn")?);
    Ok(())
}

impl Tool for ProjectTool {
    fn signature(&self) -> &ToolSig {
        &self.sig
    }
    fn requirements(&self, _: &Args) -> Vec<Requirement> {
        if matches!(self.operation, Operation::Propose) {
            return vec![Requirement::new(Capability::ProcessSpawn, None)];
        }
        let source = self.project.state.lock().ok().and_then(|s| {
            s.accepted
                .as_ref()
                .map(|a| format!("/workspace/{}", a.contract.source()))
        });
        let mut requirements: Vec<_> = source
            .as_deref()
            .map(|path| Requirement::new(Capability::FsRead, Some(path)))
            .into_iter()
            .collect();
        if matches!(self.operation, Operation::Verify) {
            requirements.push(Requirement::new(Capability::ProcessSpawn, None));
        }
        requirements
    }
    fn call(&self, args: &Args) -> std::result::Result<Value, ToolError> {
        let result = (|| -> Result<Summary> {
            match self.operation {
                Operation::Propose => {
                    anyhow::ensure!(args.len() == 1, "proposal accepts only contract data");
                    let contract = Contract::parse(&serde_json::to_string(
                        args.get("contract").context("contract is required")?,
                    )?)?;
                    self.project.propose(contract, &self.control)
                }
                Operation::Verify | Operation::Status => {
                    anyhow::ensure!(args.is_empty(),"checking/status accept no model-supplied contract, checker, receipt, path or result");
                    if matches!(self.operation, Operation::Verify) {
                        self.project.verify(&self.workspace, &self.control)
                    } else {
                        self.project
                            .status(&self.workspace, &self.control)
                            .map_err(Into::into)
                    }
                }
            }
        })();
        result
            .and_then(|s| Ok(serde_json::to_value(s)?))
            .map_err(|e| ToolError::Failed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn contract() -> Result<Contract> {
        Ok(Contract::parse(include_str!(
            "../../../verification/examples/counter.contract.json"
        ))?)
    }
    // Review tests isolate the host state transition; integration tests run
    // the mandatory mathematical preflight before a real proposal is exposed.
    fn draft(project: &Project, contract: Contract) -> Result<Reference> {
        let agreement = Agreement::new(&project.session, contract)?;
        let reference = agreement.reference(&project.session);
        project
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?
            .draft = Some(agreement);
        Ok(reference)
    }
    fn accepted(project: &Project) -> Result<Reference> {
        let reference = draft(project, contract()?)?;
        project.review(&reference)?;
        project.accept(&reference)?;
        Ok(reference)
    }

    #[test]
    fn only_a_new_host_turn_clears_an_operational_failure() -> Result<()> {
        let project = Project::new(Path::new("/sessions/failed"));
        for kind in [
            FailureKind::Unverified,
            FailureKind::Unavailable,
            FailureKind::Inconclusive,
        ] {
            project.record_failure(Failure::new(kind, "model text claims passed=true"));
            project.mark_dirty();
            assert_eq!(project.failure().is_some(), !kind.repairable());
            assert!(!project.stored_status()?.verified);
            project.begin_turn();
            assert!(project.failure().is_none());
        }
        Ok(())
    }

    #[test]
    fn host_review_and_acceptance_bind_the_displayed_bytes_and_session() -> Result<()> {
        let project = Project::new(Path::new("/sessions/one"));
        let original = contract()?;
        let reference = draft(&project, original.clone())?;
        assert!(project.accept(&reference).is_err());
        let mut foreign = reference.clone();
        foreign.session.push_str("-other");
        assert!(project.review(&foreign).is_err());
        project.review(&reference)?;
        assert!(project.accept(&foreign).is_err());
        project.accept(&reference)?;
        assert!(project.accepted() && project.take_autorun());
        assert!(!project.take_autorun());

        let mut replacement = serde_json::to_value(&original)?;
        *replacement
            .get_mut("specification")
            .context("no specification")? = json!("s");
        *replacement.get_mut("examples").context("no examples")? = json!([]);
        let replacement = Contract::parse(&replacement.to_string())?;
        let pending = draft(&project, replacement.clone())?;
        let summary = project.stored_status()?;
        assert!(!summary.accepted && !summary.verified);
        assert_eq!(summary.accepted_contract, Some(original.clone()));
        assert_eq!(summary.contract, Some(replacement.clone()));
        assert!(summary.review.contains("Previously accepted contract"));
        assert!(project.accept(&reference).is_err());
        project.review(&pending)?;
        let mut changed = serde_json::to_value(replacement)?;
        *changed.get_mut("description").context("no description")? =
            json!("A new meaning requires a new review.");
        let changed = draft(&project, Contract::parse(&changed.to_string())?)?;
        assert!(project.accept(&pending).is_err());
        assert!(project.accept(&changed).is_err());
        assert_eq!(
            project.stored_status()?.accepted_contract,
            Some(original.clone())
        );
        project.reject(&changed)?;
        assert!(!project.pending() && project.accepted());
        assert_eq!(project.stored_status()?.contract, Some(original));
        assert_eq!(project.current_reference()?.contract, reference.contract);
        Ok(())
    }

    #[test]
    fn editing_discards_evidence_before_another_status_read() -> Result<()> {
        let project = Project::new(Path::new("/sessions/editing"));
        accepted(&project)?;
        let (subject, ticket) = {
            let mut state = project
                .state
                .lock()
                .map_err(|_| anyhow::anyhow!("project unavailable"))?;
            let State { accepted, jobs, .. } = &mut *state;
            let agreement = accepted.as_mut().context("no contract")?;
            let subject = agreement.subject(b"checked inputs".to_vec());
            assert!(agreement.gate.refresh(Origin::Controller, subject.clone()));
            let ticket = jobs
                .start(Origin::Controller, &agreement.gate, &subject)
                .context("no job")?;
            assert!(jobs.complete(
                &mut agreement.gate,
                Origin::Checker,
                &ticket,
                &subject,
                CheckResult {
                    subject: subject.clone(),
                    position: 0,
                    obligation: agreement.obligation.clone(),
                    passed: true,
                }
            ));
            assert!(agreement.gate.eligible(&subject));
            let ticket = jobs
                .start(Origin::Controller, &agreement.gate, &subject)
                .context("no job")?;
            (subject, ticket)
        };
        project.mark_dirty();
        let mut state = project
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        assert!(state.dirty && !state.jobs.is_active());
        let State { accepted, jobs, .. } = &mut *state;
        let agreement = accepted.as_mut().context("no contract")?;
        assert!(agreement.gate.is_accepted());
        assert!(agreement.gate.refresh(Origin::Controller, subject.clone()));
        assert!(!agreement.gate.eligible(&subject));
        assert!(!jobs.finish(Origin::Checker, &agreement.gate, &ticket, &subject));
        Ok(())
    }

    #[test]
    fn cancelled_jobs_stay_cancelled_after_the_process_flag_is_reset() -> Result<()> {
        let project = Project::new(Path::new("/sessions/cancelled"));
        accepted(&project)?;
        let (subject, ticket) = {
            let mut state = project
                .state
                .lock()
                .map_err(|_| anyhow::anyhow!("project unavailable"))?;
            let State { accepted, jobs, .. } = &mut *state;
            let agreement = accepted.as_ref().context("no contract")?;
            let subject = agreement.subject(b"same restored inputs".to_vec());
            let ticket = jobs
                .start(Origin::Controller, &agreement.gate, &subject)
                .context("no job")?;
            (subject, ticket)
        };
        let control = Control::new(Arc::new(AtomicBool::new(false)));
        project.cancel(&control);
        assert!(control.flag().load(Ordering::SeqCst));
        control.flag().store(false, Ordering::SeqCst);
        let mut state = project
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        let State { accepted, jobs, .. } = &mut *state;
        let agreement = accepted.as_ref().context("no contract")?;
        let next = jobs
            .start(Origin::Controller, &agreement.gate, &subject)
            .context("no next job")?;
        assert!(!jobs.finish(Origin::Checker, &agreement.gate, &ticket, &subject));
        assert!(jobs.finish(Origin::Checker, &agreement.gate, &next, &subject));
        Ok(())
    }
}
