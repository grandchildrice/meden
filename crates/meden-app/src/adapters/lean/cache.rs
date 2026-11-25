//! Independent immutable compiler/search observations and renderer projections.
//! Neither layer carries accepted-contract evidence or restores an approval.
use crate::adapters::lean::Snapshot;
use crate::explanation::cache::artifact::{Compilation, Id};
use crate::explanation::cache::{hash, Identity, Reading, Rendering, Store};
use crate::explanation::Explanation;
use crate::identity::snapshots::{GraphRevision, PreparedRevision, SourceRevision};
use anyhow::{Context, Result};
use meden_runtime::{process::Control, workspace::Workspace};
use serde::{Deserialize, Serialize};
use sha2::Digest as _;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
type Observation = crate::explanation::cache::Observation<Snapshot>;
type Artifact = crate::explanation::cache::Artifact<Snapshot>;
type Search = crate::explanation::cache::Search<super::search::Report>;

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Note {
    pub file: String,
    pub name: String,
    pub line: usize,
    pub semantic: Explanation,
    pub main_theorem: bool,
}

impl Store {
    pub(super) fn searched(
        &self,
        original: &Observation,
        definition: &meden_runtime::plugin::Descriptor,
    ) -> Result<Option<Observation>> {
        let compilation = original.unsearched()?;
        let Some(id) = self
            .index::<Search>()?
            .get(&hash(&(compilation, definition.identity()))?)
            .cloned()
        else {
            return Ok(None);
        };
        let search = self.read::<Search>(&id)?;
        anyhow::ensure!(
            &search.observation == compilation
                && search.searcher == serde_json::to_value(definition)?,
            "Search input binding changed"
        );
        let mut snapshot = search.report.project(original.snapshot().clone())?;
        bind_revision(&mut snapshot, &id)?;
        Ok(Some(Observation {
            reused: original.reused,
            identity: Identity::Searched {
                compilation: compilation.clone(),
                search: id,
            },
            snapshot: Arc::new(snapshot),
        }))
    }
    fn retain_search(
        &self,
        original: Observation,
        definition: &meden_runtime::plugin::Descriptor,
        report: super::search::Report,
    ) -> Result<Observation> {
        let compilation = original.unsearched()?.clone();
        anyhow::ensure!(
            definition.role() == meden_runtime::plugin::Role::CounterexampleSearch,
            "Invalid search identity"
        );
        let mut snapshot = report.project(Arc::unwrap_or_clone(original.snapshot))?;
        let search = Search {
            observation: compilation.clone(),
            searcher: serde_json::to_value(definition)?,
            report,
        };
        let key = hash(&(&search.observation, definition.identity()))?;
        let id = self.write(key, &search)?;
        bind_revision(&mut snapshot, &id)?;
        Ok(Observation {
            reused: false,
            identity: Identity::Searched {
                compilation,
                search: id,
            },
            snapshot: Arc::new(snapshot),
        })
    }
}
pub fn search(
    context: &crate::adapters::Context<'_>,
    original: Observation,
    selected: &crate::search::Registration<'_, Snapshot, super::search::Report>,
) -> Result<Observation> {
    let _timing = meden_runtime::metrics::Span::start("analysis.search");
    let store = Store::host(context.workspace)?;
    selected.validate()?;
    if let Some(hit) = store.searched(&original, selected.definition())? {
        context.control.with_active(|| Ok(()))?;
        selected.validate()?;
        return Ok(hit);
    }
    let report = selected
        .search(context, crate::search::Request::new(&original)?)?
        .into_report();
    selected.validate()?;
    context
        .control
        .with_active(|| {
            store
                .retain_search(original, selected.definition(), report)
                .map_err(std::io::Error::other)
        })
        .map_err(Into::into)
}

