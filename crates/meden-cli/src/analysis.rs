//! Shared host application service. Candidate assessments never grant proof authority.
use std::cell::RefCell;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::atomic::Ordering;

use crate::semantic::{self, Assessment, Input, Prepared, Stage};
use anyhow::{Context, Result};
use meden_llm::Gateway;
use meden_runtime::{process::Control, workspace::Workspace};
use serde::{Deserialize, Serialize};

pub(crate) mod collect;
pub(crate) mod journal;
pub(crate) mod layout;
pub(crate) mod observe;
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
    pub judge: Option<crate::judge::Binding>,
    pub input: Input,
    pub state: State,
    pub assessment: Option<Assessment>,
    pub partial_translation: Option<semantic::Translation>,
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
pub(crate) fn load(root: &Path) -> Result<Option<Record>> {
    Ok(journal::list(root)?.pop())
}

pub(crate) fn extract(
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
) -> Result<sources::Snapshot> {
    // Preparation is local and idempotent; the public extraction journey needs no setup step.
    crate::atlas::Configuration::host(workspace.path()).ensure_prepared(control)?;
    let mut snapshot = collect::capture(workspace, control, paths)?;
    let observed = snapshot.files.keys().cloned().collect::<Vec<_>>();
    let collection = snapshot
        .collection
        .as_ref()
        .context("Missing collected module bindings")?;
    collect::check_metadata(workspace, control, &collection.metadata)?;
    let mut graph = crate::atlas::Configuration::host(workspace.path()).extract(
        &snapshot.files,
        &collection.modules,
        control,
    )?;
    graph.revision = format!(
        "sha256:{}",
        crate::atlas::digest(&serde_json::to_vec(&(&graph.revision, collection))?)
    );
    collect::check_metadata(workspace, control, &collection.metadata)?;
    anyhow::ensure!(
        sources::capture(workspace, control, &observed)?.revision == snapshot.revision,
        "Sources changed during Atlas extraction; no graph admitted"
    );
    snapshot.atlas = Some(graph);
    control.with_active(|| Ok(()))?;
    crate::explanation::cache::retain(workspace, &snapshot)?;
    Ok(snapshot)
}

pub(crate) fn run(
    workspace: &Workspace,
    gateway: &Gateway,
    judge: Option<&Gateway>,
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
        judge.map(crate::judge::binding),
    )?);
    notify(&record.borrow());

    let result = (|| -> Result<()> {
        anyhow::ensure!(
            gateway.is_authorized(workspace.session_id())
                && crate::judge::authorized(judge, workspace.session_id()),
            "Review both configured processors before meaning analysis"
        );
        observe(workspace, control, snapshot, &record)?;
        let assessment = semantic::analyze(
            &gateway.for_session(workspace.session_id()),
            &judge.unwrap_or(gateway).for_session(workspace.session_id()),
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
                && crate::judge::authorized(judge, workspace.session_id()),
            "Processor permission ended; no meaning-analysis result admitted"
        );
        observe(workspace, control, snapshot, &record)?;
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

fn observe(
    workspace: &Workspace,
    control: &Control,
    snapshot: &sources::Snapshot,
    record: &RefCell<Record>,
) -> Result<()> {
    let paths = snapshot.files.keys().cloned().collect::<Vec<_>>();
    let current = sources::capture(workspace, control, &paths)?;
    if current.revision != snapshot.revision {
        record.borrow_mut().state = State::Stale;
        anyhow::bail!("Sources changed during meaning analysis. Reread the graph and run again; no current result admitted.");
    }
    if let Some(collection) = &snapshot.collection {
        if let Err(error) = collect::check_metadata(workspace, control, &collection.metadata) {
            record.borrow_mut().state = State::Stale;
            return Err(error);
        }
    }
    if let Some(atlas) = &snapshot.atlas {
        anyhow::ensure!(
            crate::atlas::Configuration::host(workspace.path()).current_resources()?
                == atlas.resources_sha256,
            "Atlas resources changed; reread the graph and run again"
        );
    }
    Ok(())
}
