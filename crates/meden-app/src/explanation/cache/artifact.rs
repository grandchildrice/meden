//! Cache identities belong to their complete serialized artifact, not a view
//! publication or lookup key. Decoding establishes spelling only.
use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};
use std::marker::PhantomData;

pub(super) trait Content: Serialize + DeserializeOwned {
    type Domain;
    const DIRECTORY: &'static str;
}

/// Original compiler artifact, separate from source/view publications.
pub enum Compilation {}
/// A separately retained counterexample-search report.
pub enum CounterexampleSearch {}
/// A separately retained natural-language realization.
pub enum MeaningRendering {}

/// Canonical immutable cache identity. Neither decoding nor equality admits
/// evidence or proves that a corresponding body exists.
///
/// Search artifacts cannot replace compilation artifacts:
/// ```compile_fail,E0308
/// use meden_app::explanation::cache::artifact::{Id, Compilation, CounterexampleSearch};
/// fn wrong(id: Id<CounterexampleSearch>) -> Id<Compilation> { id }
/// ```
/// Rendering artifacts cannot replace search artifacts:
/// ```compile_fail,E0308
/// use meden_app::explanation::cache::artifact::{Id, CounterexampleSearch, MeaningRendering};
/// fn wrong(id: Id<MeaningRendering>) -> Id<CounterexampleSearch> { id }
/// ```
/// Publication identities are a separate context:
/// ```compile_fail,E0308
/// use meden_app::{explanation::cache::artifact::{Id, Compilation}, view::{publication::ArtifactId, sources}};
/// fn wrong(id: ArtifactId<sources::Artifact>) -> Id<Compilation> { id }
/// ```
/// The actual retained owner supplies its original compiler identity:
/// ```
/// use meden_app::explanation::cache::{Observation, artifact::{Id, Compilation}};
/// fn original(observation: &Observation) -> &Id<Compilation> { observation.compilation() }
/// ```
pub struct Id<T> {
    digest: String,
    content: PhantomData<fn() -> T>,
}
impl<T> Id<T> {
    pub(super) fn of<C: Content<Domain = T>>(original: &C) -> Result<Self> {
        Ok(Self {
            digest: super::hash(original)?,
            content: PhantomData,
        })
    }
    /// Explicit original spelling for transport/diagnostics, without authority.
    pub fn as_str(&self) -> &str {
        &self.digest
    }
}
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            digest: self.digest.clone(),
            content: PhantomData,
        }
    }
}
impl<T> std::fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.digest.fmt(f)
    }
}
impl<T> std::fmt::Display for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.digest.fmt(f)
    }
}
impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.digest == other.digest
    }
}
impl<T> Eq for Id<T> {}
impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.digest.cmp(&other.digest)
    }
}
impl<T> Serialize for Id<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.digest)
    }
}
impl<'de, T> Deserialize<'de> for Id<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let digest = String::deserialize(d)?;
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(serde::de::Error::custom("Invalid cache artifact identity"));
        }
        Ok(Self {
            digest,
            content: PhantomData,
        })
    }
}
impl<T> schemars::JsonSchema for Id<T> {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "CacheArtifactId".into()
    }
    fn inline_schema() -> bool {
        true
    }
    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({"type":"string","pattern":"^[a-f0-9]{64}$"})
    }
}
