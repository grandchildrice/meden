//! Host adapter for candidate analysis. It never records verification evidence.
use std::path::Path;
use std::sync::atomic::Ordering;

use anyhow::{Context, Result};
use meden_llm::Gateway;
use meden_runtime::{process::Control, workspace::Workspace};
use serde::{Deserialize, Serialize};

use super::{lock, sources, View};
use crate::semantic::{self, Assessment, Input, Prepared, Stage};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum State {
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
pub(super) struct Record {
    pub version: u8,
    pub engine: String,
    pub model: String,
    pub input: Input,
    pub state: State,
    pub assessment: Option<Assessment>,
    pub error: Option<String>,
}
impl Record {
    fn save(&self, root: &Path) -> Result<()> {
        crate::storage::write(
            &crate::storage::directory(root)?.join("meaning-analysis.json"),
            self,
        )
    }
}
pub(super) fn load(root: &Path) -> Result<Option<Record>> {
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
pub(super) fn invalidate(view: &mut View) {
    if let Some(record) = &mut view.analysis {
        if matches!(record.state, State::Complete) {
            record.state = State::Stale;
        }
    }
}
pub(super) fn run(
    view: &std::sync::Mutex<View>,
    workspace: &Workspace,
    gateway: &Gateway,
    control: &Control,
    input: Input,
) -> Result<()> {
    let snapshot = lock(view)
        .source_snapshot
        .clone()
        .context("Read the source graph before meaning analysis")?;
    let prepared = Prepared::new(
        input.clone(),
        snapshot.files.clone(),
        snapshot.atlas.as_ref(),
    )?;
    let record = Record {
        version: 1,
        engine: semantic::ENGINE.into(),
        model: gateway.provider().model.clone(),
        input,
        state: State::Observing,
        assessment: None,
        error: None,
    };
    record.save(workspace.path())?;
    lock(view).analysis = Some(record);
    let result = execute(view, workspace, gateway, control, &snapshot, &prepared);
    if let Err(error) = &result {
        let mut view = lock(view);
        if let Some(record) = &mut view.analysis {
            if record.state != State::Stale {
                record.state = if control.flag().load(Ordering::SeqCst) {
                    State::Cancelled
                } else {
                    State::Failed
                };
            }
            record.error = Some(format!("{error:#}"));
            record.assessment = None;
            // A failure to persist is still displayed; no successful result is revived.
            if let Err(storage) = record.save(workspace.path()) {
                record.error = Some(format!("{error:#}; checkpoint failed: {storage:#}"));
            }
        }
    }
    result
}
fn observe_current(
    view: &std::sync::Mutex<View>,
    workspace: &Workspace,
    control: &Control,
    snapshot: &sources::Snapshot,
) -> Result<()> {
    let paths = snapshot.files.keys().cloned().collect::<Vec<_>>();
    let current = sources::capture(workspace, control, &paths)?;
    if let Some(atlas) = &snapshot.atlas {
        anyhow::ensure!(
            crate::atlas::Configuration::host().current_resources()? == atlas.resources_sha256,
            "Atlas resources changed; reread the graph and run again"
        );
    }
    if current.revision != snapshot.revision {
        if let Some(record) = &mut lock(view).analysis {
            record.state = State::Stale;
        }
        anyhow::bail!("Sources changed during meaning analysis. Reread the graph and run again; no current result admitted.");
    }
    Ok(())
}
fn execute(
    view: &std::sync::Mutex<View>,
    workspace: &Workspace,
    gateway: &Gateway,
    control: &Control,
    snapshot: &sources::Snapshot,
    prepared: &Prepared,
) -> Result<()> {
    observe_current(view, workspace, control, snapshot)?;
    let assessment = semantic::analyze(
        &gateway.for_session(workspace.session_id()),
        prepared,
        control.flag(),
        &|stage| {
            let mut view = lock(view);
            view.activity = match stage {
                Stage::Translating => "意味解析：宣言の自然言語化",
                Stage::Comparing => "意味解析：要件との比較",
            }
            .into();
            if let Some(record) = &mut view.analysis {
                record.state = match stage {
                    Stage::Translating => State::Translating,
                    Stage::Comparing => State::Comparing,
                };
            }
        },
    )?;
    anyhow::ensure!(
        gateway.is_authorized(workspace.session_id()),
        "Processor permission ended; no meaning-analysis result admitted"
    );
    observe_current(view, workspace, control, snapshot)?;
    control.with_active(|| {
        let mut view = lock(view);
        let record = view
            .analysis
            .as_mut()
            .ok_or_else(|| std::io::Error::other("analysis record unavailable"))?;
        // Native serialization/work scheduling are adapters, not refined proof admission.
        if record.input != *prepared.input() {
            return Err(std::io::Error::other("analysis input changed"));
        }
        let mut completed = record.clone();
        completed.state = State::Complete;
        completed.assessment = Some(assessment);
        completed
            .save(workspace.path())
            .map_err(std::io::Error::other)?;
        *record = completed;
        view.activity = "意味解析が完了（モデルによる候補評価）".into();
        Ok(())
    })?;
    Ok(())
}
