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
use std::time::{Duration, Instant};

/// The ordinary confined file worker with its original fixed options/budgets.
/// Callers cannot replace the operation, payload or host-held workspace policy.
pub struct Reader<'a> {
    workspace: &'a Workspace,
    control: &'a Control,
    started: Instant,
}
impl<'a> Reader<'a> {
    pub fn new(workspace: &'a Workspace, control: &'a Control, started: Instant) -> Self {
        Self {
            workspace,
            control,
            started,
        }
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
                capture_bytes: 67_108_864,
                output_bytes: 67_108_864,
            },
        )
        .map_err(anyhow::Error::new)
    }
}
impl collection::Reader for Reader<'_> {
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
        self.observe(Operation::Snapshot, paths, &json!({"observation":true}))?
            .into_iter()
            .map(|value| serde_json::from_value(value).map_err(Into::into))
            .collect()
    }
}
