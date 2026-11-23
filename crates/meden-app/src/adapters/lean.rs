//! Lean-specific compilation adapter; no compiler syntax belongs to the core.
use super::Context;
use crate::{
    analysis::sources::Snapshot,
    compiler::{Compiler, Request},
};
use anyhow::{Context as _, Result};
pub struct Lean {
    atlas: crate::atlas::Configuration,
}
impl Lean {
    pub fn new(atlas: crate::atlas::Configuration) -> Self {
        Self { atlas }
    }
}

impl Compiler for Lean {
    fn compile(&self, context: &Context<'_>, request: Request) -> Result<Snapshot> {
        let paths = request.paths();
        let workspace = context.workspace;
        let control = context.control;
        let atlas = &self.atlas;
        atlas.ensure_prepared(control)?;
        if let Some(cached) = crate::explanation::cache::compiled(workspace, control, paths)? {
            return Ok(cached);
        }
        let mut snapshot =
            crate::analysis::collect::capture(workspace, control, paths, atlas.clone())?;
        let observed = snapshot.files.keys().cloned().collect::<Vec<_>>();
        let collection = snapshot
            .collection
            .as_ref()
            .context("Missing collected module bindings")?;
        if paths.is_empty() {
            if let Some(cached) =
                crate::explanation::cache::compiled(workspace, control, &collection.roots)?
            {
                return Ok(cached);
            }
        }
        crate::analysis::collect::check_metadata(workspace, control, &collection.metadata)?;
        let mut graph = atlas.extract(&snapshot.files, &collection.modules, control)?;
        graph.revision = format!(
            "sha256:{}",
            crate::atlas::digest(&serde_json::to_vec(&(&graph.revision, collection))?)
        );
        crate::analysis::collect::check_metadata(workspace, control, &collection.metadata)?;
        anyhow::ensure!(
            crate::analysis::sources::capture(workspace, control, &observed)?.revision
                == snapshot.revision,
            "Sources changed during compilation; no graph admitted"
        );
        snapshot.atlas = Some(graph);
        control.with_active(|| Ok(()))?;
        Ok(snapshot)
    }
}
