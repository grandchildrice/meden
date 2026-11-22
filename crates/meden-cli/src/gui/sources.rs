//! GUI presentation for shared policy-bound source observations.
pub(super) use crate::analysis::sources::{capture, Snapshot};
use anyhow::Result;
use meden_runtime::{process::Control, workspace::Workspace};

#[derive(Clone, serde::Serialize)]
pub(super) struct Preparation {
    state: &'static str,
    cache_hit: bool,
    error: Option<String>,
}

pub(super) fn unavailable(view: &std::sync::Mutex<super::View>, path: &str, error: String) {
    let mut view = super::lock(view);
    view.source_preparation.insert(
        path.into(),
        Preparation {
            state: "failed",
            cache_hit: false,
            error: Some(error),
        },
    );
    view.source_snapshot = None;
}

/// Finish compiler, meaning and search preparation before publishing the page.
/// Hover/graph selection never executes a model or a checker.
pub(super) fn prepare(
    view: &std::sync::Mutex<super::View>,
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
    super::lock(view).source_preparation.insert(
        path.into(),
        Preparation {
            state: "preparing",
            cache_hit: false,
            error: None,
        },
    );
    let result = (|| -> Result<(Snapshot, bool)> {
        if !crate::adapters::lake::enabled()? {
            if let Some(snapshot) = crate::explanation::cache::compiled(workspace, control, path)
                .ok()
                .flatten()
            {
                return Ok((snapshot, true));
            }
        }
        crate::analysis::extract_cached(workspace, control, &[path.into()])
    })();
    let mut view = super::lock(view);
    match result {
        Ok((snapshot, cache_hit)) => {
            if publish {
                view.source_snapshot = Some(snapshot);
            }
            view.source_preparation.insert(
                path.into(),
                Preparation {
                    state: "ready",
                    cache_hit,
                    error: None,
                },
            );
        }
        Err(error) => {
            if publish {
                view.source_snapshot = None;
            }
            view.source_preparation.insert(
                path.into(),
                Preparation {
                    state: "failed",
                    cache_hit: false,
                    error: Some(format!("{error:#}")),
                },
            );
        }
    }
}

pub(super) fn refresh(
    view: &std::sync::Mutex<super::View>,
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
) -> Result<()> {
    super::lock(view).source_snapshot = None;
    let snapshot = match capture(workspace, control, paths) {
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
        for path in paths {
            view.source_preparation.insert(
                path.clone(),
                Preparation {
                    state: "preparing",
                    cache_hit: false,
                    error: None,
                },
            );
        }
    }
    let extraction = crate::analysis::extract(workspace, control, paths);
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
            view.source_preparation.insert(
                path.clone(),
                Preparation {
                    state: "ready",
                    cache_hit: false,
                    error: None,
                },
            );
        }
        view.source_snapshot = Some(snapshot);
        view.atlas = atlas.status();
        Ok(())
    })?;
    Ok(())
}
