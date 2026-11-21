//! GUI presentation for shared policy-bound source observations.
pub(super) use crate::analysis::sources::{capture, Snapshot};
use anyhow::Result;
use meden_runtime::{process::Control, workspace::Workspace};

pub(super) fn refresh(
    view: &std::sync::Mutex<super::View>,
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
) -> Result<()> {
    super::lock(view).source_snapshot = None;
    let snapshot = capture(workspace, control, paths)?;
    let mut view = super::lock(view);
    if view.analysis.as_ref().is_some_and(|record| {
        record.input.source_revision != snapshot.revision || record.input.graph_revision.is_some()
    }) {
        super::analysis::invalidate(&mut view);
    }
    view.source_snapshot = Some(snapshot);
    Ok(())
}

pub(super) fn extract_atlas(
    view: &std::sync::Mutex<super::View>,
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
) -> Result<()> {
    let atlas = crate::atlas::Configuration::host(workspace.path());
    {
        let mut view = super::lock(view);
        view.source_snapshot = None;
        super::analysis::invalidate(&mut view);
        view.activity = "Compiling Lean and extracting Atlas dependencies".into();
    }
    let extraction = crate::analysis::extract(workspace, control, paths);
    super::lock(view).atlas = atlas.status();
    let snapshot = extraction?;
    control.with_active(|| {
        let mut view = super::lock(view);
        view.source_snapshot = Some(snapshot);
        view.atlas = atlas.status();
        Ok(())
    })?;
    Ok(())
}
