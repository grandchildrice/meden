//! Model proposals and host-only review of immutable, single-use HTTP exports.

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use meden_agent::tool::{Args, Prepared as ToolPrepared, Tool, ToolError, ToolSig};
use meden_agent::Registry;
use meden_core::file::{Operation as FileOperation, Request as FileRequest};
use meden_core::{Capability, Policy, Requirement};
use meden_runtime::process::{Control, Limits};
use meden_runtime::transfer::Export;
use meden_runtime::workspace::Workspace;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Observed kernel phase; this enum conveys no authority to advance it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// Prepared but not reviewed.
    Pending,
    /// Original request exposed to the host.
    Reviewed,
    /// Host refusal.
    Rejected,
    /// A dispatch was reserved.
    Attempted,
    /// An actual transport response arrived.
    ResponseReceived,
    /// The attempt remains consumed without a known response.
    DeliveryUncertain,
}
impl From<meden_core::export_control::Phase> for State {
    fn from(value: meden_core::export_control::Phase) -> Self {
        use meden_core::export_control::Phase;
        match value {
            Phase::Pending => Self::Pending,
            Phase::Reviewed => Self::Reviewed,
            Phase::Rejected => Self::Rejected,
            Phase::Attempted => Self::Attempted,
            Phase::ResponseReceived => Self::ResponseReceived,
            Phase::DeliveryUncertain => Self::DeliveryUncertain,
        }
    }
}
impl std::fmt::Display for State {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output.write_str(match self {
            Self::Pending => "pending",
            Self::Reviewed => "reviewed",
            Self::Rejected => "rejected",
            Self::Attempted => "attempted",
            Self::ResponseReceived => "response_received",
            Self::DeliveryUncertain => "delivery_uncertain",
        })
    }
}

/// A host input must identify both the session and the immutable entry.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub session: String,
    pub id: usize,
}

#[derive(Clone, Serialize)]
pub struct Summary {
    #[serde(flatten)]
    pub reference: Reference,
    pub source: String,
    pub contract: Option<String>,
    pub target: String,
    pub bytes: usize,
    pub payload_sha256: String,
    pub state: State,
}

pub struct Prepared {
    reference: Reference,
    source: String,
    export: Export,
}

impl Prepared {
    pub fn summary(&self) -> Result<Summary> {
        let snapshot = self.export.snapshot()?;
        Ok(Summary {
            reference: self.reference.clone(),
            source: self.source.clone(),
            contract: snapshot
                .contract
                .map(|v| format!("{:x}", Sha256::digest(&v.subject.contract))),
            target: snapshot.target,
            bytes: snapshot.payload.len(),
            payload_sha256: format!("{:x}", Sha256::digest(&snapshot.payload)),
            state: snapshot.phase.into(),
        })
    }

    /// Exact bytes in hex, independent of terminal escapes or UTF-8 decoding.
    pub fn hex(&self, offset: usize, limit: usize) -> Result<String> {
        let mut text = String::new();
        for byte in self
            .export
            .snapshot()?
            .payload
            .iter()
            .skip(offset)
            .take(limit)
        {
            let _ = write!(text, "{byte:02x}");
        }
        Ok(text)
    }

    pub fn preview(&self, offset: usize) -> Result<String> {
        let mut text = String::new();
        for (line, chunk) in self
            .export
            .snapshot()?
            .payload
            .get(offset..)
            .unwrap_or_default()
            .chunks(16)
            .take(8)
            .enumerate()
        {
            let position = offset.saturating_add(line.saturating_mul(16));
            let _ = write!(text, "{position:08x}  ");
            for byte in chunk {
                let _ = write!(text, "{byte:02x} ");
            }
            text.push_str(" | ");
            for byte in chunk {
                text.push(if byte.is_ascii_graphic() || *byte == b' ' {
                    char::from(*byte)
                } else {
                    '.'
                });
            }
            text.push('\n');
        }
        Ok(text)
    }
}

pub struct Exports {
    session: String,
    project: Arc<crate::project::Project>,
    entries: Mutex<Vec<Arc<Prepared>>>,
}

impl Exports {
    pub fn new(project: Arc<crate::project::Project>) -> Self {
        Self {
            session: project.session_id().to_owned(),
            project,
            entries: Mutex::new(Vec::new()),
        }
    }

    pub fn matches(
        &self,
        workspace: &std::path::Path,
        project: &Arc<crate::project::Project>,
    ) -> bool {
        self.session == project.session_id()
            && self.project.matches(workspace)
            && Arc::ptr_eq(&self.project, project)
    }

    pub fn list(&self) -> Result<Vec<Summary>> {
        self.entries
            .lock()
            .map_err(|_| anyhow::anyhow!("export queue unavailable"))?
            .iter()
            .map(|entry| entry.summary())
            .collect()
    }

