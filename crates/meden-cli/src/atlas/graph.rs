//! Bounded Atlas data and source correspondence checks; no proof receipt.
use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct Extraction {
    pub revision: String,
    pub upstream: String,
    pub graph: Graph,
    pub resources_sha256: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{Input, Prepared, Target};
    use serde_json::json;

    #[test]
    fn scoped_context_prunes_only_known_proof_edges_and_requires_the_exact_graph() -> Result<()> {
        let files = BTreeMap::from([("M.lean".into(), "source\n".into())]);
        let node = |id: &str, kind: &str| json!({"id":id,"shortName":id,"kind":kind,"filePath":"M.lean","lineStart":1,"lineEnd":1,"hasSorry":false,"statement":"Nat","value":null,"axioms":[],"standardAxioms":[],"externalDependencies":[]});
        let extraction = Extraction::new(
            json!({"metadata":{"projectName":"scope fixture","leanVersion":"test"},"nodes":[node("T","theorem"),node("Proof","theorem"),node("New","definition")],"edges":[{"source":"T","target":"Proof","kind":"theorem_value_to_theorem"},{"source":"T","target":"New","kind":"future_theorem_value_to_definition"}],"externalReferences":[]}),
            &files,
            &super::super::Manifest {
                version: 1,
                engine_sha256: "fixture".into(),
                files: BTreeMap::new(),
            },
        )?;
        let target = Target {
            file: "M.lean".into(),
            line: 1,
            name: "T".into(),
        };
        let context = extraction.meaning(&target)?;
        let declarations = context
            .get("declarations")
            .and_then(Value::as_array)
            .context("scoped declarations")?;
        let names = declarations
            .iter()
            .filter_map(|node| node.get("id").and_then(Value::as_str))
            .collect::<BTreeSet<_>>();
        assert_eq!(names, BTreeSet::from(["T", "New"]));
        let mut input = Input {
            source_revision: format!(
                "sha256:{}",
                super::super::digest(&serde_json::to_vec(&files)?)
            ),
            graph_revision: Some("wrong graph".into()),
            target,
            requirement: "original requirement".into(),
        };
        assert!(Prepared::new(input.clone(), files.clone(), Some(&extraction)).is_err());
        input.graph_revision = Some(extraction.revision.clone());
        assert!(Prepared::new(input.clone(), files.clone(), Some(&extraction)).is_ok());
        input.target.name = "missing declaration".into();
        assert!(Prepared::new(input, files, Some(&extraction)).is_err());
        Ok(())
    }
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Graph {
    pub metadata: Metadata,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub external_references: Vec<String>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Metadata {
    pub project_name: String,
    pub lean_version: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Node {
    pub id: String,
    pub short_name: String,
    pub kind: String,
    pub file_path: String,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
    pub has_sorry: bool,
    pub statement: String,
    pub value: Option<String>,
    pub axioms: Vec<String>,
    pub standard_axioms: Vec<String>,
    pub external_dependencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic: Option<crate::explanation::Explanation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<NodeMeta>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NodeMeta {
    #[serde(default)]
    pub is_main_theorem: bool,
}
#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct Edge {
    pub source: String,
    pub target: String,
    pub kind: String,
}

impl Extraction {
    pub(super) fn new(
        value: Value,
        files: &BTreeMap<String, String>,
        resources: &super::Manifest,
    ) -> Result<Self> {
        let graph: Graph = serde_json::from_value(value)?;
        anyhow::ensure!(
            graph.nodes.len() <= 5000
                && graph.edges.len() <= 100_000
                && graph.external_references.len() <= 50_000,
            "Atlas graph exceeds bounds"
        );
        let mut names = BTreeSet::new();
        for node in &graph.nodes {
            anyhow::ensure!(
                names.insert(node.id.as_str()) && !node.id.is_empty() && node.id.len() <= 1000,
                "Atlas declaration identity invalid"
            );
            let source = files
                .get(&node.file_path)
                .context("Atlas declaration is outside observed sources")?;
            anyhow::ensure!(
                node.statement.len() <= 65_536
                    && node.value.as_ref().is_none_or(|v| v.len() <= 65_536),
                "Atlas expression exceeds bounds"
            );
            anyhow::ensure!(
                node.semantic
                    .as_ref()
                    .is_none_or(crate::explanation::Explanation::valid),
                "Invalid semantic observation"
            );
            if let Some(line) = node.line_start {
                anyhow::ensure!(
                    line > 0 && line <= source.lines().count(),
                    "Atlas source range is unavailable"
                );
            }
        }
        for edge in &graph.edges {
            anyhow::ensure!(
                names.contains(edge.source.as_str()) && names.contains(edge.target.as_str()),
                "Atlas dependency target is missing"
            );
        }
        let revision = format!(
            "sha256:{}",
            super::digest(&serde_json::to_vec(&(
                super::UPSTREAM,
                files,
                &graph,
                resources
            ))?)
        );
        Ok(Self {
            revision,
            upstream: super::UPSTREAM.into(),
            graph,
            resources_sha256: super::digest(&serde_json::to_vec(resources)?),
        })
    }

    pub fn meaning(&self, target: &crate::semantic::Target) -> Result<Value> {
        let root = self
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
            let node = self
                .graph
                .nodes
                .iter()
                .find(|node| node.id == name)
                .context("Atlas dependency unavailable")?;
            for edge in self.graph.edges.iter().filter(|edge| edge.source == name) {
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
        let declarations = self
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
            serde_json::json!({"graph_revision":self.revision,"extractor":"Lean Atlas","upstream":self.upstream,"lean_version":self.graph.metadata.lean_version,"scope":"Selected observed modules; theorem proof-only dependencies pruned; external definitions are not expanded","declarations":declarations}),
        )
    }
}
