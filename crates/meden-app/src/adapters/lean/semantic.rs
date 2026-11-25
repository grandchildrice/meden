//! Lean-owned formal context and dependency freshness for candidate assessment.
//! Generated readings/checks are excluded from the blind translation context.
use super::Snapshot;
use crate::{analysis::Freshness, identity::snapshots::GraphRevision, semantic::Target};
use anyhow::{Context as _, Result};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Serialize)]
pub struct Context {
    graph_revision: GraphRevision,
    extractor: &'static str,
    upstream: String,
    lean_version: String,
    scope: &'static str,
    declarations: Vec<crate::atlas::Node>,
}
impl crate::semantic::Observation for Snapshot {
    type Context = Context;
    fn graph_revision(&self) -> Option<&GraphRevision> {
        self.atlas.as_ref().map(|graph| &graph.revision)
    }
    fn context(&self, target: &Target) -> Result<Option<Context>> {
        if let Some(graph) = &self.atlas {
            graph.graph.validate(&self.source.files)?;
        }
        self.atlas
            .as_ref()
            .map(|graph| context(graph, target))
            .transpose()
    }
}
fn context(graph: &crate::atlas::Extraction, target: &Target) -> Result<Context> {
    let root = graph
        .graph
        .nodes
        .iter()
        .find(|node| {
            node.id == target.name
                && node.file_path == target.file
                && node.line_start == Some(target.line)
        })
        .context("The selected Atlas declaration changed; reread the graph")?;
    let mut seen = BTreeSet::new();
    let mut pending = vec![root.id.as_str()];
    while let Some(name) = pending.pop() {
        if !seen.insert(name) {
            continue;
        }
        let node = graph
            .graph
            .nodes
            .iter()
            .find(|node| node.id == name)
            .context("Atlas dependency unavailable")?;
        for edge in graph.graph.edges.iter().filter(|edge| edge.source == name) {
            if node.kind == "theorem"
                && matches!(
                    edge.kind.as_str(),
                    "theorem_value_to_definition" | "theorem_value_to_theorem"
                )
            {
                continue;
            }
            pending.push(edge.target.as_str());
        }
    }
    let declarations = graph
        .graph
        .nodes
        .iter()
        .filter(|node| seen.contains(node.id.as_str()))
        .map(|node| {
            let mut formal = node.clone();
            formal.semantic = None;
            formal
        })
        .collect();
    Ok(Context {
        graph_revision: graph.revision.clone(),
        extractor: "Lean Atlas",
        upstream: graph.upstream.clone(),
        lean_version: graph.graph.metadata.lean_version.clone(),
        scope: "Selected observed modules; theorem proof-only dependencies pruned; external definitions are not expanded",
        declarations,
    })
}
impl crate::analysis::Observation for Snapshot {
    fn check(&self, context: &crate::adapters::Context<'_>) -> Result<Freshness> {
        check_binding(
            context,
            self.collection.as_ref(),
            self.atlas
                .as_ref()
                .map(|graph| graph.resources_sha256.as_str()),
        )
    }
}
pub(super) fn check_binding(
    context: &crate::adapters::Context<'_>,
    collection: Option<&crate::analysis::collect::Collection>,
    resources: Option<&str>,
) -> Result<Freshness> {
    let workspace = context.workspace;
    let control = context.control;
    if let Some(collection) = collection {
        if let Err(error) =
            crate::analysis::collect::check_metadata(workspace, control, &collection.metadata)
        {
            return Ok(Freshness::Stale(error));
        }
        if let Some(expected) = &collection.prepared {
            let observed = crate::atlas::Configuration::host(workspace.path()).input_fingerprint(
                workspace.path(),
                &collection.modules,
                crate::atlas::InputMode::Fingerprint,
                control,
            )?;
            if &observed != expected {
                return Ok(Freshness::Stale(anyhow::anyhow!(
                    "Prepared dependencies changed; reread the file before meaning analysis"
                )));
            }
        }
    }
    if let Some(resources) = resources {
        anyhow::ensure!(
            crate::atlas::Configuration::host(workspace.path()).current_resources()? == resources,
            "Atlas resources changed; reread the graph and run again"
        );
    }
    Ok(Freshness::Current)
}
#[cfg(test)]
mod oracle;
#[cfg(test)]
mod tests;
