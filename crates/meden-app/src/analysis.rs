//! Shared host application service. Candidate assessments never grant proof authority.
use std::cell::RefCell;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::atomic::Ordering;

use crate::model::Model;
use crate::semantic::{self, Assessment, Input, Prepared, Stage};
use anyhow::Result;
use meden_runtime::{process::Control, workspace::Workspace};
use serde::{Deserialize, Serialize};

pub mod collect;
pub mod journal;
pub mod layout;
pub mod observe;
pub mod prepared;
pub mod repair;
pub mod sources;

/// Backend freshness observations, never proof evidence or execution authority.
pub enum Freshness {
    Current,
    Stale(anyhow::Error),
}
/// The backend checks its own dependency/resource bindings. Original source
/// bytes and the complete retained body are checked independently by this use case.
pub trait Observation: semantic::Observation + Serialize {
    fn check(&self, context: &crate::adapters::Context<'_>) -> Result<Freshness>;
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum State {
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
pub struct Record {
    pub version: u8,
    pub sequence: u64,
    pub id: String,
    pub requirement_id: String,
    pub finding_id: Option<String>,
    pub source_artifact: Option<String>,
    pub engine: String,
    pub model: String,
    pub processor: String,
    pub judge: Option<crate::model::Binding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_definition: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judge_definition: Option<serde_json::Value>,
    pub input: Input,
    pub state: State,
    pub assessment: Option<Assessment>,
    pub partial_translation: Option<semantic::Translation>,
    pub error: Option<String>,
}
impl Record {
    pub fn summary(&self) -> String {
        let mut text = format!(
            "Meaning analysis · {:?} (candidate, not proof)\nRun: {}\nRequirement ID: {}\nFinding ID: {}\nTarget: {} · {}:{}\nRequirement: {}",
            self.state,
            self.id, self.requirement_id, self.finding_id.as_deref().unwrap_or("none"),
            self.input.target.name,
            self.input.target.file,
            self.input.target.line,
            self.input.requirement
        );
        let _ = write!(
            text,
            "\nTranslator: {} · {}\nJudge: {} · {}",
            self.model,
            self.processor,
            self.judge
                .as_ref()
                .map_or(self.model.as_str(), |j| j.model.as_str()),
            self.judge
                .as_ref()
                .map_or(self.processor.as_str(), |j| j.endpoint.as_str())
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
        if self.assessment.is_none() {
            if let Some(translation) = &self.partial_translation {
                let _ = write!(
                    text,
                    "\nBlind translation retained; comparison incomplete: {}",
                    translation.natural_language
                );
            }
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
pub fn load(root: &Path) -> Result<Option<Record>> {
    Ok(journal::list(root)?.pop())
}

pub fn run<O: Observation>(
    workspace: &Workspace,
    gateway: &dyn Model,
    judge: Option<&dyn Model>,
    control: &Control,
    snapshot: &O,
    input: Input,
    notify: &dyn Fn(&Record),
) -> Result<Record> {
    let original = serde_json::to_value(snapshot)?;
    let prepared = Prepared::new(input.clone(), snapshot)?;
    let source_artifact = journal::retain_sources(workspace.path(), &original)?;
    let translator = crate::model::bind(gateway, workspace.session_id());
    let comparer = crate::model::bind(judge.unwrap_or(gateway), workspace.session_id());
    let record = RefCell::new(journal::begin(
        workspace.path(),
        translator.binding(),
        input,
        source_artifact,
        judge.map(|_| comparer.binding()),
    )?);
    notify(&record.borrow());

    let result = (|| -> Result<()> {
        anyhow::ensure!(
            gateway.is_authorized(workspace.session_id())
                && crate::model::authorized(judge, workspace.session_id()),
            "Review both configured processors before meaning analysis"
        );
        observe(workspace, control, snapshot, &original, &record)?;
        let assessment = semantic::analyze(
            &translator,
            &comparer,
            &prepared,
            control.flag(),
            &|stage| {
                let mut current = record.borrow_mut();
                current.state = match stage {
                    Stage::Translating => State::Translating,
                    Stage::Comparing(translation) => {
                        current.partial_translation = Some(translation);
                        State::Comparing
                    }
                };
                drop(current);
                notify(&record.borrow());
            },
        )?;
        anyhow::ensure!(
            gateway.is_authorized(workspace.session_id())
                && crate::model::authorized(judge, workspace.session_id()),
            "Processor permission ended; no meaning-analysis result admitted"
        );
        observe(workspace, control, snapshot, &original, &record)?;
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

fn observe<O: Observation>(
    workspace: &Workspace,
    control: &Control,
    snapshot: &O,
    original: &serde_json::Value,
    record: &RefCell<Record>,
) -> Result<()> {
    let _timing = meden_runtime::metrics::Span::start("analysis.observe");
    if &serde_json::to_value(snapshot)? != original {
        record.borrow_mut().state = State::Stale;
        anyhow::bail!("Observed analysis context changed; reread and run again");
    }
    let paths = snapshot.sources().files.keys().cloned().collect::<Vec<_>>();
    let current = sources::capture(workspace, control, &paths)?;
    if current.revision != snapshot.sources().revision {
        record.borrow_mut().state = State::Stale;
        anyhow::bail!("Sources changed during meaning analysis. Reread the graph and run again; no current result admitted.");
    }
    match snapshot.check(&crate::adapters::Context {
        workspace,
        control,
        observer: None,
    })? {
        Freshness::Current => Ok(()),
        Freshness::Stale(error) => {
            record.borrow_mut().state = State::Stale;
            Err(error)
        }
    }
}
