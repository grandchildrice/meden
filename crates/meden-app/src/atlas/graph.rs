//! Bounded Atlas data and source correspondence checks; no proof receipt.
use crate::identity::snapshots::GraphRevision;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Deserialize, Serialize, schemars::JsonSchema)]
pub struct Extraction {
    pub revision: GraphRevision,
    pub upstream: String,
    pub graph: Graph,
    pub resources_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compiled_modules: Option<super::Modules>,
}

#[derive(Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Graph {
    pub metadata: Metadata,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub external_references: Vec<String>,
}
#[derive(Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub project_name: String,
    pub lean_version: String,
}
#[derive(Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Node {
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
    pub semantic: Option<crate::explanation::Description>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_kind: Option<SearchKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<NodeMeta>,
}
#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SearchKind {
    Proposition,
    Predicate,
    Value,
}
#[derive(Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NodeMeta {
    #[serde(default)]
    pub is_main_theorem: bool,
}
#[derive(Clone, Deserialize, Serialize, schemars::JsonSchema)]
pub struct Edge {
    pub source: String,
    pub target: String,
    pub kind: String,
}

impl Graph {
    pub(crate) fn validate(&self, files: &BTreeMap<String, String>) -> Result<()> {
        anyhow::ensure!(
            self.nodes.len() <= 5000
                && self.edges.len() <= 100_000
                && self.external_references.len() <= 50_000,
            "Atlas graph exceeds bounds"
        );
        let mut names = BTreeSet::new();
        for node in &self.nodes {
            anyhow::ensure!(
                names.insert(node.id.as_str())
                    && !node.id.is_empty()
                    && node.id.len() <= 1000
                    && node.short_name.len() <= 1000
                    && node.axioms.len().saturating_add(node.standard_axioms.len()) <= 1000
                    && node
                        .axioms
                        .iter()
                        .chain(&node.standard_axioms)
                        .all(|a| a.len() <= 1000),
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
                    .is_none_or(crate::explanation::Description::valid),
                "Invalid semantic observation"
            );
            if let Some(line) = node.line_start {
                anyhow::ensure!(
                    line > 0 && line <= source.lines().count(),
                    "Atlas source range is unavailable"
                );
            }
        }
        for edge in &self.edges {
            anyhow::ensure!(
                names.contains(edge.source.as_str()) && names.contains(edge.target.as_str()),
                "Atlas dependency target is missing"
            );
        }
        Ok(())
    }
}

impl Extraction {
    pub(super) fn new(
        value: Value,
        files: &BTreeMap<String, String>,
        resources: &super::Manifest,
    ) -> Result<Self> {
        let graph: Graph = serde_json::from_value(value)?;
        graph.validate(files)?;
        let revision = GraphRevision::from_digest(Sha256::digest(serde_json::to_vec(&(
            super::UPSTREAM,
            files,
            &graph,
            super::compiler_resources(resources)?,
        ))?));
        Ok(Self {
            revision,
            upstream: super::UPSTREAM.into(),
            graph,
            resources_sha256: super::compiler_resources(resources)?,
            compiled_modules: None,
        })
    }
}
