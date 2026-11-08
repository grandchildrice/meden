//! One host-reviewed project contract and its input-bound verified artifact.
//! Models propose and check; they cannot accept contracts or supply receipts.

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
    evidence::{EvidenceKind, EvidenceOrigin as Origin, Gate, Obligation, Receipt, Subject},
    Capability, Grant, Policy, Requirement,
};
use meden_runtime::{
    process::{self, Control, Limits, Termination},
    verification::{self, Artifact, Bundle, Report},
    workspace::Workspace,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const KIND: &str = "rust-ledger-v1";
const API: &str = "pub struct LedgerState { pub debit: u64, pub credit: u64, pub sequence: u64 }\npub fn transfer(mut state: LedgerState, actor: u64, id: u64, amount: u64) -> LedgerState { state }";

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub session: String,
    pub contract: String,
}

#[derive(Serialize)]
pub struct Summary {
    pub reference: Reference,
    pub proposed: bool,
    pub accepted: bool,
    pub verified: bool,
    pub contract: &'static str,
    pub source: &'static str,
    pub api: &'static str,
    pub artifact: Option<PathBuf>,
    pub source_sha256: Option<String>,
    pub artifact_sha256: Option<String>,
}

struct State {
    gate: Option<Gate>,
    dirty: bool,
    autorun: bool,
    artifact: Option<Artifact>,
}

