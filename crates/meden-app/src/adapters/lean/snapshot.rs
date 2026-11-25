//! Lean-owned decoded data and raw compiler validation.
use anyhow::{Context as _, Result};

/// Lean's decoded compilation context. Generic source capture and the compiler
/// port do not own its module collection, elaborated graph or retained modules.
#[derive(Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "LeanSnapshot")]
pub struct Snapshot {
    #[serde(flatten)]
    pub source: crate::analysis::sources::Snapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atlas: Option<crate::atlas::Extraction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<crate::analysis::collect::Collection>,
}
impl Snapshot {
    /// Retain only the original source when no Lean compilation was requested.
    pub fn source_only(source: crate::analysis::sources::Snapshot) -> Self {
        Self {
            source,
            atlas: None,
            collection: None,
        }
    }
}
impl crate::analysis::sources::Source for Snapshot {
    fn sources(&self) -> &crate::analysis::sources::Snapshot {
        &self.source
    }
}
impl crate::compiler::Observation for Snapshot {
    fn validate(&self) -> Result<()> {
        let graph = self
            .atlas
            .as_ref()
            .context("Compiler did not produce a compiled observation")?;
        anyhow::ensure!(
            graph
                .graph
                .nodes
                .iter()
                .all(|n| n.semantic.as_ref().is_none_or(|s| s.checker.is_none())),
            "Compiler returned a search observation"
        );
        graph.graph.validate(&self.source.files)?;
        anyhow::ensure!(
            self.collection.as_ref().is_none_or(|collection| collection
                .roots
                .iter()
                .all(|p| self.source.files.contains_key(p))),
            "Source root is outside the retained inputs"
        );
        Ok(())
    }
}
