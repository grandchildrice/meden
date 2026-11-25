//! Role-separated immutable finite observations. Reads validate complete current
//! source bytes; neither cache metadata nor a hit is verification authority.
use super::{
    compiled::{scope, Compiled, Compiler},
    render::Renderer,
    search::{Report, Searcher},
    snapshot::{Binding, Snapshot},
};
use crate::search::Observation as _;
use crate::{
    adapters::Context,
    compiler,
    explanation::cache::{
        self, artifact::Id, Artifact, Identity, Observation, Reading, Rendering, Search, Store,
    },
    meaning, search,
};
use anyhow::Result;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
};
pub(super) fn read(
    context: &Context<'_>,
    paths: &[String],
    worker: &Path,
) -> Result<(Snapshot, bool)> {
    let _span = meden_runtime::metrics::Span::start("analysis.finite.prepare");
    let store = Store::host(context.workspace)?;
    let mut registry = compiler::Registry::default();
    registry.register("finite.transition", Compiler)?;
    let completed = registry
        .select("finite.transition")?
        .compile(context, compiler::Request::new(paths.to_vec())?)?;
    let compile_hit = completed.reused();
    let (method, compiled) = completed.into_parts();
    let artifact = Artifact {
        version: 2,
        compiler: serde_json::to_value(&method)?,
        snapshot: compiled,
    };
    let compilation = if compile_hit {
        Id::of(&artifact)?
    } else {
        context.control.with_active(|| {
            store
                .write(
                    scope(&artifact.snapshot.path, &method).map_err(std::io::Error::other)?,
                    &artifact,
                )
                .map_err(std::io::Error::other)
        })?
    };
    let original = Observation {
        identity: Identity::Compilation(compilation.clone()),
        reused: compile_hit,
        snapshot: Arc::new(artifact.snapshot),
    };
    let mut searchers = search::Registry::default();
    searchers.register(Searcher {
        worker: worker.into(),
    })?;
    let selected = searchers.select("finite.z3")?;
    let (search_id, report, search_hit) = searched(context, &store, &original, &selected)?;
    selected.validate()?;
    let renderer = meaning::Bound::new(Renderer)?;
    let (render_id, readings, reading_hit) = rendered(context, &store, &original, &renderer)?;
    renderer.validate()?;
    let graph = crate::identity::snapshots::GraphRevision::parse(&format!(
        "sha256:{}",
        cache::hash(&(
            &compilation,
            &search_id,
            &render_id,
            super::snapshot::identity()
        ))?
    ))?;
    let snapshot = Snapshot {
        compiled: original.snapshot().clone(),
        report,
        readings: readings
            .into_iter()
            .map(|r| (r.node, r.sentence))
            .collect::<BTreeMap<_, _>>(),
        graph,
        binding: Binding::new(
            worker.into(),
            &method,
            selected.definition(),
            renderer.definition(),
        ),
    };
    snapshot.binding.check()?;
    let paths = snapshot
        .compiled
        .source
        .files
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let current = crate::analysis::sources::capture(context.workspace, context.control, &paths)?;
    anyhow::ensure!(
        current.revision == snapshot.compiled.source.revision,
        "Finite inputs changed while preparing observations; reread the source"
    );
    context.control.with_active(|| Ok(()))?;
    Ok((snapshot, compile_hit && search_hit && reading_hit))
}

fn searched(
    context: &Context<'_>,
    store: &Store,
    original: &Observation<Compiled>,
    selected: &search::Registration<'_, Compiled, Report>,
) -> Result<(Id<cache::artifact::CounterexampleSearch>, Report, bool)> {
    let compilation = original.compilation();
    let key = cache::hash(&(&compilation, selected.definition().identity()))?;
    let loaded = (|| -> Result<_> {
        let Some(id) = store.index::<Search<Report>>()?.get(&key).cloned() else {
            return Ok(None);
        };
        let search = store.read::<Search<Report>>(&id)?;
        anyhow::ensure!(
            &search.observation == compilation
                && search.searcher == serde_json::to_value(selected.definition())?,
            "Finite search cache binding changed"
        );
        search.report.validate(original.snapshot())?;
        Ok(Some((id, search.report)))
    })();
    let (search_id, report, search_hit) = if let Ok(Some((id, report))) = loaded {
        (id, report, true)
    } else {
        let report = selected
            .search(context, search::Request::new(original)?)?
            .into_report();
        let search = Search {
            observation: compilation.clone(),
            searcher: serde_json::to_value(selected.definition())?,
            report,
        };
        let id = context
            .control
            .with_active(|| store.write(key, &search).map_err(std::io::Error::other))?;
        (id, search.report, false)
    };

    Ok((search_id, report, search_hit))
}
fn rendered(
    context: &Context<'_>,
    store: &Store,
    original: &Observation<Compiled>,
    renderer: &meaning::Bound<Renderer>,
) -> Result<(Id<cache::artifact::MeaningRendering>, Vec<Reading>, bool)> {
    let compilation = original.compilation();
    let key = cache::hash(&(&compilation, renderer.definition().identity()))?;
    let declarations = original.snapshot().declarations()?;
    let loaded = (|| -> Result<_> {
        let Some(id) = store.index::<Rendering>()?.get(&key).cloned() else {
            return Ok(None);
        };
        let rendering = store.read::<Rendering>(&id)?;
        anyhow::ensure!(
            &rendering.observation == compilation
                && rendering.renderer == serde_json::to_value(renderer.definition())?
                && rendering.readings.len() == declarations.len()
                && rendering
                    .readings
                    .iter()
                    .map(|r| &r.node)
                    .collect::<BTreeSet<_>>()
                    == declarations.iter().map(|d| &d.id).collect(),
            "Finite reading cache binding changed"
        );
        anyhow::ensure!(
            rendering.readings.iter().all(|r| !r.sentence.is_empty()
                && r.sentence.len() <= 60_000
                && r.renderer.as_deref() == Some("finite.english")
                && r.error.is_none()),
            "Invalid finite reading cache"
        );
        Ok(Some((id, rendering.readings)))
    })();
    let (render_id, readings, reading_hit) = if let Ok(Some((id, readings))) = loaded {
        (id, readings, true)
    } else {
        let readings = renderer.batch(context.control, |batch| {
            declarations
                .iter()
                .map(|d| {
                    let reading = batch.render(original.snapshot().expression(d.term.clone()))?;
                    Ok(Reading {
                        node: d.id.clone(),
                        sentence: reading.sentence,
                        renderer: Some(reading.renderer),
                        error: None,
                    })
                })
                .collect::<Result<Vec<_>>>()
        })?;
        let rendering = Rendering {
            observation: compilation.clone(),
            renderer: serde_json::to_value(renderer.definition())?,
            readings,
        };
        let id = context
            .control
            .with_active(|| store.write(key, &rendering).map_err(std::io::Error::other))?;
        (id, rendering.readings, false)
    };

    Ok((render_id, readings, reading_hit))
}