    pub fn get(&self, reference: &Reference) -> Result<Arc<Prepared>> {
        anyhow::ensure!(
            reference.session == self.session,
            "export belongs to another session"
        );
        let entries = self
            .entries
            .lock()
            .map_err(|_| anyhow::anyhow!("export queue unavailable"))?;
        entries.get(reference.id).cloned().context("unknown export")
    }

    pub fn reject(&self, reference: &Reference) -> Result<Summary> {
        let entry = self.get(reference)?;
        self.project.reject_export(&entry.export)?;
        entry.summary()
    }

    /// Expose the actual frozen request to a trusted interface before approval.
    pub fn review(&self, reference: &Reference) -> Result<Arc<Prepared>> {
        let entry = self.get(reference)?;
        self.project.review_export(&entry.export)?;
        Ok(entry)
    }

    /// Only call from the host UI/protocol; never register as a model tool.
    pub fn send(
        &self,
        reference: &Reference,
        workspace: &Workspace,
        control: &Control,
    ) -> Result<(Summary, Option<u16>)> {
        let entry = self.get(reference)?;
        let sent = self
            .project
            .send_export(&entry.export, workspace, control)?;
        Ok((entry.summary()?, sent))
    }

    fn propose(&self, source: &str, export: Export) -> Result<Summary> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| anyhow::anyhow!("export queue unavailable"))?;
        anyhow::ensure!(entries.len() < 8, "session export limit reached");
        let prepared = Arc::new(Prepared {
            reference: Reference {
                session: self.session.clone(),
                id: entries.len(),
            },
            source: source.to_owned(),
            export,
        });
        let summary = prepared.summary()?;
        entries.push(prepared);
        Ok(summary)
    }
}

struct Propose {
    queue: Arc<Exports>,
    workspace: Arc<Workspace>,
    control: Arc<Control>,
    sig: ToolSig,
    artifact: bool,
}

pub fn register(
    registry: &mut Registry,
    workspace: &Arc<Workspace>,
    control: &Arc<Control>,
    queue: &Arc<Exports>,
) {
    registry.register(Box::new(Propose {
        workspace: Arc::clone(workspace), control: Arc::clone(control), queue: Arc::clone(queue),
        sig: ToolSig::new("propose_export", "Prepare an immutable workspace file (up to 1 MiB) for host review as an HTTP POST in a session without a project contract. Project contracts require propose_artifact_export instead. This never sends or approves. The user must review the exact destination and bytes and authorize once through the host interface.", json!({"type":"object","properties":{"path":{"type":"string"},"target":{"type":"string"}},"required":["path","target"],"additionalProperties":false})),
        artifact: false,
    }));
    registry.register(Box::new(Propose {
        workspace: Arc::clone(workspace), control: Arc::clone(control), queue: Arc::clone(queue),
        sig: ToolSig::new("propose_artifact_export", "Prepare the current checked project artifact for exact host review as an HTTP POST. Includes the original checked source, backend outputs and report with its explicit evidence kind and scope. Requires accepted meaning and current complete evidence; arbitrary workspace files cannot replace the artifact. Edits, pending replacements and damaged evidence prevent delivery. This never approves or sends.", json!({"type":"object","properties":{"target":{"type":"string"}},"required":["target"],"additionalProperties":false})),
        artifact: true,
    }));
}

impl Tool for Propose {
    fn signature(&self) -> &ToolSig {
        &self.sig
    }
    fn prepare(&self, args: Args) -> Result<ToolPrepared<'_>, ToolError> {
        let target = args
            .get("target")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::failed("missing target".into()))?
            .to_owned();
        if self.artifact {
            if args.len() != 1 {
                return Err(ToolError::failed(
                    "artifact export accepts only target".into(),
                ));
            }
            let access = self
                .queue
                .project
                .prepare_access()
                .map_err(|error| ToolError::failed(error.to_string()))?;
            let mut requirements = access.requirements();
            requirements.push(Requirement::new(Capability::ProcessSpawn, None));
            return Ok(ToolPrepared::new(
                ((target, access), requirements),
                |(target, access), rules| {
                    let result = self
                        .queue
                        .project
                        .prepare_artifact_export(
                            access,
                            &self.workspace,
                            &self.control,
                            &target,
                            rules,
                        )
                        .and_then(|export| {
                            self.queue
                                .propose("checked project artifact package", export)
                        });
                    result
                        .and_then(|summary| Ok(serde_json::to_value(summary)?))
                        .map_err(|error| ToolError::failed(error.to_string()))
                },
            ));
        }
        let path = args
            .get("path")
            .and_then(Value::as_str)
            .and_then(crate::workspace::logical_path)
            .ok_or_else(|| ToolError::failed("path leaves /workspace".into()))?;
        let source = path.to_string_lossy().into_owned();
        let (request, requirements) =
            FileRequest::new(FileOperation::Snapshot, source.as_str().into(), Args::new())
                .prepare();
        Ok(ToolPrepared::new(
            ((request, source, target), requirements),
            |(request, source, target), rules| self.snapshot(&request, &source, &target, rules),
        ))
    }
}

