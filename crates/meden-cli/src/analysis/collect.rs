//! Bounded project import closure with frozen Lake configuration and path bindings.
use super::{
    layout::{Layout, Module, PackageView},
    observe::{relative, Metadata, Reader},
    sources::{self, Snapshot},
};
use anyhow::Result;
use meden_runtime::{process::Control, workspace::Workspace};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::{Duration, Instant},
};

#[derive(Clone, Serialize)]
pub(crate) struct Collection {
    pub roots: Vec<String>,
    pub external_modules: Vec<String>,
    pub modules: Vec<Module>,
    pub packages: Vec<PackageView>,
    pub metadata: Metadata,
}
pub(super) fn capture(
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
) -> Result<Snapshot> {
    let paths = paths
        .iter()
        .map(|path| relative("", path))
        .collect::<Result<Vec<_>>>()?;
    let atlas = crate::atlas::Configuration::host(workspace.path());
    let mut reader = Reader::new(workspace, control);
    let selected = if paths.is_empty() {
        BTreeMap::new()
    } else {
        sources::capture(workspace, control, &paths)?.files
    };
    let layout = Layout::discover(
        &mut reader,
        &atlas,
        control,
        &selected.keys().cloned().collect::<Vec<_>>(),
    )?;
    let mut files = selected;
    let mut modules = BTreeMap::new();
    if files.is_empty() {
        files = layout.automatic_sources(&mut reader)?;
        for path in files.keys() {
            insert(&mut modules, &layout.bind_selected(path)?)?;
        }
        anyhow::ensure!(!files.is_empty(), "Select a Lean source file, or use /atlas in a Lake project with library/executable roots");
    } else {
        for path in files.keys() {
            insert(&mut modules, &layout.bind_selected(path)?)?;
        }
    }
    let roots = files.keys().cloned().collect();
    let mut external = BTreeSet::new();
    let started = Instant::now();
    loop {
        anyhow::ensure!(
            started.elapsed() < Duration::from_secs(180),
            "Import collection exceeded its 180 second budget"
        );
        let imports = atlas.imports(&files, control)?;
        let previous = files.len();
        for row in imports {
            for module in row.modules {
                if module.external {
                    external.insert(module.name);
                } else if !modules.contains_key(&module.name) {
                    let (module, source) = layout.resolve(&module.name, &mut reader)?;
                    files.insert(module.file.clone(), source);
                    insert(&mut modules, &module)?;
                    anyhow::ensure!(
                        files.len() <= 32
                            && files.values().map(String::len).sum::<usize>() <= 1_048_576,
                        "Local import closure exceeds 32 files/1 MiB; no partial graph admitted"
                    );
                }
            }
        }
        if files.len() == previous {
            break;
        }
    }
    let mut snapshot = sources::capture(
        workspace,
        control,
        &files.keys().cloned().collect::<Vec<_>>(),
    )?;
    anyhow::ensure!(
        snapshot.files == files,
        "Sources changed during Lake import collection; reread the graph"
    );
    snapshot.collection = Some(Collection {
        roots,
        external_modules: external.into_iter().collect(),
        modules: modules.into_values().collect(),
        packages: layout.packages,
        metadata: reader.metadata,
    });
    Ok(snapshot)
}
fn insert(modules: &mut BTreeMap<String, Module>, module: &Module) -> Result<()> {
    module.validate()?;
    anyhow::ensure!(
        !modules
            .values()
            .any(|old| old.file == module.file && old.name != module.name),
        "One source cannot be compiled under two module identities"
    );
    if let Some(old) = modules.insert(module.name.clone(), module.clone()) {
        anyhow::ensure!(
            old == *module,
            "Conflicting Lean module identities; no partial graph"
        );
    }
    Ok(())
}
pub(crate) fn check_metadata(
    workspace: &Workspace,
    control: &Control,
    expected: &Metadata,
) -> Result<()> {
    let mut reader = Reader::new(workspace, control);
    for (path, value) in &expected.files {
        anyhow::ensure!(reader.read(path, true)? == *value, "Lake configuration/import resolution changed at {path}; reread the graph and explicitly rerun");
    }
    for (directory, entries) in &expected.directories {
        reader.glob_entries(directory)?;
        anyhow::ensure!(
            reader.metadata.directories.get(directory) == Some(entries),
            "Lake source inventory changed at {directory}; reread the graph"
        );
    }
    Ok(())
}