impl Store {
    fn observe(
        &self,
        snapshot: Snapshot,
        definition: &meden_runtime::plugin::Descriptor,
    ) -> Result<Observation> {
        let _timing = meden_runtime::metrics::Span::start("analysis.cache.retain_observation");
        let artifact = Artifact {
            version: 2,
            compiler: serde_json::to_value(definition)?,
            snapshot,
        };
        artifact.check()?;
        artifact.check_compiler(definition)?;
        let collection = artifact.snapshot.collection();
        let key = scope(
            &collection.roots,
            artifact.snapshot.prepared().is_some(),
            definition,
        )?;
        let id = Id::of(&artifact)?;
        if self.index::<Artifact>()?.get(&key) == Some(&id) {
            self.read::<Artifact>(&id)?.check()?;
        } else {
            self.write(key, &artifact)?;
        }
        Ok(Observation {
            reused: false,
            identity: Identity::Compilation(id),
            snapshot: Arc::new(artifact.snapshot),
        })
    }
    pub(super) fn rendered(
        &self,
        id: &Id<Compilation>,
        snapshot: &Snapshot,
        renderer: &meden_runtime::plugin::Descriptor,
    ) -> Result<Option<Snapshot>> {
        let _timing = meden_runtime::metrics::Span::start("analysis.cache.load_rendering");
        let Some(rendering_id) = self
            .index::<Rendering>()?
            .get(&hash(&(id, renderer.identity()))?)
            .cloned()
        else {
            return Ok(None);
        };
        let rendering = self.read::<Rendering>(&rendering_id)?;
        anyhow::ensure!(
            &rendering.observation == id && rendering.renderer == serde_json::to_value(renderer)?,
            "Rendering input binding changed"
        );
        let mut result = rendering.project(snapshot.clone())?;
        bind_revision(&mut result, &rendering_id)?;
        Ok(Some(result))
    }
    fn render(
        &self,
        original: Observation,
        renderer: &meden_runtime::plugin::Descriptor,
        snapshot: &mut Snapshot,
    ) -> Result<()> {
        crate::view::SourcePage::from_snapshot(original.snapshot())?;
        let _timing = meden_runtime::metrics::Span::start("analysis.cache.store_rendering");
        anyhow::ensure!(
            renderer.role() == meden_runtime::plugin::Role::MeaningRenderer,
            "Invalid renderer identity"
        );
        let graph = snapshot.graph();
        let rendering = Rendering {
            observation: original.compilation().clone(),
            renderer: serde_json::to_value(renderer)?,
            readings: graph
                .graph
                .nodes
                .iter()
                .filter_map(|node| {
                    node.semantic.as_ref().map(|semantic| Reading {
                        node: node.id.clone(),
                        sentence: semantic.sentence.clone(),
                        renderer: semantic.renderer.clone(),
                        error: semantic.rendering_error.clone(),
                    })
                })
                .collect(),
        };
        let mut expected = rendering.project(Arc::unwrap_or_clone(original.snapshot))?;
        anyhow::ensure!(
            serde_json::to_vec(&expected)? == serde_json::to_vec(snapshot)?,
            "Rendering changed a compiler/search observation"
        );
        let key = hash(&(&rendering.observation, renderer.identity()))?;
        let id = self.write(key, &rendering)?;
        bind_revision(&mut expected, &id)?;
        *snapshot = expected;
        Ok(())
    }
}

