//! Bounded source observations through the ordinary policy-bound offline file worker.
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use meden_agent::tool::Args;
use meden_core::file::{Operation, Request};
use meden_runtime::{
    process::{Control, Limits},
    workspace::Workspace,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const FILE_LIMIT: usize = 65_536;
const TOTAL_LIMIT: usize = 1_048_576;

pub(super) fn refresh(
    view: &std::sync::Mutex<super::View>,
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
) -> Result<()> {
    super::lock(view).source_snapshot = None;
    let snapshot = capture(workspace, control, paths)?;
    let mut view = super::lock(view);
    if view.analysis.as_ref().is_some_and(|record| {
        record.input.source_revision != snapshot.revision || record.input.graph_revision.is_some()
    }) {
        super::analysis::invalidate(&mut view);
    }
    view.source_snapshot = Some(snapshot);
    Ok(())
}

pub(super) fn extract_atlas(
    view: &std::sync::Mutex<super::View>,
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
) -> Result<()> {
    let atlas = crate::atlas::Configuration::host();
    {
        let mut view = super::lock(view);
        view.source_snapshot = None;
        super::analysis::invalidate(&mut view);
        view.activity = "LeanでコンパイルしてAtlas依存を抽出".into();
    }
    let mut snapshot = capture(workspace, control, paths)?;
    let extraction = atlas.extract(&snapshot.files, control);
    super::lock(view).atlas = atlas.status();
    let graph = extraction?;
    anyhow::ensure!(
        capture(workspace, control, paths)?.revision == snapshot.revision,
        "Sources changed during Atlas extraction; no graph admitted"
    );
    snapshot.atlas = Some(graph);
    control.with_active(|| {
        let mut view = super::lock(view);
        view.source_snapshot = Some(snapshot);
        view.atlas = atlas.status();
        Ok(())
    })?;
    Ok(())
}

#[derive(Clone, Serialize)]
pub(super) struct Snapshot {
    pub revision: String,
    pub files: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atlas: Option<crate::atlas::Extraction>,
}

pub(super) fn capture(
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
) -> Result<Snapshot> {
    anyhow::ensure!(
        !paths.is_empty() && paths.len() <= 32,
        "Select between 1 and 32 Rust/Lean source files"
    );
    let policy = crate::workspace::file_policy()?;
    let mut files = BTreeMap::new();
    let mut total = 0usize;
    for path in paths {
        let resolved =
            crate::workspace::logical_path(path).context("source path leaves /workspace")?;
        anyhow::ensure!(
            matches!(
                resolved.extension().and_then(|s| s.to_str()),
                Some("lean" | "rs")
            ),
            "Graphs currently support .lean and .rs source files"
        );
        let name = resolved
            .strip_prefix(Path::new("/workspace"))?
            .to_str()
            .context("source path is not UTF-8")?
            .to_owned();
        anyhow::ensure!(!files.contains_key(&name), "Duplicate source path: {name}");
        let request = Request::new(
            Operation::Snapshot,
            resolved.to_string_lossy().as_ref().into(),
            Args::new(),
        );
        let value = meden_runtime::file::run(
            workspace,
            control,
            &request,
            &policy,
            &Limits {
                timeout: Duration::from_secs(30),
                capture_bytes: 1_000_000,
                output_bytes: 1_000_000,
            },
        )
        .map_err(|error| anyhow::anyhow!("Source observation failed: {error}"))?;
        let bytes: Vec<u8> = serde_json::from_value(value)?;
        anyhow::ensure!(
            bytes.len() <= FILE_LIMIT,
            "Source file {name} exceeds 64 KiB; no truncated graph was created"
        );
        total = total
            .checked_add(bytes.len())
            .context("source budget overflow")?;
        anyhow::ensure!(total <= TOTAL_LIMIT, "Selected sources exceed 1 MiB");
        files.insert(
            name,
            String::from_utf8(bytes).context("source must be UTF-8")?,
        );
    }
    let revision = format!("sha256:{:x}", Sha256::digest(serde_json::to_vec(&files)?));
    Ok(Snapshot {
        revision,
        files,
        atlas: None,
    })
}
