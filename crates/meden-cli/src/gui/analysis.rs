//! GUI presentation for the shared candidate analysis service.
use super::{lock, View};
pub(super) use crate::analysis::{Record, State};
use anyhow::{Context, Result};

pub(super) fn invalidate(view: &mut View) {
    if let Some(record) = &mut view.analysis {
        if record.state == State::Complete {
            record.state = State::Stale;
        }
    }
}
pub(super) fn run(
    view: &std::sync::Mutex<View>,
    session: &crate::session::Session,
    input: crate::semantic::Input,
) -> Result<()> {
    let snapshot = {
        let view = lock(view);
        view.sources
            .file(&input.target.file)
            .filter(|snapshot| {
                snapshot.revision == input.source_revision
                    && snapshot.atlas.as_ref().map(|a| &a.revision) == input.graph_revision.as_ref()
            })
            .map(|snapshot| snapshot.snapshot().clone())
            .or_else(|| view.sources.current().map(|s| s.snapshot().clone()))
            .context("Read the source graph before meaning analysis")?
    };
    invalidate(&mut lock(view));
    let result = session.analyze(&snapshot, input, &|record| {
        let mut view = lock(view);
        view.activity = match record.state {
            State::Translating => "Meaning analysis: translating declaration",
            State::Comparing => "Meaning analysis: comparing requirements",
            State::Complete => "Meaning analysis complete (candidate model assessment)",
            _ => "Meaning analysis",
        }
        .into();
        view.analysis = Some(record.clone());
    });
    lock(view).analysis_history = session.analysis_history()?;
    result?;
    Ok(())
}

pub(super) fn apply_repair(
    view: &std::sync::Mutex<View>,
    session: &crate::session::Session,
    queue: &mut crate::analysis::repair::Queue,
    reference: &crate::analysis::repair::Reference,
) -> Result<()> {
    {
        let mut view = lock(view);
        anyhow::ensure!(
            view.repair_review
                .as_ref()
                .is_some_and(|review| &review.reference == reference),
            "Repair review selection changed"
        );
        view.repair_review = None;
        view.sources.clear();
        view.project.verified = false;
        invalidate(&mut view);
    }
    let result = session.apply_repair(queue, reference, &|record| {
        lock(view).analysis = Some(record.clone());
    });
    lock(view).analysis_history = session.analysis_history()?;
    let (snapshot, record) = result?;
    let snapshot = meden_app::view::PreparedSource::new(snapshot)?;
    let mut view = lock(view);
    view.sources.publish(snapshot);
    view.analysis = Some(record);
    view.activity =
        "Patch applied and original requirement reanalyzed (candidate model assessment)".into();
    Ok(())
}