impl Artifact {
    pub(super) fn check_compiler(
        &self,
        compiler: &meden_runtime::plugin::Descriptor,
    ) -> Result<()> {
        anyhow::ensure!(
            compiler.role() == meden_runtime::plugin::Role::Compiler
                && self.compiler == serde_json::to_value(compiler)?,
            "Compiler observation registration changed"
        );
        Ok(())
    }
    pub(super) fn check(&self) -> Result<()> {
        let snapshot = &self.snapshot;
        let collection = snapshot.collection();
        let graph = snapshot.graph();
        anyhow::ensure!(
            self.version == 2
                && !snapshot.source.files.is_empty()
                && snapshot.source.files.len() <= 32
                && snapshot.source.revision
                    == SourceRevision::from_digest(sha2::Sha256::digest(serde_json::to_vec(
                        &snapshot.source.files
                    )?))
                && !collection.roots.is_empty()
                && collection
                    .roots
                    .iter()
                    .all(|p| snapshot.source.files.contains_key(p))
                && graph.graph.nodes.len() <= 5000,
            "Invalid compiler observation binding"
        );
        Ok(())
    }
}
impl Rendering {
    fn project(&self, mut snapshot: Snapshot) -> Result<Snapshot> {
        anyhow::ensure!(
            self.readings.len() <= 5000,
            "Rendering exceeds declaration bounds"
        );
        let mut readings = BTreeMap::new();
        for reading in &self.readings {
            anyhow::ensure!(
                readings.insert(&reading.node, reading).is_none(),
                "Duplicate rendered declaration"
            );
        }
        for node in &mut snapshot.graph_mut().graph.nodes {
            if let Some(semantic) = &mut node.semantic {
                let reading = readings
                    .remove(&node.id)
                    .context("Rendered declaration is missing")?;
                semantic.sentence.clone_from(&reading.sentence);
                semantic.renderer.clone_from(&reading.renderer);
                semantic.rendering_error.clone_from(&reading.error);
                anyhow::ensure!(semantic.valid(), "Invalid rendered explanation");
            }
        }
        anyhow::ensure!(
            readings.is_empty(),
            "Rendering is outside the observed graph"
        );
        Ok(snapshot)
    }
}

pub(super) fn scope(
    paths: &[String],
    prepared: bool,
    compiler: &meden_runtime::plugin::Descriptor,
) -> Result<String> {
    let paths = paths.iter().collect::<BTreeSet<_>>();
    hash(&(paths, prepared, compiler.identity()))
}
fn bind_revision<T>(snapshot: &mut Snapshot, rendering: &Id<T>) -> Result<()> {
    let graph = snapshot.graph_mut();
    graph.revision = GraphRevision::from_digest(sha2::Sha256::digest(serde_json::to_vec(&(
        &graph.revision,
        rendering,
    ))?));
    Ok(())
}

pub fn retain_observation(
    workspace: &Workspace,
    compiled: crate::compiler::Compiled<Snapshot>,
) -> Result<Observation> {
    let reused = compiled.reused();
    let (definition, snapshot) = compiled.into_parts();
    let mut observation = Store::host(workspace)?.observe(snapshot, &definition)?;
    observation.reused = reused;
    Ok(observation)
}
/// Reuse only readings bound to this immutable observation and selected renderer.
pub fn rendering(
    workspace: &Workspace,
    original: &Observation,
    renderer: &meden_runtime::plugin::Descriptor,
) -> Result<Option<Snapshot>> {
    Store::host(workspace)?.rendered(original.compilation(), &original.snapshot, renderer)
}
pub fn retain_rendering(
    workspace: &Workspace,
    original: Observation,
    renderer: &meden_runtime::plugin::Descriptor,
    snapshot: &mut Snapshot,
) -> Result<()> {
    Store::host(workspace)?.render(original, renderer, snapshot)
}

fn artifacts(
    workspace: &Workspace,
    control: &Control,
    selections: impl IntoIterator<Item = Vec<String>>,
    prepared: Option<&PreparedRevision>,
    compiler: &meden_runtime::plugin::Descriptor,
) -> Result<Vec<(Id<Compilation>, Artifact)>> {
    let _timing = meden_runtime::metrics::Span::start("analysis.cache.validate");
    let store = Store::host(workspace)?;
    let index = store.index::<Artifact>()?;
    let mut ids: BTreeMap<Id<Compilation>, BTreeSet<String>> = BTreeMap::new();
    for paths in selections {
        let key = scope(&paths, prepared.is_some(), compiler)?;
        if let Some(id) = index.get(&key) {
            ids.entry(id.clone()).or_default().insert(key);
        }
    }
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let resources = crate::atlas::Configuration::host(workspace.path()).current_resources()?;
    let mut observations = Vec::new();
    for (id, keys) in ids {
        let artifact = store.read::<Artifact>(&id)?;
        artifact.check()?;
        let collection = artifact.snapshot.collection();
        anyhow::ensure!(
            keys.contains(&scope(
                &collection.roots,
                artifact.snapshot.prepared().is_some(),
                compiler
            )?),
            "Compiler observation selection changed"
        );
        artifact.check_compiler(compiler)?;
        if artifact.snapshot.prepared() != prepared
            || artifact.snapshot.graph().resources_sha256 != resources
        {
            continue;
        }
        if crate::adapters::lean::collect::check_metadata(workspace, control, &collection.metadata)
            .is_err()
        {
            continue;
        }
        let paths = artifact
            .snapshot
            .source
            .files
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let Ok(current) = crate::analysis::sources::capture(workspace, control, &paths) else {
            continue;
        };
        if current.revision != artifact.snapshot.source.revision {
            continue;
        }
        observations.push((id, artifact));
    }
    Ok(observations)
}

