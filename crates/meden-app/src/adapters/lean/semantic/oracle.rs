//! Test-only retired JSON projection, independent of the typed context serializer.
use crate::atlas::Extraction;
use anyhow::{Context as _, Result};
use serde_json::Value;
use std::collections::BTreeSet;
pub(super) fn original(graph: &Extraction, target: &crate::semantic::Target) -> Result<Value> {
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
            let mut value = serde_json::to_value(node)?;
            // The separate blind language comparison receives only formal
            // expressions, never our generated prose or sample observations.
            if let Some(fields) = value.as_object_mut() {
                fields.remove("semantic");
            }
            Ok(value)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(
        serde_json::json!({"graph_revision":graph.revision,"extractor":"Lean Atlas","upstream":graph.upstream,"lean_version":graph.graph.metadata.lean_version,"scope":"Selected observed modules; theorem proof-only dependencies pruned; external definitions are not expanded","declarations":declarations}),
    )
}
