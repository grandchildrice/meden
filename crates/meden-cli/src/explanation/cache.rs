//! Source- and resource-bound observations retained outside the model workspace.
use super::Explanation;
use anyhow::{Context, Result};
use meden_runtime::{process::Control, workspace::Workspace};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Note {
    pub file: String,
    pub name: String,
    pub line: usize,
    pub semantic: Explanation,
    pub main_theorem: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    version: u8,
    resources: String,
    source_revision: String,
    paths: Vec<String>,
    metadata: crate::analysis::observe::Metadata,
    notes: Vec<Note>,
}

pub(crate) fn retain(
    workspace: &Workspace,
    snapshot: &crate::analysis::sources::Snapshot,
) -> Result<()> {
    let graph = snapshot
        .atlas
        .as_ref()
        .context("Missing compiled observations")?;
    let artifact = Artifact {
        version: 1,
        resources: graph.resources_sha256.clone(),
        source_revision: snapshot.revision.clone(),
        paths: snapshot.files.keys().cloned().collect(),
        metadata: snapshot
            .collection
            .as_ref()
            .context("Missing compilation bindings")?
            .metadata
            .clone(),
        notes: graph
            .graph
            .nodes
            .iter()
            .filter_map(|node| {
                Some(Note {
                    file: node.file_path.clone(),
                    name: node.id.clone(),
                    line: node.line_start?,
                    semantic: node.semantic.clone()?,
                    main_theorem: node.meta.as_ref().is_some_and(|meta| meta.is_main_theorem),
                })
            })
            .collect(),
    };
    let directory = crate::storage::directory(workspace.path())?.join("explanations");
    let id = crate::atlas::digest(&serde_json::to_vec(&artifact)?);
    crate::storage::write(&directory.join(format!("{id}.json")), &artifact)?;
    let index_path = directory.join("index.json");
    let mut index: BTreeMap<String, String> =
        crate::storage::read(&index_path)?.unwrap_or_default();
    for path in &artifact.paths {
        index.insert(path.clone(), id.clone());
    }
    anyhow::ensure!(
        index.len() <= 10_000,
        "Compiled explanation index exceeds 10000 files"
    );
    crate::storage::write(&index_path, &index)
}

pub(crate) fn load(
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
) -> Result<Vec<Note>> {
    let directory = crate::storage::directory(workspace.path())?.join("explanations");
    let index: BTreeMap<String, String> =
        crate::storage::read(&directory.join("index.json"))?.unwrap_or_default();
    anyhow::ensure!(
        index.len() <= 10_000,
        "Compiled explanation index exceeds bounds"
    );
    let ids = paths
        .iter()
        .filter_map(|path| index.get(path))
        .collect::<BTreeSet<_>>();
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let resources = crate::atlas::Configuration::host(workspace.path()).current_resources()?;
    let mut notes = Vec::new();
    for id in ids {
        anyhow::ensure!(
            id.len() == 64 && id.bytes().all(|c| c.is_ascii_hexdigit()),
            "Invalid explanation artifact ID"
        );
        let artifact: Artifact = crate::storage::read(&directory.join(format!("{id}.json")))?
            .context("Missing explanation artifact")?;
        anyhow::ensure!(
            artifact.version == 1
                && artifact.paths.len() <= 32
                && artifact.notes.len() <= 5000
                && crate::atlas::digest(&serde_json::to_vec(&artifact)?) == *id,
            "Invalid explanation artifact binding"
        );
        if artifact.resources != resources {
            continue;
        }
        // Stale observations never supersede a current source preview.
        if crate::analysis::collect::check_metadata(workspace, control, &artifact.metadata).is_err()
        {
            continue;
        }
        let Ok(current) = crate::analysis::sources::capture(workspace, control, &artifact.paths)
        else {
            continue;
        };
        if current.revision != artifact.source_revision {
            continue;
        }
        for note in artifact.notes {
            anyhow::ensure!(
                artifact.paths.contains(&note.file)
                    && note.line > 0
                    && note.semantic.origin == "elaborated"
                    && note.semantic.valid(),
                "Invalid explanation observation"
            );
            if paths.contains(&note.file) {
                notes.push(note);
            }
        }
    }
    Ok(notes)
}
