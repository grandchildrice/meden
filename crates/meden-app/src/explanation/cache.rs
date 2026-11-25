//! Independent immutable compiler/search observations and renderer projections.
//! Neither layer carries accepted-contract evidence or restores an approval.
use super::Explanation;
use crate::analysis::sources::Snapshot;
use crate::identity::snapshots::{GraphRevision, PreparedRevision, SourceRevision};
use anyhow::{Context, Result};
use meden_runtime::{process::Control, workspace::Workspace};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::Digest as _;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Note {
    pub file: String,
    pub name: String,
    pub line: usize,
    pub semantic: Explanation,
    pub main_theorem: bool,
}

/// Consuming reference to the exact retained pre-rendering observation.
/// Renderers cannot mutate the original inputs through this reference.
///
/// ```compile_fail,E0596
/// fn rewrite_original(original: &meden_app::explanation::cache::Observation) {
///     original.snapshot().files.clear();
/// }
/// ```
pub struct Observation {
    id: String,
    compilation: String,
    reused: bool,
    snapshot: Snapshot,
}
impl Observation {
    pub fn reused(&self) -> bool {
        self.reused
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    /// The exact retained compiler/search result, before renderer changes.
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    version: u8,
    compiler: serde_json::Value,
    snapshot: Snapshot,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Rendering {
    observation: String,
    renderer: serde_json::Value,
    readings: Vec<Reading>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Search {
    observation: String,
    searcher: serde_json::Value,
    report: crate::search::Report,
}
impl Store {
    fn searched(
        &self,
        original: &Observation,
        definition: &meden_runtime::plugin::Descriptor,
    ) -> Result<Option<Observation>> {
        let Some(id) = self
            .index("searches")?
            .get(&hash(&(&original.id, definition.identity()))?)
            .cloned()
        else {
            return Ok(None);
        };
        let search: Search = self.read("searches", &id)?;
        anyhow::ensure!(
            search.observation == original.id
                && search.searcher == serde_json::to_value(definition)?,
            "Search input binding changed"
        );
        let mut snapshot = search.report.project(original.snapshot.clone())?;
        bind_revision(&mut snapshot, &id)?;
        Ok(Some(Observation {
            reused: original.reused,
            id,
            compilation: original.compilation.clone(),
            snapshot,
        }))
    }
    fn retain_search(
        &self,
        original: Observation,
        definition: &meden_runtime::plugin::Descriptor,
        report: crate::search::Report,
    ) -> Result<Observation> {
        anyhow::ensure!(
            definition.role() == meden_runtime::plugin::Role::CounterexampleSearch,
            "Invalid search identity"
        );
        let mut snapshot = report.project(original.snapshot)?;
        let compilation = original.compilation;
        let search = Search {
            observation: original.id,
            searcher: serde_json::to_value(definition)?,
            report,
        };
        let key = hash(&(&search.observation, definition.identity()))?;
        let id = self.write("searches", key, &search)?;
        bind_revision(&mut snapshot, &id)?;
        Ok(Observation {
            reused: false,
            id,
            compilation,
            snapshot,
        })
    }
}
pub fn search(
    context: &crate::adapters::Context<'_>,
    original: Observation,
    selected: &crate::search::Registration<'_>,
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
        .search(context, crate::search::Request::new(&original))?
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

// Deliberately exclude the checker, refutation, expression, source and graph.
// A renderer cache can replace only these presentation fields.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Reading {
    node: String,
    sentence: String,
    renderer: Option<String>,
    error: Option<String>,
}

struct Store(PathBuf);
impl Store {
    fn host(workspace: &Workspace) -> Result<Self> {
        Ok(Self(
            crate::storage::directory(workspace.path())?.join("compiled-views"),
        ))
    }
    fn index(&self, role: &str) -> Result<BTreeMap<String, String>> {
        let index: BTreeMap<String, String> =
            crate::storage::read(&self.0.join(format!("{role}-index.json")))?.unwrap_or_default();
        anyhow::ensure!(
            index.len() <= 10_000,
            "Compiled view index exceeds 10000 entries"
        );
        Ok(index)
    }
    fn read<T: DeserializeOwned + Serialize>(&self, role: &str, id: &str) -> Result<T> {
        anyhow::ensure!(
            id.len() == 64 && id.bytes().all(|c| c.is_ascii_hexdigit()),
            "Invalid compiled view artifact ID"
        );
        let artifact: T = crate::storage::read(&self.0.join(role).join(format!("{id}.json")))?
            .context("Missing compiled view artifact")?;
        anyhow::ensure!(
            hash(&artifact)? == id,
            "Compiled view artifact binding changed"
        );
        Ok(artifact)
    }
    fn write(&self, role: &str, key: String, artifact: &impl Serialize) -> Result<String> {
        let id = hash(artifact)?;
        let mut index = self.index(role)?;
        index.insert(key, id.clone());
        anyhow::ensure!(
            index.len() <= 10_000,
            "Compiled view index exceeds 10000 entries"
        );
        crate::storage::write(&self.0.join(role).join(format!("{id}.json")), artifact)?;
        crate::storage::write(&self.0.join(format!("{role}-index.json")), &index)?;
        Ok(id)
    }
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
        let collection = artifact
            .snapshot
            .collection
            .as_ref()
            .context("Missing compilation bindings")?;
        let key = scope(&collection.roots, collection.prepared.is_some(), definition)?;
        let id = hash(&artifact)?;
        if self.index("observations")?.get(&key) == Some(&id) {
            self.read::<Artifact>("observations", &id)?.check()?;
        } else {
            self.write("observations", key, &artifact)?;
        }
        Ok(Observation {
            reused: false,
            compilation: id.clone(),
            id,
            snapshot: artifact.snapshot,
        })
    }
    fn rendered(
        &self,
        id: &str,
        snapshot: &Snapshot,
        renderer: &meden_runtime::plugin::Descriptor,
    ) -> Result<Option<Snapshot>> {
        let _timing = meden_runtime::metrics::Span::start("analysis.cache.load_rendering");
        let Some(rendering_id) = self
            .index("renderings")?
            .get(&hash(&(id, renderer.identity()))?)
            .cloned()
        else {
            return Ok(None);
        };
        let rendering: Rendering = self.read("renderings", &rendering_id)?;
        anyhow::ensure!(
            rendering.observation == id && rendering.renderer == serde_json::to_value(renderer)?,
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
        crate::view::SourcePage::from_snapshot(&original.snapshot)?;
        let _timing = meden_runtime::metrics::Span::start("analysis.cache.store_rendering");
        anyhow::ensure!(
            renderer.role() == meden_runtime::plugin::Role::MeaningRenderer,
            "Invalid renderer identity"
        );
        let graph = snapshot.atlas.as_ref().context("Missing rendered graph")?;
        let rendering = Rendering {
            observation: original.compilation,
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
        let mut expected = rendering.project(original.snapshot)?;
        anyhow::ensure!(
            serde_json::to_vec(&expected)? == serde_json::to_vec(snapshot)?,
            "Rendering changed a compiler/search observation"
        );
        let key = hash(&(&rendering.observation, renderer.identity()))?;
        let id = self.write("renderings", key, &rendering)?;
        bind_revision(&mut expected, &id)?;
        *snapshot = expected;
        Ok(())
    }
}

impl Artifact {
    fn check_compiler(&self, compiler: &meden_runtime::plugin::Descriptor) -> Result<()> {
        anyhow::ensure!(
            compiler.role() == meden_runtime::plugin::Role::Compiler
                && self.compiler == serde_json::to_value(compiler)?,
            "Compiler observation registration changed"
        );
        Ok(())
    }
    fn check(&self) -> Result<()> {
        let snapshot = &self.snapshot;
        let collection = snapshot
            .collection
            .as_ref()
            .context("Missing compilation bindings")?;
        let graph = snapshot
            .atlas
            .as_ref()
            .context("Missing compiled observations")?;
        anyhow::ensure!(
            self.version == 2
                && !snapshot.files.is_empty()
                && snapshot.files.len() <= 32
                && snapshot.revision
                    == SourceRevision::from_digest(sha2::Sha256::digest(serde_json::to_vec(
                        &snapshot.files
                    )?))
                && !collection.roots.is_empty()
                && collection
                    .roots
                    .iter()
                    .all(|p| snapshot.files.contains_key(p))
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
        for node in &mut snapshot
            .atlas
            .as_mut()
            .context("Missing compiled graph")?
            .graph
            .nodes
        {
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

fn hash(value: &impl Serialize) -> Result<String> {
    Ok(crate::atlas::digest(&serde_json::to_vec(value)?))
}
fn scope(
    paths: &[String],
    prepared: bool,
    compiler: &meden_runtime::plugin::Descriptor,
) -> Result<String> {
    let paths = paths.iter().collect::<BTreeSet<_>>();
    hash(&(paths, prepared, compiler.identity()))
}
fn bind_revision(snapshot: &mut Snapshot, rendering: &str) -> Result<()> {
    let graph = snapshot.atlas.as_mut().context("Missing compiled graph")?;
    graph.revision = GraphRevision::from_digest(sha2::Sha256::digest(serde_json::to_vec(&(
        &graph.revision,
        rendering,
    ))?));
    Ok(())
}

pub fn retain_observation(
    workspace: &Workspace,
    compiled: crate::compiler::Compiled,
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
    Store::host(workspace)?.rendered(&original.compilation, &original.snapshot, renderer)
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
) -> Result<Vec<(String, Artifact)>> {
    let _timing = meden_runtime::metrics::Span::start("analysis.cache.validate");
    let store = Store::host(workspace)?;
    let index = store.index("observations")?;
    let mut ids: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
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
        let artifact: Artifact = store.read("observations", &id)?;
        artifact.check()?;
        let collection = artifact
            .snapshot
            .collection
            .as_ref()
            .context("Missing compilation bindings")?;
        anyhow::ensure!(
            keys.contains(&scope(
                &collection.roots,
                collection.prepared.is_some(),
                compiler
            )?),
            "Compiler observation selection changed"
        );
        artifact.check_compiler(compiler)?;
        if collection.prepared.as_ref() != prepared
            || artifact
                .snapshot
                .atlas
                .as_ref()
                .context("Missing compiled graph")?
                .resources_sha256
                != resources
        {
            continue;
        }
        if crate::analysis::collect::check_metadata(workspace, control, &collection.metadata)
            .is_err()
        {
            continue;
        }
        let paths = artifact.snapshot.files.keys().cloned().collect::<Vec<_>>();
        let Ok(current) = crate::analysis::sources::capture(workspace, control, &paths) else {
            continue;
        };
        if current.revision != artifact.snapshot.revision {
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
                compilation: id.clone(),
                id,
                snapshot: artifact.snapshot,
            },
            selected.definition(),
        )?
        else {
            continue;
        };
        selected.validate()?;
        // Index projection never executes a renderer, compiler or search.
        let Some(snapshot) = store.rendered(
            &searched.compilation,
            &searched.snapshot,
            renderer.definition(),
        )?
        else {
            continue;
        };
        renderer.validate()?;
        let graph = snapshot.atlas.context("Missing compiled graph")?;
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
    invocation: &crate::compiler::Invocation,
) -> Result<Option<crate::compiler::Compiled>> {
    compiled_for(workspace, control, paths, None, invocation)
}
/// Only a freshly prepared exact compiler-input fingerprint may reuse Lake observations.
pub fn compiled_prepared(
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
    fingerprint: &PreparedRevision,
    invocation: &crate::compiler::Invocation,
) -> Result<Option<crate::compiler::Compiled>> {
    compiled_for(workspace, control, paths, Some(fingerprint), invocation)
}
fn compiled_for(
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
    prepared: Option<&PreparedRevision>,
    invocation: &crate::compiler::Invocation,
) -> Result<Option<crate::compiler::Compiled>> {
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
