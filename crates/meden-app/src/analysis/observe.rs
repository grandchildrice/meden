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
    collections::BTreeMap,
    path::{Component, Path},
    time::Duration,
};

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub name: String,
    pub kind: String,
}

#[derive(Clone, Default, Deserialize, Serialize)]
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
    fn call(&self, operation: Operation, path: &str, payload: Args) -> Result<serde_json::Value> {
        let path = crate::workspace::logical_path(path)
            .context("Lake path leaves the selected workspace")?;
        let request = Request::new(
            operation,
            path.to_str().context("Lake path is not UTF-8")?.into(),
            payload,
        );
        meden_runtime::file::run(
            self.workspace,
            self.control,
            &request,
            &crate::workspace::file_policy()?,
            &Limits {
                timeout: Duration::from_secs(30),
                capture_bytes: 8_388_608,
                output_bytes: 8_388_608,
            },
        )
        .map_err(|error| anyhow::anyhow!("Lake workspace observation failed: {error}"))
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
        let entries: Vec<Entry> =
            serde_json::from_value(self.call(Operation::List, directory, payload)?)?;
        anyhow::ensure!(
            entries
                .iter()
                .all(|e| e.name != "." && e.name != ".." && !e.name.contains('/')),
            "Invalid directory entry"
        );
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
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(value.as_str().context("Invalid Lake input response")?)?;
            anyhow::ensure!(
                bytes.len() <= limit,
                "Lake input exceeds selected byte limit"
            );
            Some(String::from_utf8(bytes).context("Lake input is not UTF-8")?)
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
