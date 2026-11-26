//! Concrete read-only observations for navigation, outside consumers and ports.
use crate::navigation::collection;
use crate::view::{directory::Entry, file::Content};
use anyhow::{Context, Result};
use meden_agent::tool::Args;
use meden_core::file::{Operation, Request};
use meden_runtime::{
    process::{Control, Limits},
    workspace::Workspace,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

const FRAME_BYTES: usize = 67_108_864;

/// The ordinary confined file worker with its original fixed options/budgets.
/// Callers cannot replace the operation, payload or host-held workspace policy.
pub struct Reader<'a> {
    workspace: &'a Workspace,
    control: &'a Control,
    started: Instant,
    hints: Option<BTreeMap<String, u64>>,
}
impl<'a> Reader<'a> {
    pub fn new(workspace: &'a Workspace, control: &'a Control, started: Instant) -> Self {
        Self {
            workspace,
            control,
            started,
            hints: None,
        }
    }
    /// Original disposable sizes may group reads; actual content and frame limits
    /// remain independently checked. Unknown sizes use the maximum file budget.
    pub fn with_hints(mut self, paths: &[String], hints: &[&Value]) -> Result<Self> {
        anyhow::ensure!(paths.len() == hints.len(), "Incomplete source size hints");
        self.hints = Some(
            paths
                .iter()
                .zip(hints)
                .filter_map(|(path, hint)| {
                    hint.get("bytes")
                        .and_then(Value::as_u64)
                        .map(|bytes| (path.clone(), bytes))
                })
                .collect(),
        );
        Ok(self)
    }
    /// Disposable metadata hints retain their original complete JSON spelling.
    /// They are not content validity, prepared inputs or proof evidence.
    pub fn stamps(&self, paths: &[String]) -> Result<Vec<Value>> {
        self.observe(Operation::Snapshot, paths, &json!({"metadata":true}))
    }
    fn observe(
        &self,
        operation: Operation,
        paths: &[String],
        payload: &Value,
    ) -> Result<Vec<Value>> {
        anyhow::ensure!(
            self.started.elapsed() < Duration::from_secs(1800),
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
            self.workspace,
            self.control,
            &requests,
            &crate::workspace::file_policy()?,
            &Limits {
                timeout: Duration::from_secs(30),
                capture_bytes: FRAME_BYTES,
                output_bytes: u64::try_from(FRAME_BYTES)?,
            },
        )
        .map_err(anyhow::Error::new)
    }
}
impl collection::Reader for Reader<'_> {
    fn source_batch(&self) -> usize {
        if self.hints.is_some() {
            64
        } else {
            32
        }
    }
    fn list(&self, paths: &[String]) -> Result<Vec<Vec<Entry>>> {
        self.observe(
            Operation::List,
            paths,
            &json!({"entries":true,"max_entries":100_000}),
        )?
        .into_iter()
        .map(|value| serde_json::from_value(value).map_err(Into::into))
        .collect()
    }
    fn read(&self, paths: &[String]) -> Result<Vec<Content>> {
        let chunks = if let Some(hints) = &self.hints {
            anyhow::ensure!(
                (1..=256).contains(&paths.len()),
                "Invalid source frame path count"
            );
            let chunks = source_chunks(paths, hints)?;
            if chunks.len() > 1 {
                // A single frame prepares all its paths in observe. Split frames
                // must also reject a later invalid path before the first dispatch.
                for path in paths {
                    crate::workspace::logical_path(path)
                        .context("Workspace index path leaves the selected root")?;
                }
            }
            chunks
        } else {
            vec![paths]
        };
        let mut contents = Vec::new();
        for chunk in chunks {
            for value in self.observe(Operation::Snapshot, chunk, &json!({"observation":true}))? {
                contents.push(serde_json::from_value(value)?);
            }
        }
        Ok(contents)
    }
}

fn source_chunks<'a>(
    paths: &'a [String],
    hints: &BTreeMap<String, u64>,
) -> Result<Vec<&'a [String]>> {
    let mut chunks = Vec::new();
    let mut start = 0usize;
    let mut used = 1024usize;
    for (index, path) in paths.iter().enumerate() {
        let bytes = hints
            .get(path)
            .copied()
            .unwrap_or(u64::MAX)
            .min(u64::try_from(crate::view::file::MAX_BYTES)?);
        // Six bytes per input byte covers JSON control escapes; the fixed
        // allowance covers every content variant's fields and separators.
        let cost = usize::try_from(bytes)?
            .saturating_mul(6)
            .saturating_add(1024);
        if index > start
            && (index.saturating_sub(start) >= 64 || used.saturating_add(cost) > FRAME_BYTES)
        {
            chunks.push(
                paths
                    .get(start..index)
                    .context("Invalid source frame range")?,
            );
            start = index;
            used = 1024;
        }
        used = used.saturating_add(cost);
    }
    if start < paths.len() {
        chunks.push(paths.get(start..).context("Invalid final source frame")?);
    }
    Ok(chunks)
}