pub struct Project {
    reference: Reference,
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
        Self {
            reference: Reference {
                session,
                contract: KIND.into(),
            },
            bundle: std::env::var_os("MEDEN_VERIFIER_BUNDLE").map(PathBuf::from),
            storage: root
                .parent()
                .and_then(Path::parent)
                .unwrap_or(root)
                .join("artifacts"),
            state: Mutex::new(State {
                gate: None,
                dirty: false,
                autorun: false,
                artifact: None,
            }),
        }
    }

    pub fn matches(&self, root: &Path) -> bool {
        self.reference.session == Self::new(root).reference.session
    }

    fn obligation() -> Obligation {
        Obligation {
            statement: verification::LEDGER_CONTRACT.as_bytes().to_vec(),
            kind: EvidenceKind::Refinement,
            checker: verification::ledger_checker_id().into_bytes(),
        }
    }

    fn subject(&self, inputs: Vec<u8>) -> Subject {
        Subject {
            session: self.reference.session.as_bytes().to_vec(),
            contract: verification::LEDGER_CONTRACT.as_bytes().to_vec(),
            inputs,
        }
    }

    fn binding(&self, report: &Report, source: &str) -> Result<Subject> {
        Ok(self.subject(serde_json::to_vec(&json!({"source":hash(source.as_bytes()),"artifact":report.artifact_sha256,
            "bundle":report.bundle_sha256,"checker":report.checker_sha256,"worker":report.worker_sha256}))?))
    }

    pub fn propose(&self) -> Result<Summary> {
        anyhow::ensure!(self.bundle.is_some(), "verification needs a host-prepared MEDEN_VERIFIER_BUNDLE before confidential work; no checker download or fallback is performed");
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        if state.gate.is_none() {
            state.gate = Some(Gate::propose(
                self.subject(b"pending".to_vec()),
                vec![Self::obligation()],
            ));
        }
        Ok(self.summary(&state, false))
    }

    fn reference(&self, reference: &Reference) -> Result<()> {
        anyhow::ensure!(
            reference.session == self.reference.session && reference.contract == KIND,
            "contract belongs to another session or unsupported revision"
        );
        Ok(())
    }

    pub fn review(&self, reference: &Reference) -> Result<Summary> {
        self.reference(reference)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        anyhow::ensure!(
            state
                .gate
                .as_mut()
                .context("no contract proposal")?
                .review(Origin::Host),
            "review refused"
        );
        Ok(self.summary(&state, false))
    }

    pub fn accept(&self, reference: &Reference) -> Result<Summary> {
        self.reference(reference)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        anyhow::ensure!(
            state
                .gate
                .as_mut()
                .context("no contract proposal")?
                .accept(Origin::Host),
            "contract must be reviewed before host acceptance"
        );
        state.autorun = true;
        Ok(self.summary(&state, false))
    }

    pub fn pending(&self) -> bool {
        self.state
            .lock()
            .is_ok_and(|s| s.gate.as_ref().is_some_and(|g| !g.is_accepted()))
    }
    pub fn accepted(&self) -> bool {
        self.state
            .lock()
            .is_ok_and(|s| s.gate.as_ref().is_some_and(Gate::is_accepted))
    }
    pub fn current_reference(&self) -> Reference {
        self.reference.clone()
    }
    pub fn take_autorun(&self) -> bool {
        self.state
            .lock()
            .is_ok_and(|mut s| std::mem::take(&mut s.autorun))
    }

    pub fn mark_dirty(&self) {
        if let Ok(mut state) = self.state.lock() {
            // Invalidate at the permitting decision, before any write runs.
            // Restoring the old bytes before a status read must not revive
            // receipts discarded by a development operation.
            if let Some(gate) = &mut state.gate {
                gate.refresh(Origin::Controller, self.subject(b"editing".to_vec()));
            }
            state.dirty = true;
        }
    }

    pub fn status(&self, workspace: &Workspace, control: &Control) -> Result<Summary> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        let current = if let Some(artifact) = &state.artifact {
            self.binding(&artifact.report, &snapshot(workspace, control)?)?
        } else {
            return Ok(self.summary(&state, false));
        };
        let verified = state.gate.as_mut().is_some_and(|gate| {
            gate.refresh(Origin::Controller, current.clone()) && gate.eligible(&current)
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
        Summary {
            reference: self.reference.clone(),
            proposed: state.gate.is_some(),
            accepted: state.gate.as_ref().is_some_and(Gate::is_accepted),
            verified: verified && !state.dirty,
            contract: verification::LEDGER_CONTRACT,
            source: "ledger.rs",
            api: API,
            artifact: state.artifact.as_ref().map(|a| a.directory.clone()),
            source_sha256: state
                .artifact
                .as_ref()
                .map(|a| a.report.source_sha256.clone()),
            artifact_sha256: state
                .artifact
                .as_ref()
                .map(|a| a.report.artifact_sha256.clone()),
        }
    }

    pub fn verify(&self, workspace: &Workspace, control: &Control) -> Result<Summary> {
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow::anyhow!("project unavailable"))?;
            anyhow::ensure!(
                state.gate.as_ref().is_some_and(Gate::is_accepted),
                "host contract acceptance is required before verification"
            );
            anyhow::ensure!(
                state
                    .gate
                    .as_mut()
                    .context("no contract")?
                    .refresh(Origin::Controller, self.subject(b"checking".to_vec())),
                "job binding refused"
            );
            state.dirty = true;
        }
        let source = snapshot(workspace, control)?;
        let bundle = Bundle::load(
            self.bundle
                .as_deref()
                .context("no prepared checker bundle")?,
        )?;
        let checked =
            verification::verify_ledger(&source, &bundle, &std::env::current_exe()?, control)?;
        let fresh = snapshot(workspace, control)?;
        let subject = self.binding(checked.report(), &fresh)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        control
            .with_active(|| {
                let gate = state
                    .gate
                    .as_mut()
                    .ok_or_else(|| io::Error::other("no contract"))?;
                if source != fresh || !gate.refresh(Origin::Controller, subject.clone()) {
                    return Err(io::Error::other("project changed; evidence is not current"));
                }
                let receipt = Receipt {
                    subject: subject.clone(),
                    position: 0,
                    obligation: Self::obligation(),
                    passed: true,
                    active: true,
                };
                if checked.report().checker_sha256 != verification::ledger_checker_id()
                    || !gate.record(Origin::Checker, &subject, &receipt)
                    || !gate.eligible(&subject)
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

fn snapshot(workspace: &Workspace, control: &Control) -> Result<String> {
    let input =
        serde_json::to_vec(&json!({"operation":"snapshot","arguments":{"path":"ledger.rs"}}))?;
    let output = process::run_with_input(
        &mut workspace.file_command()?,
        &input,
        &Limits {
            timeout: Duration::from_secs(30),
            capture_bytes: 400_000,
            output_bytes: 1_000_000,
        },
        control,
    )?;
    anyhow::ensure!(
        !output.truncated
            && matches!(output.termination, Termination::Exited(code) if code.success()),
        "cannot snapshot ledger.rs: {}",
        output.stderr
    );
    let bytes: Vec<u8> = serde_json::from_str(&output.stdout)?;
    anyhow::ensure!(bytes.len() <= 65_536, "ledger source exceeds 64 KiB");
    Ok(String::from_utf8(bytes)?)
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
    for (name, operation, description) in [
        ("propose_ledger", Operation::Propose, "Propose the fixed Rust ledger-v1 contract for host review. This does not accept it. Read the returned meaning/API; ledger.rs contains only the struct and function, without imports or other items. Use finite if/return expressions and u64 arithmetic; saturating_add/sub and checked_add/sub are supported, with Option<u64>, Some/None, match, if-let and let-else. Option is_some/is_none/unwrap/unwrap_or are supported; unwrap must never panic. No imports, attributes, other free calls/methods, macros or loops. The example body is unimplemented and must be replaced."),
        ("verify_ledger", Operation::Verify, "Independently check current ledger.rs against the host-accepted contract using the offline Rust-to-Lean backend. Tests alone do not suffice. Success retains exact code, proof and native library outside the writable workspace; any later source change makes current evidence stale."),
        ("project_status", Operation::Status, "Read host-derived contract, evidence freshness and exact checked artifact status. A model's prose or forged proof/report files never make this status verified.") ] {
        registry.register(Box::new(ProjectTool { project:Arc::clone(project),workspace:Arc::clone(workspace),control:Arc::clone(control),
            sig:ToolSig::new(name,description,json!({"type":"object","properties":{},"additionalProperties":false})),operation }));
    }
    policy.grants.push(Grant::parse("process.spawn")?);
    Ok(())
}

impl Tool for ProjectTool {
    fn signature(&self) -> &ToolSig {
        &self.sig
    }
    fn requirements(&self, _: &Args) -> Vec<Requirement> {
        match self.operation {
            Operation::Propose => Vec::new(),
            Operation::Verify => vec![
                Requirement::new(Capability::ProcessSpawn, None),
                Requirement::new(Capability::FsRead, Some("/workspace/ledger.rs")),
            ],
            Operation::Status => vec![Requirement::new(
                Capability::FsRead,
                Some("/workspace/ledger.rs"),
            )],
        }
    }
    fn call(&self, args: &Args) -> std::result::Result<Value, ToolError> {
        if !args.is_empty() {
            return Err(ToolError::Failed("project operations accept no model-supplied contract, checker, receipt, path or result".into()));
        }
        let result = match self.operation {
            Operation::Propose => self.project.propose(),
            Operation::Verify => self.project.verify(&self.workspace, &self.control),
            Operation::Status => self.project.status(&self.workspace, &self.control),
        };
        result
            .and_then(|s| Ok(serde_json::to_value(s)?))
            .map_err(|e| ToolError::Failed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editing_discards_evidence_before_another_status_read() -> Result<()> {
        let project = Project::new(Path::new("/sessions/editing"));
        let subject = project.subject(b"checked inputs".to_vec());
        let obligation = Project::obligation();
        let mut gate = Gate::propose(subject.clone(), vec![obligation.clone()]);
        assert!(gate.review(Origin::Host));
        assert!(gate.accept(Origin::Host));
        assert!(gate.record(
            Origin::Checker,
            &subject,
            &Receipt {
                subject: subject.clone(),
                position: 0,
                obligation,
                passed: true,
                active: true,
            }
        ));
        assert!(gate.eligible(&subject));
        project
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?
            .gate = Some(gate);

        project.mark_dirty();
        let mut state = project
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("project unavailable"))?;
        assert!(state.dirty);
        let gate = state.gate.as_mut().context("no contract")?;
        assert!(gate.is_accepted());
        // Even an identical source after the write needs fresh evidence.
        assert!(gate.refresh(Origin::Controller, subject.clone()));
        assert!(!gate.eligible(&subject));
        Ok(())
    }

    #[test]
    fn host_acceptance_requires_review_of_the_exact_session_contract() -> Result<()> {
        let mut project = Project::new(Path::new("/sessions/one"));
        project.bundle = Some(PathBuf::from("/prepared/verifier.json"));
        let proposal = project.propose()?;
        assert!(!proposal.accepted && !proposal.verified);
        assert!(project.accept(&proposal.reference).is_err());
        let foreign = Project::new(Path::new("/sessions/two")).current_reference();
        assert!(project.review(&foreign).is_err());
        assert!(!project.accepted());
        let mut changed_contract = proposal.reference.clone();
        changed_contract.contract.push_str("-changed");
        assert!(project.review(&changed_contract).is_err());
        assert!(!project.review(&proposal.reference)?.accepted);
        assert!(project.accept(&foreign).is_err());
        assert!(project.accept(&proposal.reference)?.accepted);
        assert!(project.accepted());
        assert!(project.take_autorun());
        assert!(!project.take_autorun());
        assert!(!project.stored_status()?.verified);
        Ok(())
    }
}
