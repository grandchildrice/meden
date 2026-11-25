//! Native frozen-module compiler implementation.
use super::{cache, Snapshot};
use crate::adapters::Context;
use crate::compiler::{Compiled, Compiler, Invocation};
use crate::identity::snapshots::GraphRevision;
use anyhow::Result;
use sha2::Digest as _;

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
        super::definition::compiler(
            "lean.frozen",
            include_bytes!("compiler.rs"),
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
        let snapshot = crate::analysis::collect::capture(workspace, control, paths, atlas.clone())?;
        let observed = snapshot.source.files.keys().cloned().collect::<Vec<_>>();
        let collection = &snapshot.collection;
        if paths.is_empty() {
            if let Some(cached) = cache::compiled(workspace, control, &collection.roots, &request)?
            {
                return Ok(cached);
            }
        }
        crate::analysis::collect::check_metadata(workspace, control, &collection.metadata)?;
        let (mut graph, modules) =
            atlas.extract(&snapshot.source.files, &collection.modules, control)?;
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
        control.with_active(|| Ok(()))?;
        Ok(request.observe(Snapshot::compiled(
            snapshot.source,
            graph,
            snapshot.collection,
            super::snapshot::Inputs::Frozen { modules },
        )))
    }
}
