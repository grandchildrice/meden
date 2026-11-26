//! Bounded navigation collection, independent of a viewer or source language.
//! Reader implementations own policy-bound I/O and observation attribution.
use crate::view::{
    directory::{Entry, Kind},
    file::Content,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};

/// Navigation metadata, not compilation, a source snapshot or proof evidence.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct File<S> {
    /// Original relative workspace path.
    pub path: String,
    /// Complete text digest; empty when unindexed.
    pub sha256: String,
    /// Exact observed byte size, including unread size-limit observations.
    pub bytes: usize,
    /// Lexical declaration count.
    pub declarations: usize,
    /// Lexical claim count.
    pub theorems: usize,
    /// Original lexical import names.
    pub imports: Vec<String>,
    /// Backend-owned navigation headings.
    pub symbols: Vec<S>,
    /// Whether a complete text index was produced, never compiler success.
    pub indexed: bool,
}
impl<S> File<S> {
    /// Explicit unread navigation, with no invented digest or declarations.
    pub fn unindexed(path: String, bytes: usize) -> Self {
        Self {
            path,
            bytes,
            sha256: String::new(),
            declarations: 0,
            theorems: 0,
            imports: Vec::new(),
            symbols: Vec::new(),
            indexed: false,
        }
    }
}
/// Complete syntax-only indexing result; symbols remain backend-owned.
pub struct Indexed<S> {
    /// Observed lexical count.
    pub declarations: usize,
    /// Observed lexical claim count.
    pub theorems: usize,
    /// Original import-name candidates.
    pub imports: Vec<String>,
    /// Original declaration headings, without inferred evidence.
    pub symbols: Vec<S>,
}
/// Trusted host-selected pure indexing implementation.
pub trait Indexer: Sync {
    /// Backend-owned decoded heading type.
    type Symbol: Send;
    /// Pure format availability; must not execute preparation.
    fn supports(&self, path: &str) -> bool;
    /// Index the complete original text, without compilation or inference.
    fn index(&self, path: &str, text: &str) -> Result<Indexed<Self::Symbol>>;
}
/// Complete, position-preserving observations from the existing file worker.
/// A reader cannot substitute another operation via an untyped operation label.
pub trait Reader: Sync {
    /// Desired source request width; collection enforces the worker's 256-path bound.
    fn source_batch(&self) -> usize;
    /// One complete listing per original requested directory, in request order.
    fn list(&self, paths: &[String]) -> Result<Vec<Vec<Entry>>>;
    /// One complete availability observation per requested source, in request order.
    fn read(&self, paths: &[String]) -> Result<Vec<Content>>;
}
/// Build artifacts and private/hidden directory entries are not navigated.
pub const EXCLUDED: &[&str] = &["node_modules", "target", "dist", "__pycache__"];
#[derive(Clone, Copy)]
struct Bounds {
    directories: usize,
    depth: usize,
    entries: usize,
    files: usize,
}
const BOUNDS: Bounds = Bounds {
    directories: 65_536,
    depth: 32,
    entries: 1_000_000,
    files: 250_000,
};

/// Discover supported paths without following symlinks or special entries.
/// Failure returns no successful prefix; directory/resource limits are fixed.
pub fn paths(reader: &impl Reader, indexer: &impl Indexer) -> Result<Vec<String>> {
    let _timing = meden_runtime::metrics::Span::start("navigation.collect.paths");
    walk(reader, indexer, BOUNDS)
}
fn walk(reader: &impl Reader, indexer: &impl Indexer, bounds: Bounds) -> Result<Vec<String>> {
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
            directories <= bounds.directories
                && batch.iter().all(|(_, depth)| *depth <= bounds.depth),
            "Workspace index exceeds directory/depth limits"
        );
        let values = reader.list(
            &batch
                .iter()
                .map(|(path, _)| path.clone())
                .collect::<Vec<_>>(),
        )?;
        anyhow::ensure!(
            values.len() == batch.len(),
            "Incomplete workspace directory observations"
        );
        for ((directory, depth), entries) in batch.into_iter().zip(values) {
            anyhow::ensure!(
                entries.len() <= 100_000,
                "Workspace directory exceeds 100000 entries"
            );
            entry_count = entry_count.saturating_add(entries.len());
            anyhow::ensure!(
                entry_count <= bounds.entries,
                "Workspace index exceeds one million directory entries"
            );
            for entry in entries {
                anyhow::ensure!(
                    !entry.name.is_empty()
                        && entry.name.len() <= 255
                        && !entry.name.contains(['/', '\0'])
                        && !matches!(entry.name.as_str(), "." | ".."),
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
                anyhow::ensure!(
                    path.len() <= 4096,
                    "Workspace inventory path exceeds 4096 bytes"
                );
                match entry.kind {
                    Kind::Directory => pending.push((path, depth.saturating_add(1))),
                    Kind::File if indexer.supports(&path) => {
                        paths.push(path);
                        anyhow::ensure!(
                            paths.len() <= bounds.files,
                            "Workspace inventory exceeds 250000 source files"
                        );
                    }
                    _ => {}
                }
            }
        }
    }
    paths.sort();
    anyhow::ensure!(
        paths.windows(2).all(|p| matches!(p, [a,b] if a < b)),
        "Duplicate workspace inventory path"
    );
    Ok(paths)
}

