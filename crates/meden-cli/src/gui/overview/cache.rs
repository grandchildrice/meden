//! Prepared navigation cache. Metadata is a presentation freshness hint only;
//! compiler/evidence artifacts always reobserve full source bytes independently.
use super::{observe_batch, source_files, File, View};
use crate::gui::lock;
use anyhow::{Context, Result};
use meden_core::file::Operation;
use meden_runtime::{process::Control, workspace::Workspace};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    time::Instant,
};

#[derive(Serialize, Deserialize)]
struct Entry {
    path: String,
    stamp: Value,
    file: Option<File>,
    error: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Seal {
    inventory: String,
    generation: u64,
}

pub(super) fn generation(workspace: &Workspace) -> Result<u64> {
    Ok(
        crate::storage::read::<Seal>(&directory(workspace)?.join("complete.json"))?
            .map_or(0, |seal| seal.generation),
    )
}

fn directory(workspace: &Workspace) -> Result<std::path::PathBuf> {
    let parser = crate::adapters::source::identity();
    Ok(crate::storage::directory(workspace.path())?
        .join("map")
        .join(parser))
}
fn key(path: &str) -> String {
    let digest = crate::atlas::digest(path.as_bytes());
    format!(
        "{}/{digest}.json",
        digest.chars().take(2).collect::<String>()
    )
}

// Disposable map entries need atomic replacement, not durable journal fsyncs
// for every source. A crash/corrupt entry is repaired during preparation.
fn store(path: &std::path::Path, entry: &Entry) -> Result<()> {
    use std::io::Write as _;
    let bytes = serde_json::to_vec(entry)?;
    anyhow::ensure!(bytes.len() <= 16_777_216, "Source map entry exceeds 16 MiB");
    let parent = path.parent().context("Missing map cache parent")?;
    std::fs::create_dir_all(parent)?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    staged.write_all(&bytes)?;
    staged.persist(path).map_err(|error| error.error)?;
    Ok(())
}

pub(super) fn load(workspace: &Workspace, path: &str) -> Result<Option<File>> {
    let entry: Option<Entry> = crate::storage::read(&directory(workspace)?.join(key(path)))?;
    match entry {
        Some(entry) => {
            if let Some(error) = entry.error {
                anyhow::bail!("{error}")
            }
            Ok(entry.file.filter(|file| file.path == path))
        }
        None => Ok(None),
    }
}

pub(super) fn prepare(
    view: &Mutex<View>,
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
    started: Instant,
    fresh: bool,
) -> Result<()> {
    let directory = directory(workspace)?;
    let inventory = crate::atlas::digest(&serde_json::to_vec(paths)?);
    let complete: Option<Seal> = crate::storage::read(&directory.join("complete.json"))
        .ok()
        .flatten();
    if !fresh
        && complete
            .as_ref()
            .is_some_and(|seal| seal.inventory == inventory)
    {
        return Ok(());
    }
    let fresh = fresh
        || complete
            .as_ref()
            .is_some_and(|seal| seal.inventory != inventory);
    // Reopening resumes/reuses a prepared navigation snapshot. Explicit refresh
    // reobserves stamps. Compilation always validates complete source bytes.
    let pending = if fresh {
        paths.to_vec()
    } else {
        paths
            .iter()
            .filter(|path| !directory.join(key(path)).is_file())
            .cloned()
            .collect::<Vec<_>>()
    };
    let paths = pending.as_slice();
    let cursor = AtomicUsize::new(0);
    let completed = AtomicUsize::new(0);
    let changed = std::sync::atomic::AtomicBool::new(complete.is_none());
    let preparing = lock(view).overview.state == "loading";
    if preparing {
        lock(view).activity = "Preparing cached project map".into();
    }
    let builder = Builder {
        view,
        workspace,
        control,
        directory: &directory,
        started,
        changed: &changed,
    };
    std::thread::scope(|scope| -> Result<()> {
        let handles = (0..4)
            .map(|_| {
                scope.spawn(|| -> Result<()> {
                    loop {
                        let index = cursor.fetch_add(256, Ordering::SeqCst);
                        if index >= paths.len() {
                            break;
                        }
                        let batch = paths
                            .get(index..index.saturating_add(256).min(paths.len()))
                            .context("Invalid map preparation cursor")?;
                        builder.batch(batch)?;
                        if preparing {
                            let count = completed
                                .fetch_add(batch.len(), Ordering::SeqCst)
                                .saturating_add(batch.len());
                            lock(view).overview.scanned = count.min(paths.len());
                        }
                    }
                    Ok(())
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("Map preparation stopped"))??;
        }
        Ok(())
    })?;
    if preparing {
        let previous = complete.map_or(0, |seal| seal.generation);
        let generation = if changed.load(Ordering::SeqCst) {
            previous
                .checked_add(1)
                .context("Map cache generation overflow")?
        } else {
            previous
        };
        crate::storage::write(
            &directory.join("complete.json"),
            &Seal {
                inventory,
                generation,
            },
        )?;
    }
    Ok(())
}

struct Builder<'a> {
    view: &'a Mutex<View>,
    workspace: &'a Workspace,
    control: &'a Control,
    directory: &'a std::path::Path,
    started: Instant,
    changed: &'a std::sync::atomic::AtomicBool,
}

impl Builder<'_> {
    fn batch(&self, batch: &[String]) -> Result<()> {
        let stamps = observe_batch(
            self.workspace,
            self.control,
            Operation::Snapshot,
            batch,
            &json!({"metadata":true}),
            self.started,
        )?;
        let mut pending = BTreeMap::new();
        for (path, stamp) in batch.iter().zip(stamps) {
            let previous = crate::storage::read::<Entry>(&self.directory.join(key(path)))
                .ok()
                .flatten();
            if previous.is_some_and(|e| e.path == *path && e.stamp == stamp) {
                continue;
            }
            self.changed.store(true, Ordering::SeqCst);
            pending.insert(path.clone(), stamp);
        }
        if !pending.is_empty() {
            let changed = pending.keys().cloned().collect::<Vec<_>>();
            let files = if let Ok(files) = source_files(
                self.view,
                self.workspace,
                self.control,
                &changed,
                self.started,
            ) {
                files
            } else {
                self.control.with_active(|| Ok(()))?;
                let mut files = Vec::new();
                for path in &changed {
                    match source_files(
                        self.view,
                        self.workspace,
                        self.control,
                        std::slice::from_ref(path),
                        self.started,
                    ) {
                        Ok(batch) => files.extend(batch),
                        Err(error) => {
                            self.control.with_active(|| Ok(()))?;
                            let stamp = pending.remove(path).context("Missing source stamp")?;
                            store(
                                &self.directory.join(key(path)),
                                &Entry {
                                    path: path.clone(),
                                    stamp,
                                    file: None,
                                    error: Some(format!("{error:#}")),
                                },
                            )?;
                        }
                    }
                }
                files
            };
            for file in files {
                let stamp = pending.remove(&file.path).context("Missing source stamp")?;
                // The stamp is only an index hint; source hash is retained
                // for diagnostics and compiled artifacts use exact bytes.
                store(
                    &self.directory.join(key(&file.path)),
                    &Entry {
                        path: file.path.clone(),
                        stamp,
                        file: Some(file),
                        error: None,
                    },
                )?;
            }
        }
        Ok(())
    }
}
