//! Host-reviewed mathematical meaning and its input-bound verified artifact.
//! Models propose drafts and implementations; only host review accepts meaning.

use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use meden_agent::{
    tool::{Args, Prepared, Tool, ToolError, ToolSig},
    Registry,
};
use meden_core::{
    check::{Preparation, Running},
    evidence::{
        ContractView, Contracts, EvidenceKind, EvidenceOrigin as Origin, Inspection, Obligation,
        Subject,
    },
    Capability, Grant, Policy, Requirement,
};
use meden_runtime::{
    process::{Control, Limits},
    transfer::Export,
    verification::{
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
    pub revision: u64,
    pub session: String,
    pub contract: String,
}

#[derive(Clone, Serialize)]
pub struct Summary {
    pub reference: Reference,
    pub accepted_reference: Option<Reference>,
    pub proposed: bool,
    /// Acceptance of the displayed contract, never of a pending replacement.
    pub accepted: bool,
    pub verified: bool,
    pub verification_failure: Option<FailureKind>,
    pub verification_diagnostic: Option<String>,
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
    view: ContractView,
}

pub struct Access {
    inspection: Inspection,
    agreement: Option<Agreement>,
}

impl Access {
    pub fn requirements(&self) -> Vec<Requirement> {
        self.agreement
            .as_ref()
            .map(|agreement| {
                Requirement::new(
                    Capability::FsRead,
                    Some(&format!("/workspace/{}", agreement.contract.source())),
                )
            })
            .into_iter()
            .collect()
    }
}

impl Agreement {
    fn parse(view: ContractView) -> Result<Self> {
        let contract = Contract::parse(std::str::from_utf8(&view.subject.contract)?)?;
        anyhow::ensure!(
            contract.canonical()? == view.subject.contract,
            "stored meaning is not canonical"
        );
        anyhow::ensure!(
            view.obligations.len() == 1,
            "unsupported project obligation set"
        );
        Ok(Self { contract, view })
    }
    fn subject(&self, inputs: Vec<u8>) -> Subject {
        Subject {
            inputs,
            ..self.view.subject.clone()
        }
    }
    fn reference(&self, session: &str) -> Reference {
        Reference {
            revision: self.view.revision,
            session: session.to_owned(),
            contract: hash(&self.view.subject.contract),
        }
    }
}

fn obligation(contract: &Contract) -> Result<Obligation> {
    Ok(Obligation {
        statement: contract.canonical()?,
        kind: EvidenceKind::Refinement,
        checker: kernel::checker_id(contract)?.into_bytes(),
    })
}

struct State {
    contracts: Contracts,
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
        let contracts = Contracts::new(session.as_bytes().to_vec());
        Self {
            session,
            bundle: crate::verifier::manifest(),
            storage: root
                .parent()
                .and_then(Path::parent)
                .unwrap_or(root)
                .join("artifacts"),
            state: Mutex::new(State {
                contracts,
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
                "Local verification tools are not prepared. Restart the TUI for automatic setup, or run `meden setup` before a CLI session.",
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
    pub fn propose(&self, contract: &Contract, control: &Control) -> Result<Summary> {
        let bytes = contract.canonical()?;
        let required = obligation(contract)?;
        {
            let state = self
                .state
                .lock()
                .map_err(|_| anyhow::anyhow!("project unavailable"))?;
            if state.contracts.displayed().is_some_and(|v| {
                v.subject.contract == bytes && v.obligations == vec![required.clone()]
            }) {
                return self.summary(&state, false);
            }
        }
        let result = self.bundle().and_then(|bundle| {
            kernel::check_contract(
                contract,
                &bundle,
                &std::env::current_exe().map_err(Failure::unavailable)?,
                control,
            )
        });
        if let Err(failure) = result {
            self.record_failure(failure.clone());
            return Err(failure.into());
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        control
            .with_active(|| {
                if !state
                    .contracts
                    .propose(Origin::Controller, bytes, vec![required])
                {
                    return Err(io::Error::other(
                        "contract proposal refused or generations exhausted",
                    ));
                }
                state.autorun = false;
                state.failure = None;
                self.summary(&state, false).map_err(io::Error::other)
            })
            .map_err(Into::into)
    }
    fn reference(&self, agreement: &Agreement, reference: &Reference) -> Result<()> {
        anyhow::ensure!(
            reference.session == self.session
                && reference.revision == agreement.view.revision
                && reference.contract == hash(&agreement.view.subject.contract),
            "contract belongs to another session or is no longer the displayed proposal"
        );
        Ok(())
    }
    pub fn review(&self, reference: &Reference) -> Result<Summary> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        let agreement = Agreement::parse(
            state
                .contracts
                .displayed()
                .context("no contract proposal")?,
        )?;
        self.reference(&agreement, reference)?;
        anyhow::ensure!(
            state.contracts.review(
                Origin::Host,
                reference.session.as_bytes(),
                reference.revision
            ),
            "review refused"
        );
        self.summary(&state, false)
    }
    pub fn accept(&self, reference: &Reference) -> Result<Summary> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        let agreement = Agreement::parse(
            state
                .contracts
                .displayed()
                .context("no contract proposal")?,
        )?;
        self.reference(&agreement, reference)?;
        anyhow::ensure!(
            state.contracts.accept(
                Origin::Host,
                reference.session.as_bytes(),
                reference.revision
            ),
            "exact pending contract must be reviewed before host acceptance"
        );
        state.artifact = None;
        state.failure = None;
        state.dirty = true;
        state.autorun = true;
        self.summary(&state, false)
    }
    pub fn reject(&self, reference: &Reference) -> Result<Summary> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        let agreement = Agreement::parse(
            state
                .contracts
                .displayed()
                .context("no contract proposal")?,
        )?;
        self.reference(&agreement, reference)?;
        anyhow::ensure!(
            state.contracts.reject(
                Origin::Host,
                reference.session.as_bytes(),
                reference.revision
            ),
            "exact pending contract required"
        );
        self.summary(&state, false)
    }
    pub fn pending(&self) -> bool {
        self.state.lock().is_ok_and(|s| s.contracts.pending())
    }
    pub fn accepted(&self) -> bool {
        self.state.lock().is_ok_and(|s| s.contracts.is_accepted())
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
            state.contracts.is_accepted(),
            "host acceptance of the current contract is required"
        );
        let agreement =
            Agreement::parse(state.contracts.accepted().context("no accepted contract")?)?;
        Ok(format!("Proceed under the host-accepted contract. Implement {} and use verify_project. Repair failures; report unknown if verification cannot finish. Do not change the contract or checker. Accepted meaning (host-owned):\n{}",agreement.contract.source(),agreement.contract.review_text()))
    }
    pub fn mark_dirty(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.contracts.edit(Origin::Controller);
            state.dirty = true;
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
            state.contracts.invalidate(Origin::Controller);
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
        self.current_status(workspace, control)
            .map_err(|error| self.status_failure(error))
    }
    fn status_failure(&self, error: anyhow::Error) -> Failure {
        let failure = error
            .downcast_ref::<Failure>()
            .cloned()
            .unwrap_or_else(|| Failure::unavailable(error));
        self.record_failure(failure.clone());
        failure
    }
    fn current_status(&self, workspace: &Workspace, control: &Control) -> Result<Summary> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        self.current_status_locked(
            &mut state,
            workspace,
            control,
            &crate::workspace::file_policy()?,
        )
    }
    fn current_status_locked(
        &self,
        state: &mut State,
        workspace: &Workspace,
        control: &Control,
        rules: &Policy,
    ) -> Result<Summary> {
        let result = (|| {
            if state.contracts.pending() || state.contracts.is_active() {
                return self.summary(state, false);
            }
            let Some(view) = state.contracts.accepted() else {
                return self.summary(state, false);
            };
            let Some(artifact) = state.artifact.as_ref() else {
                return self.summary(state, false);
            };
            if let Err(error) = artifact.validate() {
                state.contracts.edit(Origin::Controller);
                state.dirty = true;
                return Err(Failure::unavailable(error).into());
            }
            let agreement = Agreement::parse(view)?;
            let current = Self::binding(
                &agreement,
                &artifact.report,
                &snapshot(workspace, agreement.contract.source(), control, rules)?,
            )?;
            let verified = state.contracts.refresh(
                Origin::Controller,
                agreement.view.revision,
                current.inputs.clone(),
            ) && state
                .contracts
                .eligible(agreement.view.revision, current.inputs);
            state.dirty = false;
            self.summary(state, verified)
        })();
        if let Err(error) = &result {
            state.failure = Some(
                error
                    .downcast_ref::<Failure>()
                    .cloned()
                    .unwrap_or_else(|| Failure::unavailable(error)),
            );
        }
        result
    }
    pub fn prepare_access(&self) -> Result<Access> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        let inspection = state.contracts.inspect();
        let agreement = inspection
            .accepted()
            .cloned()
            .map(Agreement::parse)
            .transpose()?;
        Ok(Access {
            inspection,
            agreement,
        })
    }
    fn with_access<T>(
        &self,
        access: Access,
        execute: impl FnOnce(&mut State, Option<Agreement>) -> Result<T>,
    ) -> Result<T> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        let expected = access
            .inspection
            .consume(&state.contracts)
            .context("project context changed after permission preparation")?;
        anyhow::ensure!(
            expected.as_ref() == access.agreement.as_ref().map(|a| &a.view),
            "prepared contract differs from its frozen inspection"
        );
        execute(&mut state, access.agreement)
    }
    fn status_access(
        &self,
        access: Access,
        workspace: &Workspace,
        control: &Control,
        rules: &Policy,
    ) -> Result<Summary> {
        self.with_access(access, |state, _| {
            self.current_status_locked(state, workspace, control, rules)
        })
        .map_err(|error| self.status_failure(error).into())
    }
    pub fn prepare_file_export(&self, target: &str, bytes: &[u8]) -> Result<Export> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        anyhow::ensure!(
            state.contracts.displayed().is_none(),
            "a project contract requires a checked artifact package; use propose_artifact_export"
        );
        Ok(Export::prepare(&state.contracts, target, bytes)?)
    }
    pub fn prepare_artifact_export(
        &self,
        access: Access,
        workspace: &Workspace,
        control: &Control,
        target: &str,
        rules: &Policy,
    ) -> Result<Export> {
        anyhow::ensure!(
            self.matches(workspace.path()),
            "project belongs to another workspace"
        );
        self.with_access(access, |state, _| {
            anyhow::ensure!(
                self.current_status_locked(state, workspace, control, rules)?
                    .verified,
                "artifact export requires current complete project verification"
            );
            let bytes = match state
                .artifact
                .as_ref()
                .context("no checked artifact")?
                .package()
            {
                Ok(bytes) => bytes,
                Err(error) => {
                    state.contracts.edit(Origin::Controller);
                    state.dirty = true;
                    let failure = Failure::unavailable(error);
                    state.failure = Some(failure.clone());
                    return Err(failure.into());
                }
            };
            Ok(control.with_active(|| Export::prepare(&state.contracts, target, &bytes))?)
        })
    }
    pub fn review_export(&self, export: &Export) -> Result<()> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        Ok(export.review_from_host(&state.contracts)?)
    }
    pub fn reject_export(&self, export: &Export) -> Result<()> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        Ok(export.reject_from_host(&state.contracts)?)
    }
    pub fn send_export(
        &self,
        export: &Export,
        workspace: &Workspace,
        control: &Control,
    ) -> Result<Option<u16>> {
        anyhow::ensure!(
            self.matches(workspace.path()),
            "project belongs to another workspace"
        );
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        self.current_status_locked(
            &mut state,
            workspace,
            control,
            &crate::workspace::file_policy()?,
        )?;
        Ok(control.with_active(|| {
            export.send(
                &state.contracts,
                self.session.as_bytes(),
                Duration::from_secs(60),
            )
        })?)
    }
    pub fn stored_status(&self) -> Result<Summary> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        self.summary(&state, false)
    }
    fn summary(&self, state: &State, verified: bool) -> Result<Summary> {
        let displayed = state
            .contracts
            .displayed()
            .map(Agreement::parse)
            .transpose()?;
        let accepted = state
            .contracts
            .accepted()
            .map(Agreement::parse)
            .transpose()?;
        let reference = displayed.as_ref().map_or_else(
            || Reference {
                revision: 0,
                session: self.session.clone(),
                contract: String::new(),
            },
            |a| a.reference(&self.session),
        );
        let mut review = displayed
            .as_ref()
            .map_or_else(String::new, |a| a.contract.review_text());
        if state.contracts.pending() {
            if let Some(accepted) = &accepted {
                review = format!("Previously accepted contract (retained until host accepts a replacement):\n{}\n\nProposed replacement:\n{review}",accepted.contract.review_text());
            }
        }
        let artifact = if state.contracts.pending() {
            None
        } else {
            state.artifact.as_ref()
        };
        Ok(Summary {
            reference,
            accepted_reference: accepted.as_ref().map(|a| a.reference(&self.session)),
            proposed: displayed.is_some(),
            accepted: state.contracts.is_accepted(),
            verified: verified && !state.dirty && !state.contracts.pending(),
            verification_failure: state.failure.as_ref().map(|f| f.kind),
            verification_diagnostic: state
                .failure
                .as_ref()
                .map(|failure| failure.to_string().chars().take(8000).collect()),
            contract: displayed.as_ref().map(|a| a.contract.clone()),
            accepted_contract: accepted.map(|a| a.contract),
            review,
            source_path: displayed
                .as_ref()
                .map_or_else(String::new, |a| a.contract.source().to_owned()),
            api: displayed
                .as_ref()
                .map_or_else(String::new, |a| a.contract.api()),
            artifact: artifact.map(|a| a.directory.clone()),
            source_sha256: artifact.map(|a| a.report.source_sha256.clone()),
            artifact_sha256: artifact.map(|a| a.report.artifact_sha256.clone()),
        })
    }
    pub fn verify(
        &self,
        access: Access,
        workspace: &Workspace,
        control: &Control,
        rules: &Policy,
    ) -> Result<Summary> {
        let result = self.verify_current(access, workspace, control, rules);
        if let Err(error) = &result {
            let kind = error
                .downcast_ref::<Failure>()
                .map_or(FailureKind::Inconclusive, |f| f.kind);
            self.record_failure(Failure::new(kind, error));
        }
        result
    }
    fn verify_current(
        &self,
        access: Access,
        workspace: &Workspace,
        control: &Control,
        rules: &Policy,
    ) -> Result<Summary> {
        let preparation = self.with_access(access, |state, agreement| {
            let agreement = agreement.context("no prepared accepted contract")?;
            let preparation =
                Preparation::begin(&mut state.contracts, agreement.view.clone(), agreement)
                    .context("host acceptance of the exact prepared contract is required")?;
            state.dirty = true;
            state.failure = None;
            Ok(preparation)
        })?;
        let source = snapshot(
            workspace,
            preparation.data().contract.source(),
            control,
            rules,
        )?;
        let bundle = self.bundle()?;
        let prepared = kernel::prepare_kernel(
            &source,
            &preparation.data().contract,
            &bundle,
            &std::env::current_exe().map_err(Failure::unavailable)?,
            control,
        )?;
        let running = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow::anyhow!("project unavailable"))?;
            control.with_active(|| {
                preparation
                    .issue(&mut state.contracts, serde_json::to_vec(prepared.inputs())?)
                    .ok_or_else(|| {
                        io::Error::other("job issue refused or contract changed during preparation")
                    })
            })?
        };
        let ticket = running.ticket().clone();
        let result = self.run_job(prepared, running, workspace, control, rules);
        if result.is_err() {
            if let Ok(mut state) = self.state.lock() {
                state.contracts.abort(Origin::Controller, &ticket);
            }
        }
        result
    }
    fn run_job(
        &self,
        prepared: kernel::PreparedKernel<'_>,
        running: Running<Agreement>,
        workspace: &Workspace,
        control: &Control,
        rules: &Policy,
    ) -> Result<Summary> {
        let agreement = running.data();
        let checked = prepared.check(control)?;
        let fresh = snapshot(workspace, agreement.contract.source(), control, rules)?;
        let mut inputs = checked.inputs();
        inputs.source_sha256 = hash(fresh.as_bytes());
        let subject = Self::binding(agreement, checked.report(), &fresh)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        control
            .with_active(|| {
                let observed_checker = checked.report().checker_sha256.as_bytes().to_vec();
                let staged = checked.stage(&self.storage)?;
                let staged = running
                    .complete(
                        &mut state.contracts,
                        serde_json::to_vec(&inputs)?,
                        subject.inputs,
                        observed_checker,
                        staged,
                    )
                    .ok_or_else(|| io::Error::other("evidence correspondence refused"))?;
                state.artifact = Some(staged.commit());
                state.dirty = false;
                self.summary(&state, true).map_err(io::Error::other)
            })
            .map_err(Into::into)
    }
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn snapshot(
    workspace: &Workspace,
    path: &str,
    control: &Control,
    rules: &Policy,
) -> Result<String> {
    let resolved =
        crate::workspace::logical_path(path).context("kernel source leaves /workspace")?;
    let request = meden_core::file::Request::new(
        meden_core::file::Operation::Snapshot,
        resolved.to_string_lossy().as_ref().into(),
        Args::new(),
    );
    let value = meden_runtime::file::run(
        workspace,
        control,
        &request,
        rules,
        &Limits {
            timeout: Duration::from_secs(30),
            capture_bytes: 400_000,
            output_bytes: 1_000_000,
        },
    )
    .map_err(|error| Failure::new(meden_runtime::file::verification_kind(&error), error))?;
    let bytes: Vec<u8> = serde_json::from_value(value).map_err(Failure::unavailable)?;
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
    fn prepare(&self, args: Args) -> std::result::Result<Prepared<'_>, ToolError> {
        let access = if matches!(self.operation, Operation::Propose) {
            None
        } else {
            Some(
                self.project
                    .prepare_access()
                    .map_err(|error| ToolError::failed(error.to_string()))?,
            )
        };
        let mut requirements = access.as_ref().map_or_else(Vec::new, Access::requirements);
        if matches!(self.operation, Operation::Propose | Operation::Verify) {
            requirements.push(Requirement::new(Capability::ProcessSpawn, None));
        }
        Ok(Prepared::new(
            ((args, access), requirements),
            |(args, access), rules| self.execute(&args, access, rules),
        ))
    }
}

