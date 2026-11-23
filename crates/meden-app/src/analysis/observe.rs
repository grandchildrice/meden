//! Host observations use the same policy-bound, pinned file worker as source snapshots.
use anyhow::{Context, Result};
use base64::Engine as _;
use meden_agent::tool::Args;
use meden_core::file::{Operation, Request};
use meden_runtime::{
    process::{Control, Limits},
    workspace::Workspace,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path},
    time::Duration,
};

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub name: String,
    pub kind: String,
}

#[derive(Clone, Default, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub files: BTreeMap<String, Option<String>>,
    pub directories: BTreeMap<String, Option<Vec<Entry>>>,
}
pub(super) struct Reader<'a> {
    workspace: &'a Workspace,
    control: &'a Control,
    directories: BTreeMap<String, Vec<Entry>>,
    pub metadata: Metadata,
}
impl<'a> Reader<'a> {
    pub fn new(workspace: &'a Workspace, control: &'a Control) -> Self {
        Self {
            workspace,
            control,
            directories: BTreeMap::new(),
            metadata: Metadata::default(),
        }
    }
    fn request(operation: Operation, path: &str, payload: Args) -> Result<Request<Args>> {
        let path = crate::workspace::logical_path(path)
            .context("Lake path leaves the selected workspace")?;
        Ok(Request::new(
            operation,
            path.to_str().context("Lake path is not UTF-8")?.into(),
            payload,
        ))
    }
    fn call(&self, operation: Operation, path: &str, payload: Args) -> Result<serde_json::Value> {
        meden_runtime::file::run(
            self.workspace,
            self.control,
            &Self::request(operation, path, payload)?,
            &crate::workspace::file_policy()?,
            &Limits {
                timeout: Duration::from_secs(30),
                capture_bytes: 8_388_608,
                output_bytes: 8_388_608,
            },
        )
        .map_err(|error| anyhow::anyhow!("Lake workspace observation failed: {error}"))
    }
    fn batch(
        &self,
        operation: Operation,
        paths: &[&str],
        payload: &Args,
    ) -> Result<Vec<serde_json::Value>> {
        let _timing = meden_runtime::metrics::Span::start(match operation {
            Operation::List => "analysis.metadata.directory_batch",
            _ => "analysis.metadata.file_batch",
        });
        let requests = paths
            .iter()
            .map(|path| Self::request(operation, path, payload.clone()))
            .collect::<Result<Vec<_>>>()?;
        let values = meden_runtime::file::read_batch(
            self.workspace,
            self.control,
            &requests,
            &crate::workspace::file_policy()?,
            &Limits {
                timeout: Duration::from_secs(30),
                capture_bytes: 67_108_864,
                output_bytes: 67_108_864,
            },
        )
        .map_err(|error| anyhow::anyhow!("Lake workspace observation failed: {error}"))?;
        for value in &values {
            // Preserve the old per-response budget as well as the batch budget.
            anyhow::ensure!(
                serde_json::to_vec(&Ok::<_, ()>(value))?.len() <= 8_388_608,
                "Lake workspace response exceeds 8 MiB; no partial observation"
            );
        }
        Ok(values)
    }
    /// First unobserved directory whose parent was actually observed to contain
    /// it. Missing parents stop traversal; file/symlink entries are still checked
    /// by the ordinary resolved worker instead of trusting their listed type.
    fn next_directory<'p>(&self, path: &'p str) -> Result<Option<&'p str>> {
        if !self.directories.contains_key("") {
            return Ok(Some(""));
        }
        if path.is_empty() {
            return Ok(None);
        }
        let mut parent = "";
        for end in path
            .char_indices()
            .filter_map(|(i, c)| (c == '/').then_some(i))
            .chain(std::iter::once(path.len()))
        {
            let prefix = path.get(..end).context("Invalid directory boundary")?;
            let name = prefix
                .rsplit('/')
                .next()
                .context("Missing directory name")?;
            let entries = self
                .directories
                .get(parent)
                .context("Unobserved directory parent")?;
            if !entries.iter().any(|entry| entry.name == name) {
                return Ok(None);
            }
            if !self.directories.contains_key(prefix) {
                return Ok(Some(prefix));
            }
            parent = prefix;
        }
        Ok(None)
    }
    /// Validate the same original observations with bounded independent batches.
    /// This does not cache successful checks between operations or use timestamps.
    pub fn check(&mut self, expected: &Metadata) -> Result<()> {
        let _timing = meden_runtime::metrics::Span::start("analysis.metadata.validate");
        anyhow::ensure!(
            expected.files.len() <= 4096
                && expected
                    .files
                    .values()
                    .flatten()
                    .map(String::len)
                    .sum::<usize>()
                    <= 1_048_576,
            "Lake configuration observations exceed bounds"
        );
        anyhow::ensure!(
            expected.directories.len() <= 2048
                && expected
                    .directories
                    .values()
                    .flatten()
                    .map(Vec::len)
                    .sum::<usize>()
                    <= 16384,
            "Lake module inventory exceeds bounds"
        );
        for path in expected.files.keys().chain(expected.directories.keys()) {
            anyhow::ensure!(
                relative("", path)? == *path,
                "Noncanonical Lake metadata path"
            );
        }
        self.prefetch_directories(expected)?;
        self.check_files(expected)?;
        for (directory, entries) in &expected.directories {
            self.glob_entries(directory)?;
            anyhow::ensure!(
                self.metadata.directories.get(directory) == Some(entries),
                "Lake source inventory changed at {directory}; reread the graph"
            );
        }
        Ok(())
    }
    fn prefetch_directories(&mut self, expected: &Metadata) -> Result<()> {
        let needed = expected
            .files
            .keys()
            .map(|path| path.rsplit_once('/').map_or("", |(parent, _)| parent))
            .chain(expected.directories.keys().map(String::as_str))
            .collect::<BTreeSet<_>>();
        loop {
            let frontier = needed
                .iter()
                .map(|path| self.next_directory(path))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            if frontier.is_empty() {
                break;
            }
            anyhow::ensure!(
                self.directories.len().saturating_add(frontier.len()) <= 2048,
                "Lake directory observation budget exceeded"
            );
            // Four legacy 8 MiB responses fit completely inside the 64 MiB frame.
            for paths in frontier.chunks(4) {
                let payload = Args::from_iter([
                    ("entries".into(), true.into()),
                    ("max_entries".into(), 100_000.into()),
                ]);
                let values = self.batch(Operation::List, paths, &payload)?;
                let rows = paths
                    .iter()
                    .zip(values)
                    .map(|(path, value)| Ok(((*path).to_owned(), parse_entries(value)?)))
                    .collect::<Result<Vec<_>>>()?;
                self.directories.extend(rows);
            }
        }
        Ok(())
    }
    fn check_files(&mut self, expected: &Metadata) -> Result<()> {
        let mut present = Vec::new();
        for (path, value) in &expected.files {
            let exists = self.exists(path)?;
            anyhow::ensure!(exists == value.is_some(), "Lake configuration/import resolution changed at {path}; reread the graph and explicitly rerun");
            if exists {
                present.push(path.as_str());
            }
        }
        for paths in present.chunks(256) {
            let payload = Args::from_iter([
                ("max_bytes".into(), 65_536.into()),
                ("encoding".into(), "base64".into()),
            ]);
            let values = self.batch(Operation::Snapshot, paths, &payload)?;
            for (path, value) in paths.iter().zip(values) {
                let text = decode_snapshot(&value, 65_536)?;
                anyhow::ensure!(expected.files.get(*path) == Some(&Some(text)),
                    "Lake configuration/import resolution changed at {path}; reread the graph and explicitly rerun");
            }
        }
        Ok(())
    }
    pub fn entries(&mut self, directory: &str) -> Result<Vec<Entry>> {
        if let Some(entries) = self.directories.get(directory) {
            return Ok(entries.clone());
        }
        anyhow::ensure!(
            self.directories.len() < 2048,
            "Lake directory observation budget exceeded"
        );
        let payload = Args::from_iter([
            ("entries".into(), true.into()),
            ("max_entries".into(), 100_000.into()),
        ]);
        let entries = parse_entries(self.call(Operation::List, directory, payload)?)?;
        self.directories.insert(directory.into(), entries.clone());
        Ok(entries)
    }
    pub fn exists(&mut self, path: &str) -> Result<bool> {
        let (parent, name) = path.rsplit_once('/').unwrap_or(("", path));
        if !parent.is_empty() && !self.exists(parent)? {
            return Ok(false);
        }
        Ok(self.entries(parent)?.iter().any(|entry| entry.name == name))
    }
    pub fn glob_entries(&mut self, directory: &str) -> Result<Vec<Entry>> {
        if !self.exists(directory)? && !directory.is_empty() {
            self.metadata.directories.insert(directory.into(), None);
            return Ok(Vec::new());
        }
        let mut entries = self.entries(directory)?;
        entries.retain(|e| {
            e.kind != "file"
                || Path::new(&e.name)
                    .extension()
                    .is_some_and(|ext| ext == "lean")
        });
        self.metadata
            .directories
            .insert(directory.into(), Some(entries.clone()));
        anyhow::ensure!(
            self.metadata
                .directories
                .values()
                .flatten()
                .map(Vec::len)
                .sum::<usize>()
                <= 16384,
            "Lake module inventory exceeds bounds"
        );
        Ok(entries)
    }
    pub fn read(&mut self, path: &str, retain: bool) -> Result<Option<String>> {
        let path = relative("", path)?;
        let value = if self.exists(&path)? {
            let limit: usize = if retain { 65_536 } else { 1_048_576 };
            let payload = Args::from_iter([
                ("max_bytes".into(), limit.into()),
                ("encoding".into(), "base64".into()),
            ]);
            let value = self.call(Operation::Snapshot, &path, payload)?;
            Some(decode_snapshot(&value, limit)?)
        } else {
            None
        };
        if retain || value.is_none() {
            self.metadata.files.insert(path, value.clone());
        }
        anyhow::ensure!(
            self.metadata.files.len() <= 4096
                && self
                    .metadata
                    .files
                    .values()
                    .flatten()
                    .map(String::len)
                    .sum::<usize>()
                    <= 1_048_576,
            "Lake configuration observations exceed bounds"
        );
        Ok(value)
    }
}
fn parse_entries(value: serde_json::Value) -> Result<Vec<Entry>> {
    let entries: Vec<Entry> = serde_json::from_value(value)?;
    anyhow::ensure!(
        entries
            .iter()
            .all(|e| e.name != "." && e.name != ".." && !e.name.contains('/')),
        "Invalid directory entry"
    );
    Ok(entries)
}
fn decode_snapshot(value: &serde_json::Value, limit: usize) -> Result<String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(value.as_str().context("Invalid Lake input response")?)?;
    anyhow::ensure!(
        bytes.len() <= limit,
        "Lake input exceeds selected byte limit"
    );
    String::from_utf8(bytes).context("Lake input is not UTF-8")
}
/// Normalize declared relative paths before constructing the original worker request.
/// Parent components are allowed only when they stay within the host workspace.
pub fn relative(base: &str, child: &str) -> Result<String> {
    let mut components = Vec::new();
    let joined = if Path::new(child).is_absolute() {
        Path::new(child)
            .strip_prefix("/workspace")
            .context("Lake path leaves the selected workspace")?
            .to_path_buf()
    } else {
        Path::new(base).join(child)
    };
    for part in joined.components() {
        match part {
            Component::CurDir => {}
            Component::Normal(name) => {
                components.push(name.to_str().context("Lake path is not UTF-8")?.to_owned());
            }
            Component::ParentDir => {
                anyhow::ensure!(
                    components.pop().is_some(),
                    "Lake path leaves the selected workspace"
                );
            }
            Component::RootDir | Component::Prefix(_) => {
                anyhow::bail!("Lake path must be workspace-relative")
            }
        }
    }
    let path = components.join("/");
    anyhow::ensure!(path.len() <= 4096, "Lake path exceeds bounds");
    Ok(path)
}
