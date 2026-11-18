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
    pub engine: String,
    pub model: String,
    pub input: Input,
    pub state: State,
    pub assessment: Option<Assessment>,
    pub error: Option<String>,
}
impl Record {
    pub(crate) fn summary(&self) -> String {
        let mut text = format!(
            "Meaning analysis · {:?} (candidate, not proof)\nTarget: {} · {}:{}\nRequirement: {}",
            self.state,
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
        text
    }
    fn save(&self, root: &Path) -> Result<()> {
        crate::storage::write(
            &crate::storage::directory(root)?.join("meaning-analysis.json"),
            self,
        )
    }
}
pub(crate) fn load(root: &Path) -> Result<Option<Record>> {
    let mut record: Option<Record> =
        crate::storage::read(&crate::storage::directory(root)?.join("meaning-analysis.json"))?;
    if let Some(record) = &mut record {
        anyhow::ensure!(
            record.version == 1 && record.engine == semantic::ENGINE,
            "unsupported meaning-analysis checkpoint"
        );
        record.input.validate()?;
        record.state = match record.state {
            State::Complete | State::Stale | State::Historical => State::Historical,
            State::Observing | State::Translating | State::Comparing => {
                record.error = Some("Previous meaning analysis did not finish. Rerun explicitly; no old result is current.".into());
                State::Cancelled
            }
            state => state,
        };
    }
    Ok(record)
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
    let record = RefCell::new(Record {
        version: 1,
        engine: semantic::ENGINE.into(),
        model: gateway.provider().model.clone(),
        input,
        state: State::Observing,
        assessment: None,
        error: None,
    });
    record.borrow().save(workspace.path())?;
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
        if let Err(storage) = record.save(workspace.path()) {
            record.error = Some(format!("{error:#}; checkpoint failed: {storage:#}"));
        }
    }
    notify(&record.borrow());
    result?;
    Ok(record.into_inner())
}
