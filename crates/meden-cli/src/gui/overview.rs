//! Read-only workspace inventory through the policy-bound file worker. This is
//! a lexical navigation index, never compiler evidence or contract acceptance.
use super::{lock, View};
use crate::adapters::source::Symbol;
use anyhow::{Context, Result};
use meden_agent::tool::Args;
use meden_core::file::{Operation, Request};
use meden_runtime::{
    process::{Control, Limits},
    workspace::Workspace,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
mod cache;

use meden_app::navigation::collection;
use meden_app::view::inventory;
pub(super) use meden_app::view::inventory::{State, Status};
use meden_app::view::publication::ArtifactId;
pub(super) type File = collection::File<Symbol>;

pub(super) type Captured = meden_app::navigation::saved::Inventory<Symbol>;

pub(super) struct Snapshot {
    captured: Arc<Captured>,
    prepared: inventory::Prepared,
    chunks: Mutex<inventory::Chunks>,
}
impl std::ops::Deref for Snapshot {
    type Target = Captured;
    fn deref(&self) -> &Captured {
        &self.captured
    }
}
impl Snapshot {
    fn new(captured: Captured) -> Result<Self> {
        let data = inventory::Input {
            generation: captured.generation.to_string(),
            files: captured
                .files
                .iter()
                .map(project_file)
                .collect::<Result<_>>()?,
            excluded: captured.excluded.iter().map(ToString::to_string).collect(),
            inventory_only: captured.inventory_only,
            indexer: format!("sha256:{}", crate::adapters::source::identity()),
        };
        let prepared = inventory::Prepared::new(&data)?;
        let chunks = Mutex::new(inventory::Chunks::new(prepared.reference().clone())?);
        Ok(Self {
            captured: Arc::new(captured),
            prepared,
            chunks,
        })
    }
    pub(super) fn revision(&self) -> &ArtifactId<inventory::Artifact> {
        &self.prepared.reference().revision
    }
    pub(super) fn bytes(&self, revision: &ArtifactId<inventory::Artifact>) -> Option<Arc<[u8]>> {
        self.prepared.bytes(revision)
    }
}
fn project_file(file: &File) -> Result<inventory::File> {
    Ok(inventory::File {
        path: file.path.clone(),
        sha256: file.sha256.clone(),
        bytes: file.bytes.to_string(),
        declarations: u32::try_from(file.declarations)?,
        theorems: u32::try_from(file.theorems)?,
        imports: file.imports.clone(),
        symbols: file
            .symbols
            .iter()
            .cloned()
            .map(inventory::Symbol::try_from)
            .collect::<Result<_>>()?,
        indexed: file.indexed,
    })
}
use collection::EXCLUDED;

pub(super) fn refresh(
    view: &Arc<Mutex<View>>,
    workspace: &Workspace,
    control: &Control,
) -> Result<()> {
    let fresh = {
        let view = lock(view);
        view.overview_snapshot.is_some() || view.overview.state != State::Loading
    };
    let mut restored = None;
    if !fresh {
        // Saved display metadata is separate from current source/analysis inputs.
        if let Ok(Some(captured)) = saved(workspace).and_then(|s| s.load()) {
            if let Ok(snapshot) = Snapshot::new(captured) {
                let snapshot = Arc::new(snapshot);
                lock(view).overview_snapshot = Some(Arc::clone(&snapshot));
                restored = Some(snapshot);
            }
        }
    }
    {
        let mut view = lock(view);
        view.overview = Status::default();
        if let Some(snapshot) = view.overview_snapshot.clone() {
            view.overview.revision = Some(snapshot.revision().to_owned());
            view.overview.total = snapshot.files.len();
        }
        view.sources.clear();
        view.readers.clear();
        view.directories.clear();
        view.source_preparation.clear();
        view.activity = "Reading workspace sources and proof declarations".into();
    }
    let result = capture(view, workspace, control, fresh).and_then(|mut snapshot| {
        for path in snapshot
            .files
            .iter()
            .filter(|f| f.path.split('/').count() <= 2 && f.path.ends_with("Main.lean"))
            .take(8)
            .map(|f| f.path.clone())
            .collect::<Vec<_>>()
        {
            super::sources::prepare(view, workspace, control, &path, false)?;
        }
        apply_annotations(workspace, control, &mut snapshot.files);
        if let Some(original) = restored.filter(|saved| *saved.captured == snapshot) {
            // Fresh complete observations match every retained metadata value.
            // Reuse only the validated publication, with a new detail cache.
            return Ok(Snapshot {
                captured: Arc::clone(&original.captured),
                prepared: original.prepared.clone(),
                chunks: Mutex::new(inventory::Chunks::new(
                    original.prepared.reference().clone(),
                )?),
            });
        }
        let snapshot = Snapshot::new(snapshot)?;
        let _ = saved(workspace).and_then(|s| s.retain(&snapshot.captured));
        Ok(snapshot)
    });
    let mut view = lock(view);
    match result {
        Ok(snapshot) => {
            view.overview.state = State::Ready;
            view.overview.scanned = snapshot.files.len();
            view.overview.revision = Some(snapshot.revision().to_owned());
            view.overview_snapshot = Some(Arc::new(snapshot));
            view.activity = "Workspace loaded".into();
            Ok(())
        }
        Err(error) => {
            view.overview.state = State::Failed;
            view.overview.error = Some(format!("{error:#}"));
            Err(error)
        }
    }
}

fn capture(
    view: &Mutex<View>,
    workspace: &Workspace,
    control: &Control,
    fresh: bool,
) -> Result<Captured> {
    let started = Instant::now();
    let paths = collection::paths(
        &Reader {
            workspace,
            control,
            started,
        },
        &crate::adapters::navigation::Syntax,
    )?;
    lock(view).overview.total = paths.len();
    let inventory_only = paths.len() > 10_000;
    let files = if inventory_only {
        inventory_files(view, workspace, control, &paths, started, fresh)?
    } else {
        source_files(view, workspace, control, &paths, started)?
    };
    let generation = if inventory_only {
        cache::generation(workspace)?
    } else {
        0
    };
    Ok(Captured {
        files,
        excluded: EXCLUDED.iter().map(ToString::to_string).collect(),
        inventory_only,
        generation,
    })
}

fn inventory_files(
    view: &Mutex<View>,
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
    started: Instant,
    fresh: bool,
) -> Result<Vec<File>> {
    cache::prepare(view, workspace, control, paths, started, fresh)?;
    // Keep a complete file inventory without parsing gigabytes of declarations.
    // Individual files are indexed through the same policy-bound read on open.
    let mut inventory: Vec<File> = paths
        .iter()
        .map(|path| File::unindexed(path.clone(), 0))
        .collect();
    // Keep entry-module theorem locations available even in a distant map.
    let entries = paths
        .iter()
        .filter(|p| {
            p.split('/').count() <= 2 && Path::new(p).file_name().is_some_and(|n| n == "Main.lean")
        })
        .take(32)
        .cloned()
        .collect::<Vec<_>>();
    for entry in entries {
        if let Ok(files) = source_files(
            view,
            workspace,
            control,
            std::slice::from_ref(&entry),
            started,
        ) {
            for file in files {
                if let Ok(index) = inventory.binary_search_by(|old| old.path.cmp(&file.path)) {
                    *inventory
                        .get_mut(index)
                        .context("Missing entry-module slot")? = file;
                }
            }
        }
    }
    Ok(inventory)
}

/// Expand a large project's visible file and nearby import-only modules.
/// Unread files remain explicit inventory entries, never compiler evidence.
pub(super) fn index_file(view: &Mutex<View>, workspace: &Workspace, control: &Control, path: &str) {
    if let Err(error) = index_sources(view, workspace, control, path) {
        lock(view).overview.error = Some(format!(
            "Could not index {path}: {error:#}. Source browsing remains available."
        ));
    }
}

pub(super) fn observe_file(
    view: &Mutex<View>,
    workspace: &Workspace,
    control: &Control,
    path: &str,
) {
    let large = lock(view)
        .overview_snapshot
        .as_ref()
        .is_some_and(|s| s.inventory_only);
    if large {
        if let Err(error) = cache::prepare(
            view,
            workspace,
            control,
            &[path.into()],
            Instant::now(),
            true,
        ) {
            lock(view).overview.error = Some(format!("Could not update {path}: {error:#}"));
            return;
        }
    }
    index_file(view, workspace, control, path);
}

fn index_sources(
    view: &Mutex<View>,
    workspace: &Workspace,
    control: &Control,
    path: &str,
) -> Result<()> {
    let snapshot = lock(view).overview_snapshot.clone();
    let Some(snapshot) = snapshot.filter(|s| s.files.iter().any(|f| f.path == path)) else {
        return Ok(());
    };
    let mut indexed = if snapshot.inventory_only {
        cached_sources(&snapshot, workspace, path)?
    } else {
        source_files(view, workspace, control, &[path.into()], Instant::now())?
    };
    apply_annotations(workspace, control, &mut indexed);
    let changed = indexed.iter().any(|file| {
        snapshot
            .files
            .binary_search_by(|old| old.path.cmp(&file.path))
            .ok()
            .and_then(|i| snapshot.files.get(i))
            .is_some_and(|old| old != file)
    });
    if !changed {
        return Ok(());
    }
    let mut captured = snapshot.captured.as_ref().clone();
    for file in indexed {
        if let Ok(index) = captured
            .files
            .binary_search_by(|old| old.path.cmp(&file.path))
        {
            if let Some(original) = captured.files.get_mut(index) {
                *original = file;
            }
        }
    }
    // Projection/hash/serialization must not hold the status lock. Querying the
    // original publication remains available throughout this preparation.
    let next = Snapshot::new(captured)?;
    let mut current = lock(view);
    anyhow::ensure!(
        current
            .overview_snapshot
            .as_ref()
            .is_some_and(|s| Arc::ptr_eq(s, &snapshot)),
        "Inventory changed during navigation preparation"
    );
    current.overview.revision = Some(next.revision().to_owned());
    current.overview_snapshot = Some(Arc::new(next));
    Ok(())
}

fn apply_annotations(workspace: &Workspace, control: &Control, files: &mut [File]) {
    // Optional annotation failure leaves lexical navigation available.
    let _ = crate::adapters::navigation::annotate(
        &crate::adapters::Context {
            workspace,
            control,
            observer: None,
        },
        files
            .iter_mut()
            .filter(|f| f.indexed)
            .map(|f| (f.path.as_str(), f.symbols.as_mut_slice())),
    );
}

struct Reader<'a> {
    workspace: &'a Workspace,
    control: &'a Control,
    started: Instant,
}
impl collection::Reader for Reader<'_> {
    fn list(&self, paths: &[String]) -> Result<Vec<Vec<meden_app::view::directory::Entry>>> {
        observe_batch(
            self.workspace,
            self.control,
            Operation::List,
            paths,
            &json!({"entries":true,"max_entries":100_000}),
            self.started,
        )?
        .into_iter()
        .map(|v| serde_json::from_value(v).map_err(Into::into))
        .collect()
    }
    fn read(&self, paths: &[String]) -> Result<Vec<meden_app::view::file::Content>> {
        observe_batch(
            self.workspace,
            self.control,
            Operation::Snapshot,
            paths,
            &json!({"observation":true}),
            self.started,
        )?
        .into_iter()
        .map(|v| serde_json::from_value(v).map_err(Into::into))
        .collect()
    }
}
fn source_files(
    view: &Mutex<View>,
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
    started: Instant,
) -> Result<Vec<File>> {
    collection::sources(
        &Reader {
            workspace,
            control,
            started,
        },
        &crate::adapters::navigation::Syntax,
        paths,
        || {
            let mut view = lock(view);
            if view.overview.state == State::Loading
                && view.activity != "Preparing cached project map"
            {
                view.overview.scanned = view
                    .overview
                    .scanned
                    .saturating_add(1)
                    .min(view.overview.total);
            }
        },
    )
}