fn source<I: Indexer>(
    indexer: &I,
    path: &str,
    content: Content,
) -> Result<(File<I::Symbol>, usize)> {
    content.validate()?;
    let (size, observed) = match &content {
        Content::Text { bytes, .. }
        | Content::Binary { bytes }
        | Content::UnsupportedEncoding { bytes } => {
            (usize::try_from(*bytes)?, usize::try_from(*bytes)?)
        }
        Content::SizeLimit { bytes } => (bytes.parse::<usize>()?, 0),
    };
    let mut file = File::unindexed(path.into(), size);
    if let Content::Text { text, .. } = content {
        if !indexer.supports(path) {
            return Ok((file, observed));
        }
        let syntax = {
            let _timing = meden_runtime::metrics::Span::start("navigation.index");
            indexer.index(path, &text)?
        };
        let Indexed {
            declarations,
            theorems,
            imports,
            symbols,
        } = syntax;
        file.sha256 = format!("{:x}", Sha256::digest(text.as_bytes()));
        file.declarations = declarations;
        file.theorems = theorems;
        file.imports = imports;
        file.symbols = symbols;
        file.indexed = true;
    }
    Ok((file, observed))
}
/// Four bounded readers and a complete 128 MiB observation budget.
/// The observer reports completed items only, without publishing partial results.
pub fn sources<I: Indexer>(
    reader: &impl Reader,
    indexer: &I,
    paths: &[String],
    observed: impl Fn() + Sync,
) -> Result<Vec<File<I::Symbol>>> {
    read_all(reader, indexer, paths, &observed, 134_217_728)
}
pub(super) fn check_paths(paths: &[String]) -> Result<()> {
    anyhow::ensure!(
        paths.len() <= BOUNDS.files,
        "Workspace inventory exceeds 250000 source files"
    );
    let mut unique = std::collections::BTreeSet::new();
    for path in paths {
        anyhow::ensure!(
            !path.is_empty()
                && path.len() <= 4096
                && !path.contains('\0')
                && path
                    .split('/')
                    .all(|p| !p.is_empty() && p != "." && p != "..")
                && unique.insert(path),
            "Invalid or duplicate workspace source path"
        );
    }
    Ok(())
}
fn read_all<I: Indexer>(
    reader: &impl Reader,
    indexer: &I,
    paths: &[String],
    observed: &(impl Fn() + Sync),
    limit: usize,
) -> Result<Vec<File<I::Symbol>>> {
    let timing = meden_runtime::metrics::Span::start("navigation.collect.sources");
    let parent = timing.context();
    check_paths(paths)?;
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let width = reader.source_batch();
    anyhow::ensure!((1..=256).contains(&width), "Invalid source request width");
    let cursor = AtomicUsize::new(0);
    let bytes = AtomicUsize::new(0);
    let files = Mutex::new(BTreeMap::new());
    std::thread::scope(|scope| -> Result<()> {
        let handles = (0..4)
            .map(|_| {
                scope.spawn(|| -> Result<()> {
                    let _worker = meden_runtime::metrics::Span::linked(
                        "navigation.collect.worker",
                        Some(parent),
                    );
                    loop {
                        let index = cursor.fetch_add(width, Ordering::SeqCst);
                        if index >= paths.len() {
                            break;
                        }
                        let batch = paths
                            .get(index..index.saturating_add(width).min(paths.len()))
                            .context("Invalid source batch")?;
                        let values = reader.read(batch)?;
                        anyhow::ensure!(
                            values.len() == batch.len(),
                            "Incomplete workspace source observations"
                        );
                        for (path, value) in batch.iter().zip(values) {
                            let (file, observed_bytes) = source(indexer, path, value)?;
                            anyhow::ensure!(
                                bytes
                                    .fetch_add(observed_bytes, Ordering::SeqCst)
                                    .saturating_add(observed_bytes)
                                    <= limit,
                                "Workspace sources exceed 128 MiB"
                            );
                            files
                                .lock()
                                .map_err(|_| anyhow::anyhow!("Workspace index unavailable"))?
                                .insert(path.clone(), file);
                            observed();
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
#[cfg(test)]
mod tests;
