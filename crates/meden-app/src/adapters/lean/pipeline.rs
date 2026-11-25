//! Native Lean source/compilation/search/reading composition; no approval or evidence admission.
use anyhow::Result;
use meden_runtime::{process::Control, workspace::Workspace};

pub fn extract_cached(
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
    observer: Option<crate::adapters::Observer>,
) -> Result<(crate::adapters::lean::Snapshot, bool)> {
    let _timing = meden_runtime::metrics::Span::start("analysis.extract");
    let context = crate::adapters::Context {
        workspace,
        control,
        observer,
    };
    let atlas = crate::atlas::Configuration::host(context.workspace.path())
        .with_observer(context.observer.clone());
    atlas.ensure_prepared(control)?;
    let (registry, selected) = crate::adapters::compilers(atlas)?;
    let snapshot = registry
        .select(selected)?
        .compile(&context, crate::compiler::Request::new(paths.to_vec())?)?;
    let original = control.with_active(|| {
        crate::adapters::lean::cache::retain_observation(workspace, snapshot)
            .map_err(std::io::Error::other)
    })?;
    let (searchers, selected) = crate::adapters::searchers(
        crate::atlas::Configuration::host(workspace.path()).with_observer(context.observer.clone()),
    )?;
    let original =
        crate::adapters::lean::cache::search(&context, original, &searchers.select(selected)?)?;
    if let Some(observer) = &context.observer {
        observer("Preparing explanations", std::time::Duration::ZERO, None);
    }
    let renderer = crate::adapters::informath::Prepared::discover()?;
    if let Some(snapshot) =
        crate::adapters::lean::cache::rendering(workspace, &original, renderer.definition())?
    {
        renderer.validate()?;
        return Ok((control.with_active(|| Ok(snapshot))?, original.reused()));
    }
    let mut snapshot = original.snapshot().clone();
    renderer.enrich(snapshot.graph_mut(), control)?;
    control.with_active(|| {
        crate::adapters::lean::cache::retain_rendering(
            workspace,
            original,
            renderer.definition(),
            &mut snapshot,
        )
        .map_err(std::io::Error::other)
    })?;
    Ok((snapshot, false))
}
