//! Bounded project import closure with frozen Lake configuration and path bindings.
use super::layout::{Layout, Module, PackageView};
use crate::{
    analysis::sources,
    workspace::observation::{relative, Metadata, Reader},
};
use anyhow::Result;
use meden_runtime::{process::Control, workspace::Workspace};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::{Duration, Instant},
};

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Collection {
    pub roots: Vec<String>,
    pub external_modules: Vec<String>,
    pub modules: Vec<Module>,
    pub packages: Vec<PackageView>,
    pub metadata: Metadata,
}
#[derive(Serialize)]
pub struct Collected {
    #[serde(flatten)]
    pub source: sources::Snapshot,
    pub collection: Collection,
}
pub fn capture(
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
    atlas: crate::atlas::Configuration,
) -> Result<Collected> {
    let Selection {
        atlas,
        mut reader,
        layout,
        mut files,
        mut modules,
        roots,
    } = select(workspace, control, paths, atlas)?;
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
                            && files.values().map(String::len).sum::<usize>() <= 16_777_216,
                        "Local import closure exceeds 32 files/16 MiB; no partial graph admitted"
                    );
                }
            }
        }
        if files.len() == previous {
            break;
        }
    }
    let source = sources::capture(
        workspace,
        control,
        &files.keys().cloned().collect::<Vec<_>>(),
    )?;
    anyhow::ensure!(
        source.files == files,
        "Sources changed during Lake import collection; reread the graph"
    );
    Ok(Collected {
        source,
        collection: Collection {
            roots,
            external_modules: external.into_iter().collect(),
            modules: modules.into_values().collect(),
            packages: layout.packages,
            metadata: reader.metadata,
        },
    })
}
struct Selection<'a> {
    atlas: crate::atlas::Configuration,
    reader: Reader<'a>,
    layout: Layout,
    files: BTreeMap<String, String>,
    modules: BTreeMap<String, Module>,
    roots: Vec<String>,
}

fn select<'a>(
    workspace: &'a Workspace,
    control: &'a Control,
    paths: &[String],
    atlas: crate::atlas::Configuration,
) -> Result<Selection<'a>> {
    let paths = paths
        .iter()
        .map(|path| relative("", path))
        .collect::<Result<Vec<_>>>()?;
    let mut reader = Reader::new(workspace, control, super::layout::source_entry);
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
        anyhow::ensure!(!files.is_empty(), "Select a Lean source file, or use /graph in a Lake project with library/executable roots");
    } else {
        for path in files.keys() {
            insert(&mut modules, &layout.bind_selected(path)?)?;
        }
    }

    let roots = files.keys().cloned().collect();
    Ok(Selection {
        atlas,
        reader,
        layout,
        files,
        modules,
        roots,
    })
}

/// Select observed roots without recursively copying installed dependency sources.
/// Prepared Lake imports remain explicitly unexpanded library boundaries.
pub fn roots(
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
    atlas: crate::atlas::Configuration,
) -> Result<Collected> {
    let _timing = meden_runtime::metrics::Span::start("analysis.select_roots");
    let Selection {
        reader,
        layout,
        files,
        modules,
        roots,
        ..
    } = select(workspace, control, paths, atlas)?;
    let source = sources::capture(
        workspace,
        control,
        &files.keys().cloned().collect::<Vec<_>>(),
    )?;
    anyhow::ensure!(
        source.files == files,
        "Sources changed during Lake root selection"
    );
    Ok(Collected {
        source,
        collection: Collection {
            roots,
            external_modules: Vec::new(),
            modules: modules.into_values().collect(),
            packages: layout.packages,
            metadata: reader.metadata,
        },
    })
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
pub fn check_metadata(workspace: &Workspace, control: &Control, expected: &Metadata) -> Result<()> {
    crate::workspace::observation::check_metadata(
        workspace,
        control,
        expected,
        super::layout::source_entry,
    )
}
