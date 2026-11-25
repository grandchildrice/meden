//! Typed immutable artifact storage and retained backend-owned observations.
//! Domain projection and freshness belong to the registered backend adapter.
use anyhow::{Context as _, Result};
use meden_runtime::workspace::Workspace;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
pub mod artifact;
use artifact::{Compilation, Content, CounterexampleSearch, Id, MeaningRendering};

/// Immutable original data. Neither decoded IDs nor this owner admit evidence.
/// ```compile_fail,E0596
/// fn rewrite<O>(original: &meden_app::explanation::cache::Observation<O>) {
///     let _: &mut O = original.snapshot();
/// }
/// ```
pub struct Observation<O> {
    pub(crate) identity: Identity,
    pub(crate) reused: bool,
    pub(crate) snapshot: Arc<O>,
}
pub(crate) enum Identity {
    Compilation(Id<Compilation>),
    Searched {
        compilation: Id<Compilation>,
        search: Id<CounterexampleSearch>,
    },
}
impl<O> Observation<O> {
    pub fn reused(&self) -> bool {
        self.reused
    }
    pub fn compilation(&self) -> &Id<Compilation> {
        match &self.identity {
            Identity::Compilation(id)
            | Identity::Searched {
                compilation: id, ..
            } => id,
        }
    }
    pub fn search_id(&self) -> Option<&Id<CounterexampleSearch>> {
        match &self.identity {
            Identity::Compilation(_) => None,
            Identity::Searched { search, .. } => Some(search),
        }
    }
    pub(crate) fn unsearched(&self) -> Result<&Id<Compilation>> {
        match &self.identity {
            Identity::Compilation(id) => Ok(id),
            Identity::Searched { .. } => anyhow::bail!("Search requires an original compilation"),
        }
    }
    pub fn snapshot(&self) -> &O {
        &self.snapshot
    }
    pub(crate) fn shared(&self) -> Arc<O> {
        Arc::clone(&self.snapshot)
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Artifact<O> {
    pub(crate) version: u8,
    pub(crate) compiler: serde_json::Value,
    pub(crate) snapshot: O,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Rendering {
    pub(crate) observation: Id<Compilation>,
    pub(crate) renderer: serde_json::Value,
    pub(crate) readings: Vec<Reading>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Search<R> {
    pub(crate) observation: Id<Compilation>,
    pub(crate) searcher: serde_json::Value,
    pub(crate) report: R,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Reading {
    pub(crate) node: String,
    pub(crate) sentence: String,
    pub(crate) renderer: Option<String>,
    pub(crate) error: Option<String>,
}
impl<O: Serialize + DeserializeOwned> Content for Artifact<O> {
    type Domain = Compilation;
    const DIRECTORY: &'static str = "observations";
}
impl Content for Rendering {
    type Domain = MeaningRendering;
    const DIRECTORY: &'static str = "renderings";
}
impl<R: Serialize + DeserializeOwned> Content for Search<R> {
    type Domain = CounterexampleSearch;
    const DIRECTORY: &'static str = "searches";
}
pub(crate) struct Store(pub(crate) PathBuf);
impl Store {
    pub(crate) fn host(workspace: &Workspace) -> Result<Self> {
        Ok(Self(
            crate::storage::directory(workspace.path())?.join("compiled-views"),
        ))
    }
    pub(crate) fn index<T: Content>(&self) -> Result<BTreeMap<String, Id<T::Domain>>> {
        let role = T::DIRECTORY;
        let index: BTreeMap<String, Id<T::Domain>> =
            crate::storage::read(&self.0.join(format!("{role}-index.json")))?.unwrap_or_default();
        anyhow::ensure!(
            index.len() <= 10_000,
            "Compiled view index exceeds 10000 entries"
        );
        Ok(index)
    }
    pub(crate) fn read<T: Content>(&self, id: &Id<T::Domain>) -> Result<T> {
        let role = T::DIRECTORY;
        let artifact: T = crate::storage::read(&self.0.join(role).join(format!("{id}.json")))?
            .context("Missing compiled view artifact")?;
        anyhow::ensure!(
            &Id::of(&artifact)? == id,
            "Compiled view artifact binding changed"
        );
        Ok(artifact)
    }
    pub(crate) fn write<T: Content>(&self, key: String, artifact: &T) -> Result<Id<T::Domain>> {
        let role = T::DIRECTORY;
        let id = Id::of(artifact)?;
        let mut index = self.index::<T>()?;
        index.insert(key, id.clone());
        anyhow::ensure!(
            index.len() <= 10_000,
            "Compiled view index exceeds 10000 entries"
        );
        crate::storage::write(&self.0.join(role).join(format!("{id}.json")), artifact)?;
        crate::storage::write(&self.0.join(format!("{role}-index.json")), &index)?;
        Ok(id)
    }
}
pub(crate) fn hash(value: &impl Serialize) -> Result<String> {
    use sha2::Digest as _;
    Ok(format!(
        "{:x}",
        sha2::Sha256::digest(serde_json::to_vec(value)?)
    ))
}