impl Propose {
    fn snapshot(
        &self,
        request: &FileRequest<Args>,
        source: &str,
        target: &str,
        rules: &Policy,
    ) -> Result<Value, ToolError> {
        let value = meden_runtime::file::run(
            &self.workspace,
            &self.control,
            request,
            rules,
            &Limits {
                timeout: Duration::from_secs(30),
                capture_bytes: 4_194_368,
                output_bytes: 4_194_368,
            },
        )?;
        let bytes: Vec<u8> = serde_json::from_value(value)
            .map_err(|error| ToolError::unavailable(error.to_string()))?;
        let result = (|| -> Result<Summary> {
            anyhow::ensure!(bytes.len() <= 1_048_576, "export exceeds 1 MiB");
            let export = self.queue.project.prepare_file_export(target, &bytes)?;
            self.queue.propose(source, export)
        })();
        result
            .and_then(|summary| Ok(serde_json::to_value(summary)?))
            .map_err(|error| ToolError::failed(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{delivery::Delivery, view::sequence::Sequence};

    #[test]
    fn original_phase_labels_are_preserved_in_every_observation() -> Result<()> {
        use meden_core::export_control::Phase;
        for phase in [
            Phase::Pending,
            Phase::Reviewed,
            Phase::Rejected,
            Phase::Attempted,
            Phase::ResponseReceived,
            Phase::DeliveryUncertain,
        ] {
            let observed = State::from(phase);
            assert_eq!(observed.to_string(), phase.label());
            assert_eq!(serde_json::to_value(observed)?, phase.label());
        }
        Ok(())
    }

    #[test]
    fn shared_review_keeps_frozen_bytes_and_consumes_stale_selection() -> Result<()> {
        let project = Arc::new(crate::project::Project::new(std::path::Path::new(
            "/sessions/delivery",
        )));
        let mut delivery = Delivery::new(&project);
        let mut bytes = vec![0, 255, 10];
        let export = project.prepare_file_export("http://127.0.0.1:1234/exact", &bytes)?;
        let summary = delivery.queue.propose("payload.bin", export)?;
        bytes.fill(42);
        assert_eq!(summary.state, State::Pending);
        delivery.review(&summary.reference, 0)?;
        let first = serde_json::to_value(delivery.snapshot()?.review)?;
        assert_eq!(first.pointer("/body_hex"), Some(&json!("00ff0a")));
        assert_eq!(first.pointer("/serial"), Some(&json!("1")));
        assert_eq!(first.pointer("/summary/state"), Some(&json!("reviewed")));
        delivery.review(&summary.reference, 2)?;
        let second = serde_json::to_value(delivery.snapshot()?.review)?;
        assert_eq!(second.pointer("/body_hex"), Some(&json!("0a")));
        assert_eq!(second.pointer("/serial"), Some(&json!("2")));
        assert!(delivery
            .close(&summary.reference, Sequence::new(1)?)
            .is_err());
        assert!(delivery.snapshot()?.review.is_none());
        assert!(delivery
            .close(&summary.reference, Sequence::new(2)?)
            .is_err());
        assert_eq!(
            delivery.queue.get(&summary.reference)?.summary()?.state,
            State::Reviewed
        );
        delivery.review(&summary.reference, 0)?;
        delivery.reject(&summary.reference, Sequence::new(3)?)?;
        let outcome = serde_json::to_value(delivery.snapshot()?.outcome)?;
        assert_eq!(outcome.pointer("/summary/state"), Some(&json!("rejected")));
        assert_eq!(outcome.pointer("/http_status"), Some(&json!(Value::Null)));
        delivery.review(&summary.reference, 0)?;
        assert!(delivery.review(&summary.reference, 3).is_err());
        assert!(delivery.snapshot()?.review.is_none());
        assert!(delivery
            .queue
            .get(&Reference {
                session: "another-session".into(),
                id: 0
            })
            .is_err());
        Ok(())
    }

    #[test]
    fn proposal_limit_matches_the_generated_bounded_index() -> Result<()> {
        let project = Arc::new(crate::project::Project::new(std::path::Path::new(
            "/sessions/bounds",
        )));
        let queue = Exports::new(Arc::clone(&project));
        for id in 0..8 {
            let export = project.prepare_file_export("http://127.0.0.1:1234/exact", &[])?;
            assert_eq!(queue.propose("empty.bin", export)?.reference.id, id);
        }
        let overflow = project.prepare_file_export("http://127.0.0.1:1234/exact", &[])?;
        assert!(queue.propose("empty.bin", overflow).is_err());
        assert_eq!(queue.list()?.len(), 8);
        Ok(())
    }
}
