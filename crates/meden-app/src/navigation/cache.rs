//! Disposable syntax navigation cache. Metadata is only a presentation hint;
//! compiler/evidence inputs must still be reobserved independently.
use super::collection::File;
use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

/// Host observations and syntax indexing; this interface cannot compile or infer.
pub trait Reader: Sync {
    /// Backend-owned navigation heading data.
    type Symbol: Send;
    /// An opaque metadata hint, never a source/evidence identity.
    type Stamp: Serialize + DeserializeOwned + PartialEq;
    /// Complete position-preserving hints for the original requested paths.
    fn stamps(&self, paths: &[String]) -> Result<Vec<Self::Stamp>>;
    /// Complete syntax/availability records for exactly the requested paths.
    fn sources(&self, paths: &[String]) -> Result<Vec<File<Self::Symbol>>>;
    /// Report cancellation or a host stop before retry/publication.
    fn active(&self) -> Result<()>;
}
/// Only complete inventory preparation publishes its completion marker.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Prepare and seal the complete original navigation inventory.
    Inventory,
    /// Prepare selected entries without replacing the inventory seal.
    Selection,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry<S, M> {
    path: String,
    stamp: M,
    file: Option<File<S>>,
    error: Option<String>,
}
impl<S, M> Entry<S, M> {
    fn valid(&self, path: &str) -> bool {
        self.path == path
            && match (&self.file, &self.error) {
                (Some(file), None) => file.path == path,
                (None, Some(error)) => !error.is_empty(),
                _ => false,
            }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Seal {
    inventory: String,
    generation: u64,
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
/// Host-scoped derived storage, independent of source execution or proof receipts.
pub struct Store<S, M> {
    directory: PathBuf,
    additional: fn(&str) -> Option<String>,
    types: PhantomData<fn() -> (S, M)>,
}
impl<S: Serialize + DeserializeOwned, M: Serialize + DeserializeOwned + PartialEq> Store<S, M> {
    /// Bind host-selected syntax identities before any cache read.
    pub fn host(root: &Path, parser: &str, additional: fn(&str) -> Option<String>) -> Result<Self> {
        anyhow::ensure!(
            parser.len() == 64
                && parser
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "Invalid navigation parser identity"
        );
        Ok(Self {
            directory: crate::storage::directory(root)?.join("map").join(parser),
            additional,
            types: PhantomData,
        })
    }
    fn key(&self, path: &str) -> String {
        let digest = (self.additional)(path).map_or_else(
            || digest(path.as_bytes()),
            |method| digest(format!("{method}\0{path}").as_bytes()),
        );
        format!(
            "{}/{digest}.json",
            digest.chars().take(2).collect::<String>()
        )
    }
    fn entry(&self, path: &str) -> Result<Option<Entry<S, M>>> {
        let entry: Option<Entry<S, M>> =
            crate::storage::read(&self.directory.join(self.key(path)))?;
        if let Some(entry) = &entry {
            anyhow::ensure!(
                entry.valid(path),
                "Invalid navigation cache entry for {path}"
            );
        }
        Ok(entry)
    }
    /// Original complete-inventory generation; not verification freshness.
    pub fn generation(&self) -> Result<u64> {
        Ok(
            crate::storage::read::<Seal>(&self.directory.join("complete.json"))?
                .map_or(0, |seal| seal.generation),
        )
    }
    /// Read a complete path-bound record or its explicit saved failure.
    pub fn load(&self, path: &str) -> Result<Option<File<S>>> {
        match self.entry(path)? {
            Some(entry) => {
                if let Some(error) = entry.error {
                    anyhow::bail!("{error}")
                }
                Ok(entry.file)
            }
            None => Ok(None),
        }
    }
    fn retain(&self, entry: &Entry<S, M>) -> Result<()> {
        use std::io::Write as _;
        anyhow::ensure!(entry.valid(&entry.path), "Invalid navigation cache result");
        let bytes = serde_json::to_vec(entry)?;
        anyhow::ensure!(bytes.len() <= 16_777_216, "Source map entry exceeds 16 MiB");
        let path = self.directory.join(self.key(&entry.path));
        let parent = path.parent().context("Missing map cache parent")?;
        std::fs::create_dir_all(parent)?;
        let mut staged = tempfile::NamedTempFile::new_in(parent)?;
        staged.write_all(&bytes)?;
        staged.persist(path).map_err(|error| error.error)?;
        Ok(())
    }
    /// Resume saved presentation on reopening; explicit refresh reobserves hints.
    /// Incomplete/foreign observations or an observed stop cannot seal a prefix.
    pub fn prepare<R: Reader<Symbol = S, Stamp = M>>(
        &self,
        reader: &R,
        paths: &[String],
        fresh: bool,
        scope: Scope,
        progress: impl Fn(usize) + Sync,
    ) -> Result<()> {
        let timing = meden_runtime::metrics::Span::start("navigation.cache.prepare");
        let parent = timing.context();
        super::collection::check_paths(paths)?;
        reader.active()?;
        let additional = paths
            .iter()
            .filter_map(|p| (self.additional)(p).map(|id| (p, id)))
            .collect::<Vec<_>>();
        let inventory = digest(&if additional.is_empty() {
            serde_json::to_vec(paths)?
        } else {
            serde_json::to_vec(&(paths, additional))?
        });
        let complete: Option<Seal> = crate::storage::read(&self.directory.join("complete.json"))
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
        let pending = if fresh {
            paths.to_vec()
        } else {
            paths
                .iter()
                .filter(|p| !self.directory.join(self.key(p)).is_file())
                .cloned()
                .collect()
        };
        let cursor = AtomicUsize::new(0);
        let completed = AtomicUsize::new(0);
        let changed = AtomicBool::new(complete.is_none());
        std::thread::scope(|threads| -> Result<()> {
            let handles = (0..4)
                .map(|_| {
                    threads.spawn(|| -> Result<()> {
                        let _worker = meden_runtime::metrics::Span::linked(
                            "navigation.cache.worker",
                            Some(parent),
                        );
                        loop {
                            let index = cursor.fetch_add(256, Ordering::SeqCst);
                            if index >= pending.len() {
                                break;
                            }
                            let batch = pending
                                .get(index..index.saturating_add(256).min(pending.len()))
                                .context("Invalid map preparation cursor")?;
                            self.batch(reader, batch, &changed)?;
                            progress(
                                completed
                                    .fetch_add(batch.len(), Ordering::SeqCst)
                                    .saturating_add(batch.len())
                                    .min(pending.len()),
                            );
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
        reader.active()?;
        if scope == Scope::Inventory {
            let previous = complete.map_or(0, |seal| seal.generation);
            let generation = if changed.load(Ordering::SeqCst) {
                previous
                    .checked_add(1)
                    .context("Map cache generation overflow")?
            } else {
                previous
            };
            crate::storage::write(
                &self.directory.join("complete.json"),
                &Seal {
                    inventory,
                    generation,
                },
            )?;
        }
        Ok(())
    }
    fn batch<R: Reader<Symbol = S, Stamp = M>>(
        &self,
        reader: &R,
        paths: &[String],
        changed: &AtomicBool,
    ) -> Result<()> {
        let stamps = reader.stamps(paths)?;
        anyhow::ensure!(
            stamps.len() == paths.len(),
            "Incomplete navigation metadata observations"
        );
        let mut pending = BTreeMap::new();
        for (path, stamp) in paths.iter().zip(stamps) {
            let previous = self.entry(path).ok().flatten();
            // Failed or corrupt entries are retryable even with the same hint.
            if previous.is_some_and(|e| e.stamp == stamp && e.file.is_some()) {
                continue;
            }
            changed.store(true, Ordering::SeqCst);
            pending.insert(path.clone(), stamp);
        }
        if pending.is_empty() {
            return Ok(());
        }
        let paths = pending.keys().cloned().collect::<Vec<_>>();
        let files = if let Ok(files) = reader.sources(&paths) {
            check_files(&paths, &files)?;
            files
        } else {
            reader.active()?;
            let mut files = Vec::new();
            for path in &paths {
                match reader.sources(std::slice::from_ref(path)) {
                    Ok(batch) => {
                        check_files(std::slice::from_ref(path), &batch)?;
                        files.extend(batch);
                    }
                    Err(error) => {
                        reader.active()?;
                        self.retain(&Entry {
                            path: path.clone(),
                            stamp: pending.remove(path).context("Missing source stamp")?,
                            file: None,
                            error: Some(format!("{error:#}")),
                        })?;
                    }
                }
            }
            files
        };
        for file in files {
            let stamp = pending.remove(&file.path).context("Missing source stamp")?;
            self.retain(&Entry {
                path: file.path.clone(),
                stamp,
                file: Some(file),
                error: None,
            })?;
        }
        Ok(())
    }
}
fn check_files<S>(paths: &[String], files: &[File<S>]) -> Result<()> {
    let expected = paths.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let actual = files
        .iter()
        .map(|f| f.path.as_str())
        .collect::<BTreeSet<_>>();
    anyhow::ensure!(
        files.len() == paths.len() && expected.len() == paths.len() && actual == expected,
        "Incomplete or foreign navigation source observations"
    );
    Ok(())
}

#[cfg(test)]
mod tests;