pub fn load(workspace: &Workspace, control: &Control, paths: &[String]) -> Result<Vec<Note>> {
    let (registry, selected) =
        crate::adapters::compilers(crate::atlas::Configuration::host(workspace.path()))?;
    let compiler = registry.select(selected)?;
    let artifacts = artifacts(
        workspace,
        control,
        paths.iter().map(|p| vec![p.clone()]),
        None,
        compiler.definition(),
    )?;
    if artifacts.is_empty() {
        return Ok(Vec::new());
    }
    let store = Store::host(workspace)?;
    let renderer = crate::adapters::informath::Prepared::discover()?;
    let (searchers, selected) =
        crate::adapters::searchers(crate::atlas::Configuration::host(workspace.path()))?;
    let selected = searchers.select(selected)?;
    let mut notes = Vec::new();
    for (id, artifact) in artifacts {
        let Some(searched) = store.searched(
            &Observation {
                reused: true,
                identity: Identity::Compilation(id),
                snapshot: Arc::new(artifact.snapshot),
            },
            selected.definition(),
        )?
        else {
            continue;
        };
        selected.validate()?;
        // Index projection never executes a renderer, compiler or search.
        let Some(snapshot) = store.rendered(
            searched.compilation(),
            &searched.snapshot,
            renderer.definition(),
        )?
        else {
            continue;
        };
        renderer.validate()?;
        let graph = snapshot.into_graph();
        for node in graph.graph.nodes {
            if !paths.contains(&node.file_path) {
                continue;
            }
            let (Some(line), Some(semantic)) = (node.line_start, node.semantic) else {
                continue;
            };
            anyhow::ensure!(line > 0 && semantic.valid(), "Invalid compiled observation");
            notes.push(Note {
                file: node.file_path,
                name: node.id,
                line,
                semantic: semantic.complete()?,
                main_theorem: node.meta.is_some_and(|m| m.is_main_theorem),
            });
        }
    }
    Ok(notes)
}

pub fn compiled(
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
    invocation: &crate::compiler::Invocation<Snapshot>,
) -> Result<Option<crate::compiler::Compiled<Snapshot>>> {
    compiled_for(workspace, control, paths, None, invocation)
}
/// Only a freshly prepared exact compiler-input fingerprint may reuse Lake observations.
pub fn compiled_prepared(
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
    fingerprint: &PreparedRevision,
    invocation: &crate::compiler::Invocation<Snapshot>,
) -> Result<Option<crate::compiler::Compiled<Snapshot>>> {
    compiled_for(workspace, control, paths, Some(fingerprint), invocation)
}
fn compiled_for(
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
    prepared: Option<&PreparedRevision>,
    invocation: &crate::compiler::Invocation<Snapshot>,
) -> Result<Option<crate::compiler::Compiled<Snapshot>>> {
    let Some((_, artifact)) = artifacts(
        workspace,
        control,
        std::iter::once(paths.to_vec()),
        prepared,
        invocation.definition(),
    )?
    .into_iter()
    .next() else {
        return Ok(None);
    };
    if prepared.is_none()
        && crate::atlas::Configuration::host(workspace.path())
            .frozen_modules(&artifact.snapshot)
            .is_err()
    {
        return Ok(None);
    }
    Ok(Some(
        invocation.restore(artifact.snapshot, &artifact.compiler)?,
    ))
}

#[cfg(test)]
mod tests;
