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
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
mod cache;

use meden_app::view::inventory;
pub(super) use meden_app::view::inventory::{State, Status};
use meden_app::view::publication::ArtifactId;
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct File {
    path: String,
    sha256: String,
    bytes: usize,
    declarations: usize,
    theorems: usize,
    imports: Vec<String>,
    symbols: Vec<Symbol>,
    indexed: bool,
}

#[derive(Clone)]
pub(super) struct Captured {
    files: Vec<File>,
    excluded: Vec<&'static str>,
    inventory_only: bool,
    generation: u64,
}

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
#[derive(Deserialize)]
struct Entry {
    name: String,
    kind: String,
}
const EXCLUDED: &[&str] = &["node_modules", "target", "dist", "__pycache__"];

pub(super) fn refresh(
    view: &Arc<Mutex<View>>,
    workspace: &Workspace,
    control: &Control,
) -> Result<()> {
    let fresh = {
        let view = lock(view);
        view.overview_snapshot.is_some() || view.overview.state != State::Loading
    };
    {
        let mut view = lock(view);
        view.overview = Status::default();
        view.overview_snapshot = None;
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
        Snapshot::new(snapshot)
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
    let mut pending = vec![(String::new(), 0usize)];
    let mut paths = Vec::new();
    let mut directories = 0usize;
    let mut entry_count = 0usize;
    while !pending.is_empty() {
        let batch = pending
            .drain(pending.len().saturating_sub(128)..)
            .collect::<Vec<_>>();
        directories = directories.saturating_add(batch.len());
        anyhow::ensure!(
            directories <= 65_536 && batch.iter().all(|(_, depth)| *depth <= 32),
            "Workspace index exceeds directory/depth limits"
        );
        let values = observe_batch(
            workspace,
            control,
            Operation::List,
            &batch
                .iter()
                .map(|(path, _)| path.clone())
                .collect::<Vec<_>>(),
            &json!({"entries":true,"max_entries":100_000}),
            started,
        )?;
        for ((directory, depth), value) in batch.into_iter().zip(values) {
            let entries: Vec<Entry> = serde_json::from_value(value)?;
            entry_count = entry_count.saturating_add(entries.len());
            anyhow::ensure!(
                entry_count <= 1_000_000,
                "Workspace index exceeds one million directory entries"
            );
            for entry in entries {
                anyhow::ensure!(
                    !entry.name.is_empty()
                        && Path::new(&entry.name).components().count() == 1
                        && Path::new(&entry.name)
                            .components()
                            .all(|c| matches!(c, std::path::Component::Normal(_))),
                    "Invalid workspace inventory entry"
                );
                if entry.name.starts_with('.') || EXCLUDED.contains(&entry.name.as_str()) {
                    continue;
                }
                let path = if directory.is_empty() {
                    entry.name
                } else {
                    format!("{directory}/{}", entry.name)
                };
                if entry.kind == "directory" {
                    pending.push((path, depth.saturating_add(1)));
                } else if entry.kind == "file" && crate::adapters::navigation::supports(&path) {
                    paths.push(path);
                    anyhow::ensure!(
                        paths.len() <= 250_000,
                        "Workspace inventory exceeds 250000 source files"
                    );
                }
                // Inventory symlinks and special files are never followed.
            }
        }
    }
    paths.sort();
    lock(view).overview.total = paths.len();
    let inventory_only = paths.len() > 10_000;
    let mut files = if inventory_only {
        inventory_files(view, workspace, control, &paths, started, fresh)?
    } else {
        source_files(view, workspace, control, &paths, started)?
    };
    apply_annotations(workspace, control, &mut files);
    let generation = if inventory_only {
        cache::generation(workspace)?
    } else {
        0
    };
    Ok(Captured {
        files,
        excluded: EXCLUDED.to_vec(),
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
        .map(|path| File {
            path: path.clone(),
            sha256: String::new(),
            bytes: 0,
            declarations: 0,
            theorems: 0,
            imports: Vec::new(),
            symbols: Vec::new(),
            indexed: false,
        })
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

fn source_file(path: &str, value: Value) -> Result<(File, usize)> {
    use meden_app::view::file::Content;
    let content: Content = serde_json::from_value(value)?;
    content.validate()?;
    let (size, observed) = match &content {
        Content::Text { bytes, .. }
        | Content::Binary { bytes }
        | Content::UnsupportedEncoding { bytes } => {
            (usize::try_from(*bytes)?, usize::try_from(*bytes)?)
        }
        Content::SizeLimit { bytes } => (bytes.parse::<usize>()?, 0),
    };
    let mut file = File {
        path: path.into(),
        sha256: String::new(),
        bytes: size,
        declarations: 0,
        theorems: 0,
        imports: Vec::new(),
        symbols: Vec::new(),
        indexed: false,
    };
    if let Content::Text { text, .. } = content {
        let (declarations, theorems, imports, symbols) =
            crate::adapters::navigation::index(path, &text);
        file.sha256 = format!("{:x}", Sha256::digest(text.as_bytes()));
        file.declarations = declarations;
        file.theorems = theorems;
        file.imports = imports;
        file.symbols = symbols;
        file.indexed = true;
    }
    Ok((file, observed))
}

fn source_files(
    view: &Mutex<View>,
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
    started: Instant,
) -> Result<Vec<File>> {
    let cursor = AtomicUsize::new(0);
    let bytes = AtomicUsize::new(0);
    let files = Mutex::new(BTreeMap::new());
    std::thread::scope(|scope| -> Result<()> {
        let handles = (0..4)
            .map(|_| {
                scope.spawn(|| -> Result<()> {
                    loop {
                        let index = cursor.fetch_add(32, Ordering::SeqCst);
                        if index >= paths.len() {
                            break;
                        }
                        let batch = paths
                            .get(index..index.saturating_add(32).min(paths.len()))
                            .context("Invalid source batch")?;
                        let values = observe_batch(
                            workspace,
                            control,
                            Operation::Snapshot,
                            batch,
                            &json!({"observation":true}),
                            started,
                        )?;
                        for (path, value) in batch.iter().zip(values) {
                            let (file, observed_bytes) = source_file(path, value)?;
                            anyhow::ensure!(
                                bytes
                                    .fetch_add(observed_bytes, Ordering::SeqCst)
                                    .saturating_add(observed_bytes)
                                    <= 134_217_728,
                                "Workspace sources exceed 128 MiB"
                            );
                            lock(&files).insert(path.clone(), file);
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
                        }
                    }
                    Ok(())
                })
            })
            .collect::<Vec<_>>();
        let mut failure = None;
        for handle in handles {
            let result = handle
                .join()
                .map_err(|_| anyhow::anyhow!("Workspace index worker stopped"))?;
            if let Err(error) = result {
                failure.get_or_insert(error);
            }
        }
        failure.map_or(Ok(()), Err)
    })?;
    let files = files
        .into_inner()
        .map_err(|_| anyhow::anyhow!("Workspace index unavailable"))?
        .into_values()
        .collect::<Vec<_>>();
    anyhow::ensure!(files.len() == paths.len(), "Incomplete workspace index");
    Ok(files)
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
