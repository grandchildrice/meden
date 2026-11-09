//! Model proposals and host-only review of immutable, single-use HTTP exports.

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use meden_agent::tool::{Args, Prepared as ToolPrepared, Tool, ToolError, ToolSig};
use meden_agent::Registry;
use meden_core::file::{Operation as FileOperation, Request as FileRequest};
use meden_core::{Capability, Policy, Requirement};
use meden_runtime::process::{self, Control, Limits, Termination};
use meden_runtime::transfer::Export;
use meden_runtime::workspace::Workspace;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

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
    pub state: &'static str,
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
            state: snapshot.phase.label(),
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
    pub fn new(workspace: &std::path::Path, project: Arc<crate::project::Project>) -> Self {
        Self {
            session: session_name(workspace),
            project,
            entries: Mutex::new(Vec::new()),
        }
    }

    pub fn matches(
        &self,
        workspace: &std::path::Path,
        project: &Arc<crate::project::Project>,
    ) -> bool {
        self.session == session_name(workspace)
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

fn session_name(workspace: &std::path::Path) -> String {
    let mut name = String::new();
    for byte in workspace.as_os_str().as_encoded_bytes() {
        let _ = write!(name, "{byte:02x}");
    }
    name
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
        sig: ToolSig::new("propose_artifact_export", "Prepare the current verified project kernel package for exact host review as an HTTP POST. Includes the checked native library, source, proof inputs and report. Requires accepted meaning and current complete evidence; arbitrary workspace files cannot replace the artifact. Edits, pending replacements and damaged evidence prevent delivery. This never approves or sends.", json!({"type":"object","properties":{"target":{"type":"string"}},"required":["target"],"additionalProperties":false})),
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
            .ok_or_else(|| ToolError::Failed("missing target".into()))?
            .to_owned();
        if self.artifact {
            if args.len() != 1 {
                return Err(ToolError::Failed(
                    "artifact export accepts only target".into(),
                ));
            }
            let access = self
                .queue
                .project
                .prepare_access()
                .map_err(|error| ToolError::Failed(error.to_string()))?;
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
                                .propose("verified project kernel package", export)
                        });
                    result
                        .and_then(|summary| Ok(serde_json::to_value(summary)?))
                        .map_err(|error| ToolError::Failed(error.to_string()))
                },
            ));
        }
        let path = args
            .get("path")
            .and_then(Value::as_str)
            .and_then(crate::workspace::logical_path)
            .ok_or_else(|| ToolError::Failed("path leaves /workspace".into()))?;
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
        let result = (|| -> Result<Summary> {
            let input = serde_json::to_vec(&(request, rules))?;
            let output = process::run_with_input(
                &mut self.workspace.file_command()?,
                &input,
                &Limits {
                    timeout: Duration::from_secs(30),
                    capture_bytes: 4_194_306,
                    output_bytes: 4_194_306,
                },
                &self.control,
            )?;
            anyhow::ensure!(
                matches!(output.termination, Termination::Exited(code) if code.success())
                    && !output.truncated,
                "snapshot worker failed: {}",
                output.stderr
            );
            let bytes: Vec<u8> = serde_json::from_str(&output.stdout)?;
            anyhow::ensure!(bytes.len() <= 1_048_576, "export exceeds 1 MiB");
            let export = self.queue.project.prepare_file_export(target, &bytes)?;
            self.queue.propose(source, export)
        })();
        result
            .and_then(|summary| Ok(serde_json::to_value(summary)?))
            .map_err(|error| ToolError::Failed(error.to_string()))
    }
}
