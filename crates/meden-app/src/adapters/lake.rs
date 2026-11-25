//! Prepared Lake builds keep installed imports outside the selected-source graph.
use super::Context;
use crate::compiler::{Compiled, Compiler, Invocation};
use crate::identity::snapshots::GraphRevision;
use anyhow::{Context as _, Result};
use sha2::Digest as _;

pub struct Lake {
    atlas: crate::atlas::Configuration,
}
impl Lake {
    pub fn new(atlas: crate::atlas::Configuration) -> Self {
        Self { atlas }
    }
}
/// Trusted host choice: Lake preparation may execute project code with writable
/// workspace access. It never becomes a model-selected network executor.
pub fn enabled() -> Result<bool> {
    match std::env::var("MEDEN_ATLAS_BACKEND").ok().as_deref() {
        None | Some("frozen") => Ok(false),
        Some("lake") => Ok(true),
        Some(_) => anyhow::bail!("MEDEN_ATLAS_BACKEND must be frozen or lake"),
    }
}

impl Compiler for Lake {
    type Output = super::lean::Snapshot;
    fn descriptor(&self) -> Result<meden_runtime::plugin::Descriptor> {
        super::compiler_definition(
            "lean.lake",
            include_bytes!("lake.rs"),
            &self.atlas,
            &serde_json::json!({"imports":"prepared-boundaries","build_timeout_seconds":self.atlas.build_timeout()?.as_secs(),"extraction_timeout_seconds":600,"source_files":32,"source_bytes":16_777_216}),
        )
    }
    fn compile(
        &self,
        context: &Context<'_>,
        request: Invocation<super::lean::Snapshot>,
    ) -> Result<Compiled<super::lean::Snapshot>> {
        let atlas = &self.atlas;
        let mut snapshot = crate::analysis::collect::roots(
            context.workspace,
            context.control,
            request.paths(),
            atlas.clone(),
        )?;
        let collection = snapshot
            .collection
            .as_mut()
            .context("Missing Lake bindings")?;
        crate::analysis::collect::check_metadata(
            context.workspace,
            context.control,
            &collection.metadata,
        )?;
        let prepared = atlas.input_fingerprint(
            context.workspace.path(),
            &collection.modules,
            crate::atlas::InputMode::Prepare,
            context.control,
        )?;
        collection.prepared = Some(prepared.clone());
        if let Some(cached) = crate::adapters::lean::cache::compiled_prepared(
            context.workspace,
            context.control,
            &collection.roots,
            &prepared,
            &request,
        )? {
            return Ok(cached);
        }
        let modules = collection
            .modules
            .iter()
            .map(|m| m.name.replace('.', "/"))
            .collect::<Vec<_>>();
        let output = atlas.project(
            context.workspace.path(),
            "project-atlas-extract.sh",
            &modules,
            context.control,
        )?;
        let after = atlas.input_fingerprint(
            context.workspace.path(),
            &collection.modules,
            crate::atlas::InputMode::Fingerprint,
            context.control,
        )?;
        anyhow::ensure!(
            prepared == after,
            "Prepared Lake inputs changed during extraction"
        );
        crate::analysis::collect::check_metadata(
            context.workspace,
            context.control,
            &collection.metadata,
        )?;
        let paths = snapshot.source.files.keys().cloned().collect::<Vec<_>>();
        anyhow::ensure!(
            crate::analysis::sources::capture(context.workspace, context.control, &paths)?.files
                == snapshot.source.files,
            "Selected sources changed during prepared extraction"
        );
        let mut graph: serde_json::Value = serde_json::from_str(&output)
            .context("Prepared Atlas did not return a complete graph")?;
        bind_nodes(&mut graph, &collection.modules)?;
        let mut graph = atlas.admit_project(graph, &snapshot.source.files)?;
        graph.revision = GraphRevision::from_digest(sha2::Sha256::digest(serde_json::to_vec(&(
            &graph.revision,
            &collection,
        ))?));
        collection.external_modules = external_modules(
            atlas,
            &snapshot.source.files,
            &collection.modules,
            context.control,
        )?;
        snapshot.atlas = Some(graph);
        Ok(request.observe(snapshot))
    }
}

fn bind_nodes(
    graph: &mut serde_json::Value,
    bindings: &[crate::analysis::layout::Module],
) -> Result<()> {
    for node in graph
        .get_mut("nodes")
        .and_then(serde_json::Value::as_array_mut)
        .context("Missing prepared graph nodes")?
    {
        let logical = node
            .get("filePath")
            .and_then(serde_json::Value::as_str)
            .context("Missing graph source")?;
        let binding = bindings
            .iter()
            .find(|m| format!("{}.lean", m.name.replace('.', "/")) == logical)
            .context("Graph is outside selected Lake roots")?;
        node["filePath"] = binding.file.clone().into();
    }
    Ok(())
}

fn external_modules(
    atlas: &crate::atlas::Configuration,
    files: &std::collections::BTreeMap<String, String>,
    bindings: &[crate::analysis::layout::Module],
    control: &meden_runtime::process::Control,
) -> Result<Vec<String>> {
    Ok(atlas
        .imports(files, control)?
        .into_iter()
        .flat_map(|row| row.modules.into_iter().map(|m| m.name))
        .filter(|name| !bindings.iter().any(|m| m.name == *name))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect())
}
