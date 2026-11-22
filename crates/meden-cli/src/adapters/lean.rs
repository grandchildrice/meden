//! Lean-specific compilation adapter; no compiler syntax belongs to the core.
use super::Context;
use anyhow::{Context as _, Result};
use meden_core::adapter::{Adapter, Artifact, Input};
pub(crate) struct Lean;

impl Adapter<Context<'_>> for Lean {
    type Error = anyhow::Error;
    fn run(&self, context: &Context<'_>, input: Input) -> Result<Artifact> {
        anyhow::ensure!(
            input.format == "meden/source-paths+json",
            "Unsupported Lean adapter input"
        );
        let paths: Vec<String> = serde_json::from_slice(&input.bytes)?;
        let workspace = context.workspace;
        let control = context.control;
        let atlas = crate::atlas::Configuration::host(context.workspace.path())
            .with_observer(context.observer.clone());
        atlas.ensure_prepared(control)?;
        let mut snapshot =
            crate::analysis::collect::capture(workspace, control, &paths, atlas.clone())?;
        let observed = snapshot.files.keys().cloned().collect::<Vec<_>>();
        let collection = snapshot
            .collection
            .as_ref()
            .context("Missing collected module bindings")?;
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
        Ok(Artifact {
            producer: "lean-atlas+plausible".into(),
            format: "meden/compiled-snapshot+json".into(),
            bytes: serde_json::to_vec(&snapshot)?,
        })
    }
}