impl ProjectTool {
    fn execute(
        &self,
        args: &Args,
        access: Option<Access>,
        rules: &Policy,
    ) -> std::result::Result<Value, ToolError> {
        let result = (|| -> Result<Summary> {
            match self.operation {
                Operation::Propose => {
                    anyhow::ensure!(args.len() == 1, "proposal accepts only contract data");
                    let contract = Contract::parse(&serde_json::to_string(
                        args.get("contract").context("contract is required")?,
                    )?)?;
                    self.project.propose(&contract, &self.control)
                }
                Operation::Verify | Operation::Status => {
                    anyhow::ensure!(args.is_empty(),"checking/status accept no model-supplied contract, checker, receipt, path or result");
                    let access = access.context("missing prepared project inspection")?;
                    if matches!(self.operation, Operation::Verify) {
                        anyhow::ensure!(
                            self.project.accepted(),
                            "verification requires a host-accepted contract; call propose_contract with the mathematical contract first, then pause for host review. This call did not start a checker or create evidence"
                        );
                        self.project
                            .verify(access, &self.workspace, &self.control, rules)
                    } else {
                        self.project
                            .status_access(access, &self.workspace, &self.control, rules)
                    }
                }
            }
        })();
        result
            .and_then(|s| Ok(serde_json::to_value(s)?))
            .map_err(|e| ToolError::failed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meden_core::evidence::CheckResult;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn stale_or_mismatched_prepared_access_never_enters_the_io_callback() -> Result<()> {
        let root = tempfile::tempdir()?;
        let project = Project::new(root.path());
        let empty = project.prepare_access()?;
        accepted(&project)?;
        let ran = std::cell::Cell::new(false);
        assert!(project
            .with_access(empty, |_, _| {
                ran.set(true);
                Ok(())
            })
            .is_err());
        assert!(!ran.get());

        let old = project.prepare_access()?;
        assert_eq!(
            old.requirements(),
            vec![Requirement::new(
                Capability::FsRead,
                Some("/workspace/counter.rs")
            )]
        );
        let mut replacement: Value = serde_json::from_str(include_str!(
            "../../../verification/examples/counter.contract.json"
        ))?;
        *replacement
            .get_mut("source_path")
            .context("missing source path")? = json!("private.rs");
        let replacement = Contract::parse(&serde_json::to_string(&replacement)?)?;
        let reference = draft(&project, &replacement)?;
        project.review(&reference)?;
        project.accept(&reference)?;
        assert!(project
            .with_access(old, |_, _| {
                ran.set(true);
                Ok(())
            })
            .is_err());
        assert!(!ran.get());

        let mut mismatched = project.prepare_access()?;
        mismatched
            .agreement
            .as_mut()
            .context("no agreement")?
            .view
            .subject
            .contract = b"forged".to_vec();
        assert!(project
            .with_access(mismatched, |_, _| {
                ran.set(true);
                Ok(())
            })
            .is_err());
        assert!(!ran.get());

        let fresh = project.prepare_access()?;
        assert_eq!(
            fresh.requirements(),
            vec![Requirement::new(
                Capability::FsRead,
                Some("/workspace/private.rs")
            )]
        );
        let source = project.with_access(fresh, |_, agreement| {
            ran.set(true);
            Ok(agreement
                .context("no prepared contract")?
                .contract
                .source()
                .to_owned())
        })?;
        assert!(ran.get());
        assert_eq!(source, "private.rs");
        Ok(())
    }

    fn contract() -> Result<Contract> {
        Ok(Contract::parse(include_str!(
            "../../../verification/examples/counter.contract.json"
        ))?)
    }
    // Review tests isolate the host state transition; integration tests run
    // the mandatory mathematical preflight before a real proposal is exposed.
    fn draft(project: &Project, contract: &Contract) -> Result<Reference> {
        let mut state = project
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        anyhow::ensure!(state.contracts.propose(
            Origin::Controller,
            contract.canonical()?,
            vec![obligation(contract)?]
        ));
        let agreement = Agreement::parse(state.contracts.displayed().context("no draft")?)?;
        Ok(agreement.reference(&project.session))
    }
    fn accepted(project: &Project) -> Result<Reference> {
        let reference = draft(project, &contract()?)?;
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
        let reference = draft(&project, &original)?;
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
        let pending = draft(&project, &replacement)?;
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
        let changed = draft(&project, &Contract::parse(&changed.to_string())?)?;
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
    fn reproposing_identical_bytes_cannot_reuse_a_rejected_reference() -> Result<()> {
        let project = Project::new(Path::new("/sessions/reproposal"));
        let original = contract()?;
        let old = draft(&project, &original)?;
        project.review(&old)?;
        project.reject(&old)?;
        let new = draft(&project, &original)?;
        assert_eq!(new.contract, old.contract);
        assert_eq!(new.session, old.session);
        assert!(new.revision > old.revision);
        assert!(project.review(&old).is_err());
        assert!(project.accept(&old).is_err());
        assert!(project.reject(&old).is_err());
        assert!(project.accept(&new).is_err());
        project.review(&new)?;
        project.accept(&new)?;
        assert!(project.accepted());
        Ok(())
    }

    #[test]
    fn editing_discards_evidence_before_another_status_read() -> Result<()> {
        let project = Project::new(Path::new("/sessions/editing"));
        let reference = accepted(&project)?;
        let (subject, ticket, result) = {
            let mut state = project
                .state
                .lock()
                .map_err(|_| anyhow::anyhow!("project unavailable"))?;
            let agreement = Agreement::parse(state.contracts.accepted().context("no contract")?)?;
            let subject = agreement.subject(b"checked inputs".to_vec());
            let ticket = state
                .contracts
                .start(
                    Origin::Controller,
                    reference.revision,
                    subject.inputs.clone(),
                )
                .context("no job")?;
            let result = CheckResult {
                subject: subject.clone(),
                position: 0,
                obligation: agreement
                    .view
                    .obligations
                    .first()
                    .context("no obligation")?
                    .clone(),
                passed: true,
            };
            assert!(state.contracts.complete(
                Origin::Checker,
                reference.revision,
                &ticket,
                subject.inputs.clone(),
                result.clone()
            ));
            assert!(state
                .contracts
                .eligible(reference.revision, subject.inputs.clone()));
            let ticket = state
                .contracts
                .start(
                    Origin::Controller,
                    reference.revision,
                    subject.inputs.clone(),
                )
                .context("no job")?;
            (subject, ticket, result)
        };
        project.mark_dirty();
        let mut state = project
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        assert!(state.dirty && !state.contracts.is_active());
        assert!(state.contracts.is_accepted());
        assert!(state.contracts.refresh(
            Origin::Controller,
            reference.revision,
            subject.inputs.clone()
        ));
        assert!(!state
            .contracts
            .eligible(reference.revision, subject.inputs.clone()));
        assert!(!state.contracts.complete(
            Origin::Checker,
            reference.revision,
            &ticket,
            subject.inputs,
            result
        ));
        Ok(())
    }

    #[test]
    fn cancelled_jobs_stay_cancelled_after_the_process_flag_is_reset() -> Result<()> {
        let project = Project::new(Path::new("/sessions/cancelled"));
        let reference = accepted(&project)?;
        let (subject, ticket, result) = {
            let mut state = project
                .state
                .lock()
                .map_err(|_| anyhow::anyhow!("project unavailable"))?;
            let agreement = Agreement::parse(state.contracts.accepted().context("no contract")?)?;
            let subject = agreement.subject(b"same restored inputs".to_vec());
            let ticket = state
                .contracts
                .start(
                    Origin::Controller,
                    reference.revision,
                    subject.inputs.clone(),
                )
                .context("no job")?;
            let result = CheckResult {
                subject: subject.clone(),
                position: 0,
                obligation: agreement
                    .view
                    .obligations
                    .first()
                    .context("no obligation")?
                    .clone(),
                passed: true,
            };
            (subject, ticket, result)
        };
        let control = Control::new(Arc::new(AtomicBool::new(false)));
        project.cancel(&control);
        assert!(control.flag().load(Ordering::SeqCst));
        control.flag().store(false, Ordering::SeqCst);
        let mut state = project
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        let next = state
            .contracts
            .start(
                Origin::Controller,
                reference.revision,
                subject.inputs.clone(),
            )
            .context("no next job")?;
        assert!(!state.contracts.complete(
            Origin::Checker,
            reference.revision,
            &ticket,
            subject.inputs.clone(),
            result.clone()
        ));
        assert!(state.contracts.complete(
            Origin::Checker,
            reference.revision,
            &next,
            subject.inputs,
            result
        ));
        Ok(())
    }
}
