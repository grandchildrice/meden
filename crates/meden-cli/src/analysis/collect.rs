//! Bounded project-local import closure. Missing/unsupported dependencies stop extraction.
use super::sources::{self, Snapshot};
use anyhow::Result;
use meden_runtime::{process::Control, workspace::Workspace};
use serde::Serialize;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

#[derive(Clone, Serialize)]
pub(crate) struct Collection {
    pub roots: Vec<String>,
    pub external_modules: Vec<String>,
}
pub(super) fn capture(
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
) -> Result<Snapshot> {
    let atlas = crate::atlas::Configuration::host();
    let mut snapshot = sources::capture(workspace, control, paths)?;
    let roots = snapshot.files.keys().cloned().collect();
    let mut external = BTreeSet::new();
    let started = Instant::now();
    loop {
        anyhow::ensure!(
            started.elapsed() < Duration::from_secs(180),
            "Import collection exceeded its 180 second budget"
        );
        let imports = atlas.imports(&snapshot.files, control)?;
        let mut expanded = snapshot.files.keys().cloned().collect::<BTreeSet<_>>();
        for row in imports {
            for module in row.modules {
                if module.external {
                    external.insert(module.name);
                } else {
                    let path = format!("{}.lean", module.name.replace('.', "/"));
                    anyhow::ensure!(
                        !path.starts_with('/')
                            && path.split('/').all(|part| !part.is_empty() && part != "..")
                            && path.len() <= 4096,
                        "Invalid imported module path"
                    );
                    expanded.insert(path);
                }
            }
        }
        if expanded.len() == snapshot.files.len() {
            break;
        }
        anyhow::ensure!(
            expanded.len() <= 32,
            "Local import closure exceeds 32 files; no partial graph admitted"
        );
        snapshot = sources::capture(workspace, control, &expanded.into_iter().collect::<Vec<_>>())
            .map_err(|error|anyhow::anyhow!("Project-local import collection failed: {error:#}. Select the project module root; external packages are not fetched or silently omitted."))?;
    }
    snapshot.collection = Some(Collection {
        roots,
        external_modules: external.into_iter().collect(),
    });
    Ok(snapshot)
}
