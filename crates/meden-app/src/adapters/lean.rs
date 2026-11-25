//! Lean-specific compilation adapter; no compiler syntax belongs to the core.
pub mod cache;
pub mod search;
mod view;
use super::Context;
use crate::compiler::{Compiled, Compiler, Invocation};
use crate::identity::snapshots::GraphRevision;
use anyhow::{Context as _, Result};
use sha2::Digest as _;

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
pub struct Lean {
    atlas: crate::atlas::Configuration,
}
impl Lean {
    pub fn new(atlas: crate::atlas::Configuration) -> Self {
        Self { atlas }
    }
}

impl Compiler for Lean {
    type Output = Snapshot;
    fn descriptor(&self) -> Result<meden_runtime::plugin::Descriptor> {
        super::compiler_definition(
            "lean.frozen",
            include_bytes!("lean.rs"),
            &self.atlas,
            &serde_json::json!({"imports":"frozen-closure","source_files":32,"source_bytes":16_777_216}),
        )
    }
    fn compile(
        &self,
        context: &Context<'_>,
        request: Invocation<Snapshot>,
    ) -> Result<Compiled<Snapshot>> {
        let paths = request.paths();
        let workspace = context.workspace;
        let control = context.control;
        let atlas = &self.atlas;
        atlas.ensure_prepared(control)?;
        if let Some(cached) = cache::compiled(workspace, control, paths, &request)? {
            return Ok(cached);
        }
        let mut snapshot =
            crate::analysis::collect::capture(workspace, control, paths, atlas.clone())?;
        let observed = snapshot.source.files.keys().cloned().collect::<Vec<_>>();
        let collection = snapshot
            .collection
            .as_ref()
            .context("Missing collected module bindings")?;
        if paths.is_empty() {
            if let Some(cached) = cache::compiled(workspace, control, &collection.roots, &request)?
            {
                return Ok(cached);
            }
        }
        crate::analysis::collect::check_metadata(workspace, control, &collection.metadata)?;
        let mut graph = atlas.extract(&snapshot.source.files, &collection.modules, control)?;
        graph.revision = GraphRevision::from_digest(sha2::Sha256::digest(serde_json::to_vec(&(
            &graph.revision,
            collection,
        ))?));
        crate::analysis::collect::check_metadata(workspace, control, &collection.metadata)?;
        anyhow::ensure!(
            crate::analysis::sources::capture(workspace, control, &observed)?.revision
                == snapshot.source.revision,
            "Sources changed during compilation; no graph admitted"
        );
        snapshot.atlas = Some(graph);
        control.with_active(|| Ok(()))?;
        Ok(request.observe(snapshot))
    }
}