fn observe_batch(
    workspace: &Workspace,
    control: &Control,
    operation: Operation,
    paths: &[String],
    payload: &Value,
    started: Instant,
) -> Result<Vec<Value>> {
    anyhow::ensure!(
        started.elapsed() < Duration::from_secs(1800),
        "Workspace preparation exceeded 30 minutes"
    );
    let requests = paths
        .iter()
        .map(|path| -> Result<_> {
            let path = crate::workspace::logical_path(path)
                .context("Workspace index path leaves the selected root")?;
            Ok(Request::new(
                operation,
                path.to_str()
                    .context("Workspace index path is not UTF-8")?
                    .into(),
                serde_json::from_value::<Args>(payload.clone())?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    meden_runtime::file::read_batch(
        workspace,
        control,
        &requests,
        &crate::workspace::file_policy()?,
        &Limits {
            timeout: Duration::from_secs(30),
            capture_bytes: 67_108_864,
            output_bytes: 67_108_864,
        },
    )
    .map_err(anyhow::Error::new)
}

pub(super) fn cached(
    view: &Mutex<View>,
    workspace: &Workspace,
    revision: &ArtifactId<inventory::Artifact>,
    path: &str,
) -> Result<Arc<[u8]>> {
    let snapshot = lock(view)
        .overview_snapshot
        .clone()
        .context("Project map is still preparing")?;
    anyhow::ensure!(
        snapshot.revision() == revision,
        "Project map inventory changed"
    );
    anyhow::ensure!(
        snapshot
            .files
            .binary_search_by(|file| file.path.as_str().cmp(path))
            .is_ok(),
        "Source is outside the prepared map inventory"
    );
    if let Some(bytes) = lock(&snapshot.chunks).bytes(path) {
        return Ok(bytes);
    }
    let files = cached_sources(&snapshot, workspace, path)?
        .iter()
        .map(project_file)
        .collect::<Result<Vec<_>>>()?;
    for file in &files {
        file.validate()?;
    }
    anyhow::ensure!(
        lock(view)
            .overview_snapshot
            .as_ref()
            .is_some_and(|s| Arc::ptr_eq(s, &snapshot)),
        "Project map inventory changed during its read"
    );
    let bytes = lock(&snapshot.chunks).publish(
        path,
        &inventory::Chunk {
            revision: revision.clone(),
            files,
        },
    )?;
    Ok(bytes)
}

fn cached_sources(snapshot: &Snapshot, workspace: &Workspace, path: &str) -> Result<Vec<File>> {
    if !snapshot.inventory_only {
        return Ok(snapshot
            .files
            .iter()
            .filter(|file| file.path == path)
            .cloned()
            .collect());
    }
    let mut root = snapshot
        .files
        .first()
        .and_then(|f| f.path.rsplit_once('/'))
        .map_or("", |(dir, _)| dir)
        .to_owned();
    while !root.is_empty()
        && snapshot
            .files
            .iter()
            .any(|f| !f.path.starts_with(&format!("{root}/")))
    {
        root = root.rsplit_once('/').map_or("", |(dir, _)| dir).to_owned();
    }
    let resolve = |module: &str| -> Option<String> {
        let relative = format!("{}.lean", module.replace('.', "/"));
        for candidate in [format!("{root}/{relative}"), relative.clone()] {
            if snapshot
                .files
                .binary_search_by(|f| f.path.cmp(&candidate))
                .is_ok()
            {
                return Some(candidate);
            }
        }
        let suffix = format!("/{relative}");
        let mut matches = snapshot.files.iter().filter(|f| f.path.ends_with(&suffix));
        let found = matches.next()?;
        matches.next().is_none().then(|| found.path.clone())
    };
    let mut indexed = vec![cache::load(workspace, path)?
        .context("Project map cache is unavailable; refresh the workspace")?];
    let mut seen = BTreeSet::from([path.to_owned()]);
    let mut cursor = 0;
    while cursor < indexed.len() && indexed.len() < 32 {
        let file = indexed.get(cursor).context("Invalid navigation cursor")?;
        let neighbors = if cursor == 0 || file.declarations == 0 {
            file.imports
                .iter()
                .filter_map(|module| resolve(module))
                .filter(|p| seen.insert(p.clone()))
                .take(32usize.saturating_sub(indexed.len()))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        if !neighbors.is_empty() {
            for path in neighbors {
                indexed.push(
                    cache::load(workspace, &path)?
                        .context("Project map cache is incomplete; refresh the workspace")?,
                );
            }
        }
        cursor = cursor.saturating_add(1);
    }
    Ok(indexed)
}

fn saved(workspace: &Workspace) -> Result<meden_app::navigation::saved::Store> {
    meden_app::navigation::saved::Store::host(
        workspace.path(),
        crate::adapters::navigation::identity(),
    )
}
