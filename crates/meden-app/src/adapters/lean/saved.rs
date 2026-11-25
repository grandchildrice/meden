//! Read-only complete saved views. No current-input or execution admission.
use super::{cache, Snapshot};
use crate::{
    explanation::cache::{Artifact, Identity, Observation, Store},
    view::sources::SavedSource,
};
use anyhow::{Context as _, Result};
use std::sync::Arc;

/// Read a complete original cache chain for viewing only. The ordinary pipeline
/// still checks all current metadata, dependency and compiled-module inputs.
pub fn load(
    context: &crate::adapters::Context<'_>,
    paths: &[String],
) -> Result<Option<SavedSource>> {
    let _timing = meden_runtime::metrics::Span::start("analysis.saved_view");
    let atlas = crate::atlas::Configuration::host(context.workspace.path());
    let (compilers, selected) = crate::adapters::compilers(atlas.clone())?;
    let compiler = compilers.select(selected)?;
    let prepared = crate::adapters::lake::enabled()?;
    let key = cache::scope(paths, prepared, compiler.definition())?;
    let store = Store::host(context.workspace)?;
    let Some(id) = store.index::<Artifact<Snapshot>>()?.get(&key).cloned() else {
        return Ok(None);
    };
    let artifact = store.read::<Artifact<Snapshot>>(&id)?;
    artifact.check()?;
    artifact.check_compiler(compiler.definition())?;
    let collection = artifact
        .snapshot
        .collection()
        .context("Missing saved input collection")?;
    anyhow::ensure!(
        cache::scope(
            &collection.roots,
            artifact.snapshot.prepared().is_some(),
            compiler.definition()
        )? == key,
        "Saved source selection changed"
    );
    let paths = artifact
        .snapshot
        .source
        .files
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    if crate::analysis::sources::capture(context.workspace, context.control, &paths)?.files
        != artifact.snapshot.source.files
    {
        return Ok(None);
    }
    let original = Observation {
        identity: Identity::Compilation(id),
        reused: true,
        snapshot: Arc::new(artifact.snapshot),
    };
    let (searchers, selected) = crate::adapters::searchers(atlas)?;
    let selected = searchers.select(selected)?;
    selected.validate()?;
    let Some(searched) = store.searched(&original, selected.definition())? else {
        return Ok(None);
    };
    let renderer = crate::adapters::informath::Prepared::discover()?;
    let Some(snapshot) = store.rendered(
        searched.compilation(),
        searched.snapshot(),
        renderer.definition(),
    )?
    else {
        return Ok(None);
    };
    renderer.validate()?;
    selected.validate()?;
    context.control.with_active(|| Ok(()))?;
    Ok(Some(SavedSource::new(&snapshot)?))
}
