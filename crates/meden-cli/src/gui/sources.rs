//! GUI presentation for shared policy-bound source observations.
use crate::analysis::sources::capture;
use anyhow::Result;
pub(super) use meden_app::view::PreparedSource as Snapshot;
use meden_runtime::{process::Control, workspace::Workspace};

mod preparation;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

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

pub(super) fn unavailable(view: &Mutex<super::View>, path: &str, error: String) -> Result<()> {
    let mut view = super::lock(view);
    view.sources.forget_file(path);
    let original = view.source_preparation.begin(path, "Reading source")?;
    view.source_preparation
        .finish(&original, 0, Err(error.into()));
    view.sources.clear_current();
    Ok(())
}

/// Finish compiler, meaning and search preparation before publishing the page.
/// Hover/graph selection never executes a model or a checker.
pub(super) fn prepare(
    view: &Arc<Mutex<super::View>>,
    workspace: &Workspace,
    control: &Control,
    path: &str,
    publish: bool,
) -> Result<()> {
    if std::path::Path::new(path)
        .extension()
        .is_none_or(|ext| ext != "lean")
    {
        return Ok(());
    }
    // A Lake project can span hundreds of thousands of modules. Its host
    // preparation is explicit; do not start a whole-project build on map startup.
    if !publish && crate::adapters::lake::enabled().unwrap_or(false) {
        return Ok(());
    }
    let started = Instant::now();
    let original = super::lock(view)
        .source_preparation
        .begin(path, "Resolving modules")?;
    super::lock(view).sources.forget_file(path);
    let result = crate::analysis::extract_cached(
        workspace,
        control,
        &[path.into()],
        Some(preparation::observer(
            view,
            std::slice::from_ref(&original),
            started,
        )),
    )
    .and_then(|(snapshot, cached)| Ok((Snapshot::new(snapshot)?, cached)));
    match result {
        Ok((snapshot, cache_hit)) => {
            let completion = control.with_active(|| {
                let mut view = super::lock(view);
                if publish {
                    view.sources.publish(snapshot);
                }
                view.source_preparation.finish(
                    &original,
                    preparation::millis(started),
                    Ok(cache_hit),
                );
                Ok(())
            });
            if let Err(error) = completion {
                let mut view = super::lock(view);
                if publish {
                    view.sources.clear_current();
                }
                view.source_preparation.finish(
                    &original,
                    preparation::millis(started),
                    Err(preparation::failure(control, error.to_string())),
                );
            }
        }
        Err(error) => {
            let mut view = super::lock(view);
            if publish {
                view.sources.clear_current();
            }
            view.source_preparation.finish(
                &original,
                preparation::millis(started),
                Err(preparation::failure(control, format!("{error:#}"))),
            );
        }
    }
    Ok(())
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
                unavailable(view, path, format!("{error:#}"))?;
            }
            return Err(error);
        }
    };
    let mut view = super::lock(view);
    if view.analyses.data().current.as_ref().is_some_and(|record| {
        record.input.source_revision != snapshot.revision || record.input.graph_revision.is_some()
    }) {
        view.analyses.invalidate();
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
    let started = Instant::now();
    let originals = {
        let mut view = super::lock(view);
        view.sources.clear_current();
        view.analyses.invalidate();
        view.activity = "Compiling Lean and extracting Atlas dependencies".into();
        paths
            .iter()
            .map(|path| view.source_preparation.begin(path, "Resolving modules"))
            .collect::<Result<Vec<_>>>()?
    };
    let extraction = crate::analysis::extract_cached(
        workspace,
        control,
        paths,
        Some(preparation::observer(view, &originals, started)),
    )
    .and_then(|(snapshot, _)| Snapshot::new(snapshot));
    super::lock(view).atlas = atlas.status();
    let snapshot = match extraction {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let mut view = super::lock(view);
            for path in paths {
                view.sources.forget_file(path);
            }
            for original in &originals {
                view.source_preparation.finish(
                    original,
                    preparation::millis(started),
                    Err(preparation::failure(control, format!("{error:#}"))),
                );
            }
            return Err(error);
        }
    };
    let completion = control.with_active(|| {
        let mut view = super::lock(view);
        for original in &originals {
            view.source_preparation
                .finish(original, preparation::millis(started), Ok(false));
        }
        view.sources.publish(snapshot);
        view.atlas = atlas.status();
        Ok(())
    });
    if let Err(error) = completion {
        let mut view = super::lock(view);
        for path in paths {
            view.sources.forget_file(path);
        }
        for original in &originals {
            view.source_preparation.finish(
                original,
                preparation::millis(started),
                Err(preparation::failure(control, error.to_string())),
            );
        }
        return Err(error.into());
    }
    Ok(())
}
