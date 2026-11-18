//! Shared host application service. Candidate assessments never grant proof authority.
use std::cell::RefCell;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::atomic::Ordering;

use crate::semantic::{self, Assessment, Input, Prepared, Stage};
use anyhow::Result;
use meden_llm::Gateway;
use meden_runtime::{process::Control, workspace::Workspace};
use serde::{Deserialize, Serialize};

pub(crate) mod journal;
pub(crate) mod repair;
pub(crate) mod sources;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum State {
    Observing,
    Translating,
    Comparing,
    Complete,
    Failed,
    Cancelled,
    Stale,
    Historical,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub version: u8,
    pub sequence: u64,
    pub id: String,
    pub requirement_id: String,
    pub finding_id: Option<String>,
    pub source_artifact: Option<String>,
    pub engine: String,
    pub model: String,
    pub processor: String,
    pub input: Input,
    pub state: State,
    pub assessment: Option<Assessment>,
    pub error: Option<String>,
}
impl Record {
    pub(crate) fn summary(&self) -> String {
        let mut text = format!(
            "Meaning analysis · {:?} (candidate, not proof)\nRun: {}\nRequirement ID: {}\nFinding ID: {}\nTarget: {} · {}:{}\nRequirement: {}",
            self.state,
            self.id, self.requirement_id, self.finding_id.as_deref().unwrap_or("none"),
            self.input.target.name,
            self.input.target.file,
            self.input.target.line,
            self.input.requirement
        );
        if let Some(assessment) = &self.assessment {
            let _ = write!(
                text,
                "\nCandidate {:?}\nDeclaration means: {}\nDiscrepancy: {}\nExplanation: {}",
                assessment.verdict,
                assessment.translation.natural_language,
                assessment.comparison.discrepancy,
                assessment.comparison.explanation
            );
        }
        if let Some(error) = &self.error {
            let _ = write!(text, "\nStopped: {error}");
        }
        // Long model explanations must not scroll the host outcome out of view.
        let _ = write!(text, "\nResult: {:?} · {}", self.state, self.id);
        if let Some(assessment) = &self.assessment {
            let _ = write!(
                text,
                "\nCandidate {:?} · candidate assessment, not proof",
                assessment.verdict
            );
        }
        text
    }
    fn save(&self, root: &Path) -> Result<()> {
        journal::save(root, self)
    }
}
pub(crate) fn load(root: &Path) -> Result<Option<Record>> {
    Ok(journal::list(root)?.pop())
}

pub(crate) fn extract(
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
) -> Result<sources::Snapshot> {
    let mut snapshot = sources::capture(workspace, control, paths)?;
    let graph = crate::atlas::Configuration::host().extract(&snapshot.files, control)?;
    anyhow::ensure!(
        sources::capture(workspace, control, paths)?.revision == snapshot.revision,
        "Sources changed during Atlas extraction; no graph admitted"
    );
    snapshot.atlas = Some(graph);
    control.with_active(|| Ok(()))?;
    Ok(snapshot)
}

pub(crate) fn run(
    workspace: &Workspace,
    gateway: &Gateway,
    control: &Control,
    snapshot: &sources::Snapshot,
    input: Input,
    notify: &dyn Fn(&Record),
) -> Result<Record> {
    let prepared = Prepared::new(
        input.clone(),
        snapshot.files.clone(),
        snapshot.atlas.as_ref(),
    )?;
    let source_artifact = journal::retain_sources(workspace.path(), snapshot)?;
    let record = RefCell::new(journal::begin(
        workspace.path(),
        gateway.endpoint(),
        &gateway.provider().model,
        input,
        source_artifact,
    )?);
    notify(&record.borrow());
    let observe = || -> Result<()> {
        let paths = snapshot.files.keys().cloned().collect::<Vec<_>>();
        let current = sources::capture(workspace, control, &paths)?;
        if current.revision != snapshot.revision {
            record.borrow_mut().state = State::Stale;
            anyhow::bail!("Sources changed during meaning analysis. Reread the graph and run again; no current result admitted.");
        }
        if let Some(atlas) = &snapshot.atlas {
            anyhow::ensure!(
                crate::atlas::Configuration::host().current_resources()? == atlas.resources_sha256,
                "Atlas resources changed; reread the graph and run again"
            );
        }
        Ok(())
    };
    let result = (|| -> Result<()> {
        observe()?;
        let assessment = semantic::analyze(
            &gateway.for_session(workspace.session_id()),
            &prepared,
            control.flag(),
            &|stage| {
                record.borrow_mut().state = match stage {
                    Stage::Translating => State::Translating,
                    Stage::Comparing => State::Comparing,
                };
                notify(&record.borrow());
            },
        )?;
        anyhow::ensure!(
            gateway.is_authorized(workspace.session_id()),
            "Processor permission ended; no meaning-analysis result admitted"
        );
        observe()?;
        control.with_active(|| {
            // Frozen input is owned by this invocation; presentation cannot replace it.
            let mut completed = record.borrow().clone();
            completed.state = State::Complete;
            completed.assessment = Some(assessment);
            if completed
                .assessment
                .as_ref()
                .is_some_and(|a| a.verdict != semantic::Verdict::Match)
            {
                completed.finding_id = Some(completed.id.replacen("analysis-", "finding-", 1));
            }
            completed
                .save(workspace.path())
                .map_err(std::io::Error::other)?;
            *record.borrow_mut() = completed;
            Ok(())
        })?;
        Ok(())
    })();
    if let Err(error) = &result {
        let mut record = record.borrow_mut();
        if record.state != State::Stale {
            record.state = if control.flag().load(Ordering::SeqCst) {
                State::Cancelled
            } else {
                State::Failed
            };
        }
        record.error = Some(format!("{error:#}"));
        record.assessment = None;
        record.finding_id = None;
        if let Err(storage) = record.save(workspace.path()) {
            record.error = Some(format!("{error:#}; checkpoint failed: {storage:#}"));
        }
    }
    notify(&record.borrow());
    result?;
    Ok(record.into_inner())
}
