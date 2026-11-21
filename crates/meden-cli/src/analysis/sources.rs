//! Bounded source observations through the ordinary policy-bound offline file worker.
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use base64::Engine as _;
use meden_agent::tool::Args;
use meden_core::file::{Operation, Request};
use meden_runtime::{
    process::{Control, Limits},
    workspace::Workspace,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const FILE_LIMIT: usize = 1_048_576;
const TOTAL_LIMIT: usize = 16_777_216;

#[derive(Clone, Serialize)]
pub(crate) struct Snapshot {
    pub revision: String,
    pub files: BTreeMap<String, String>,
    pub explanations: BTreeMap<String, Vec<crate::explanation::source::Symbol>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atlas: Option<crate::atlas::Extraction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<super::collect::Collection>,
}

pub(crate) fn capture(
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
            Args::from_iter([
                ("encoding".into(), "base64".into()),
                ("max_bytes".into(), FILE_LIMIT.into()),
            ]),
        );
        let value = meden_runtime::file::run(
            workspace,
            control,
            &request,
            &policy,
            &Limits {
                timeout: Duration::from_secs(30),
                capture_bytes: 8_388_608,
                output_bytes: 8_388_608,
            },
        )
        .map_err(|error| anyhow::anyhow!("Source observation failed for {name}: {error}. Select an existing file from the workspace."))?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(value.as_str().context("Invalid source snapshot response")?)?;
        anyhow::ensure!(
            bytes.len() <= FILE_LIMIT,
            "Source file {name} exceeds 1 MiB; no truncated graph was created"
        );
        total = total
            .checked_add(bytes.len())
            .context("source budget overflow")?;
        anyhow::ensure!(total <= TOTAL_LIMIT, "Selected sources exceed 16 MiB");
        files.insert(
            name,
            String::from_utf8(bytes).context("source must be UTF-8")?,
        );
    }
    let revision = format!("sha256:{:x}", Sha256::digest(serde_json::to_vec(&files)?));
    let explanations = files
        .iter()
        .filter(|(path, _)| Path::new(path).extension().is_some_and(|ext| ext == "lean"))
        .map(|(path, text)| {
            (
                path.clone(),
                crate::explanation::source::symbols(text, true),
            )
        })
        .collect();
    Ok(Snapshot {
        revision,
        files,
        explanations,
        atlas: None,
        collection: None,
    })
}
