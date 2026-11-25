//! Frozen Lean dependency bindings and named-target relocation for shared repairs.
use super::Snapshot;
use crate::analysis::{self, repair};
use crate::semantic::Input;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    method: String,
    resources: String,
    prepared: Option<crate::identity::snapshots::PreparedRevision>,
    collection: analysis::collect::Collection,
}
fn method() -> String {
    format!(
        "{:x}",
        Sha256::digest(
            [
                include_bytes!("repair.rs").as_slice(),
                include_bytes!("semantic.rs").as_slice(),
                include_bytes!("snapshot.rs").as_slice(),
                include_bytes!("../../analysis/repair.rs").as_slice(),
            ]
            .concat()
        )
    )
}
impl repair::Context for Context {
    fn check(&self, context: &crate::adapters::Context<'_>) -> Result<()> {
        anyhow::ensure!(
            self.method == method(),
            "Originally selected Lean repair method changed"
        );
        match super::semantic::check_binding(
            context,
            &self.collection,
            self.prepared.as_ref(),
            &self.resources,
        )? {
            analysis::Freshness::Current => Ok(()),
            analysis::Freshness::Stale(error) => Err(error),
        }
    }
    fn paths(&self) -> Option<&[String]> {
        Some(&self.collection.roots)
    }
    fn relocatable(&self) -> bool {
        true
    }
}
impl repair::Observation for Snapshot {
    type Repair = Context;
    fn repair_context(&self) -> Result<Context> {
        {
            let collection = self.collection();
            anyhow::ensure!(
                !collection.roots.is_empty()
                    && collection
                        .roots
                        .iter()
                        .all(|p| self.source.files.contains_key(p)),
                "Repair roots are outside original sources"
            );
        }
        Ok(Context {
            method: method(),
            resources: self.graph().resources_sha256.clone(),
            collection: self.collection().clone(),
            prepared: self.prepared().cloned(),
        })
    }
    fn refreshed_input(&self, original: &Input) -> Result<Input> {
        let mut input = original.clone();
        input.source_revision.clone_from(&self.source.revision);
        input.graph_revision = Some(self.graph().revision.clone());
        {
            let graph = self.graph();
            let mut nodes =
                graph.graph.nodes.iter().filter(|node| {
                    node.id == input.target.name && node.file_path == input.target.file
                });
            let node = nodes
                .next()
                .context("Repair applied; declaration is missing from the new graph")?;
            input.target.line = node
                .line_start
                .context("Repair applied; new declaration has no source line")?;
            anyhow::ensure!(
                nodes.next().is_none(),
                "Repair applied; new declaration identity is ambiguous"
            );
        }
        Ok(input)
    }
}
