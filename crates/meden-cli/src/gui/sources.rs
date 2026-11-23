//! GUI presentation for shared policy-bound source observations.
use crate::analysis::sources::capture;
use anyhow::Result;
pub(super) use meden_app::view::PreparedSource as Snapshot;
use meden_runtime::{process::Control, workspace::Workspace};

mod preparation;
pub(super) use preparation::Preparation;
use std::sync::{Arc, Mutex};

/// A fresh read invalidates every retained graph containing a changed or
/// unavailable input. Other tabs keep their immutable, still-matching pages.
pub(super) fn observe(view: &Mutex<super::View>, observation: &meden_app::view::file::FileView) {
    let mut view = super::lock(view);
    for path in view.sources.observe(observation) {
        view.source_preparation.remove(&path);
    }
    if !observation.is_text() {
        view.source_preparation.remove(&observation.path);
    }
}

pub(super) fn unavailable(view: &Mutex<super::View>, path: &str, error: String) {
    let mut view = super::lock(view);
    view.sources.forget_file(path);
    view.source_preparation
        .entry(path.into())
        .or_insert_with(Preparation::new)
        .finish(Some(error), false);
    view.sources.clear_current();
}

/// Finish compiler, meaning and search preparation before publishing the page.
/// Hover/graph selection never executes a model or a checker.
pub(super) fn prepare(
    view: &Arc<Mutex<super::View>>,
    workspace: &Workspace,
    control: &Control,
    path: &str,
    publish: bool,
) {
    if std::path::Path::new(path)
        .extension()
        .is_none_or(|ext| ext != "lean")
    {
        return;
    }
    // A Lake project can span hundreds of thousands of modules. Its host
    // preparation is explicit; do not start a whole-project build on map startup.
    if !publish && crate::adapters::lake::enabled().unwrap_or(false) {
        return;
    }
    super::lock(view)
        .source_preparation
        .insert(path.into(), Preparation::new());
    super::lock(view).sources.forget_file(path);
    let result = crate::analysis::extract_cached(
        workspace,
        control,
        &[path.into()],
        Some(preparation::observer(view, &[path.into()])),
    )
    .and_then(|(snapshot, cached)| Ok((Snapshot::new(snapshot)?, cached)));
    let mut view = super::lock(view);
    match result {
        Ok((snapshot, cache_hit)) => {
            if publish {
                view.sources.publish(snapshot);
            }
            if let Some(preparation) = view.source_preparation.get_mut(path) {
                preparation.finish(None, cache_hit);
            }
        }
        Err(error) => {
            if publish {
                view.sources.clear_current();
            }
            if let Some(preparation) = view.source_preparation.get_mut(path) {
                preparation.finish(Some(format!("{error:#}")), false);
            }
        }
    }
}

pub(super) fn refresh(
    view: &Mutex<super::View>,
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
) -> Result<()> {
    super::lock(view).sources.clear_current();
    let snapshot = match capture(workspace, control, paths).and_then(Snapshot::new) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            for path in paths {
                unavailable(view, path, format!("{error:#}"));
            }
            return Err(error);
        }
    };
    let mut view = super::lock(view);
    if view.analysis.as_ref().is_some_and(|record| {
        record.input.source_revision != snapshot.revision || record.input.graph_revision.is_some()
    }) {
        super::analysis::invalidate(&mut view);
    }
    view.sources.publish(snapshot);
    Ok(())
}

pub(super) fn extract_atlas(
    view: &Arc<Mutex<super::View>>,
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
) -> Result<()> {
    let atlas = crate::atlas::Configuration::host(workspace.path());
    {
        let mut view = super::lock(view);
        view.sources.clear_current();
        super::analysis::invalidate(&mut view);
        view.activity = "Compiling Lean and extracting Atlas dependencies".into();
        for path in paths {
            view.source_preparation
                .insert(path.clone(), Preparation::new());
        }
    }
    let extraction = crate::analysis::extract_cached(
        workspace,
        control,
        paths,
        Some(preparation::observer(view, paths)),
    )
    .and_then(|(snapshot, _)| Snapshot::new(snapshot));
    super::lock(view).atlas = atlas.status();
    let snapshot = match extraction {
        Ok(snapshot) => snapshot,
        Err(error) => {
            for path in paths {
                unavailable(view, path, format!("{error:#}"));
            }
            return Err(error);
        }
    };
    control.with_active(|| {
        let mut view = super::lock(view);
        for path in snapshot.files.keys() {
            view.source_preparation
                .entry(path.clone())
                .or_insert_with(Preparation::new)
                .finish(None, false);
        }
        view.sources.publish(snapshot);
        view.atlas = atlas.status();
        Ok(())
    })?;
    Ok(())
}
