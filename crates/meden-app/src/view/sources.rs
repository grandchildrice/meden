//! Prepared source retention. Querying a publication never performs host I/O.
use super::{
    file,
    publication::{ArtifactId, ArtifactRef},
    PreparedSource,
};
use crate::analysis::sources::Snapshot as Original;
use schemars::JsonSchema;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Arc,
};

/// A bounded number of complete retained artifacts, independent of file count.
pub const MAX_ARTIFACTS: usize = 64;
/// Serialized retention budget; backend inputs have their separate I/O bounds.
pub const MAX_BYTES: usize = 134_217_728;

/// Small generated status projection, containing no source text or declarations.
#[derive(Default, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Index {
    /// Latest prepared backend observation; saved-only views do not set this.
    pub current: Option<ArtifactRef<Artifact>>,
    /// Exact retained artifact for each observed path.
    pub files: BTreeMap<String, ArtifactRef<Artifact>>,
}

/// Complete saved presentation without a backend input for model use.
/// Current dependency freshness remains a separate preparation operation.
/// ```compile_fail,E0599
/// fn input(saved:&meden_app::view::sources::SavedSource) { let _ = saved.snapshot(); }
/// ```
pub struct SavedSource {
    original: Original,
    publication: super::publication::Publication<Artifact>,
}
impl SavedSource {
    /// Freeze only complete presentation and original source bytes; drop the
    /// backend execution/assessment context when its caller releases it.
    pub fn new<O: super::Projection>(snapshot: &O) -> anyhow::Result<Self> {
        let page = super::SourcePage::from_snapshot(snapshot)?;
        Ok(Self {
            original: snapshot.sources().clone(),
            publication: super::publication::Publication::new(&page)?,
        })
    }
}
enum Retained<O> {
    Saved(SavedSource),
    Prepared(PreparedSource<O>),
}
impl<O: super::Projection> Retained<O> {
    fn original(&self) -> &Original {
        match self {
            Self::Saved(s) => &s.original,
            Self::Prepared(s) => s.sources(),
        }
    }
    fn publication(&self) -> &super::publication::Publication<Artifact> {
        match self {
            Self::Saved(s) => &s.publication,
            Self::Prepared(s) => s.publication(),
        }
    }
    fn prepared(&self) -> Option<&PreparedSource<O>> {
        match self {
            Self::Prepared(s) => Some(s),
            Self::Saved(_) => None,
        }
    }
}

/// Sole owner of retained originals, publication identities and eviction order.
pub struct Sources<O> {
    current: Option<ArtifactId<Artifact>>,
    files: BTreeMap<String, ArtifactId<Artifact>>,
    artifacts: BTreeMap<ArtifactId<Artifact>, Retained<O>>,
    order: VecDeque<ArtifactId<Artifact>>,
    bytes: usize,
}
impl<O> Default for Sources<O> {
    fn default() -> Self {
        Self {
            current: None,
            files: BTreeMap::new(),
            artifacts: BTreeMap::new(),
            order: VecDeque::new(),
            bytes: 0,
        }
    }
}
impl<O: super::Projection> Sources<O> {
    /// Inspect the latest original observation for explicit analysis only.
    pub fn current(&self) -> Option<&PreparedSource<O>> {
        self.current
            .as_ref()
            .and_then(|id| self.artifacts.get(id))
            .and_then(Retained::prepared)
    }
    /// Original retained inputs associated with one file, not a browser value.
    pub fn file(&self, path: &str) -> Option<&PreparedSource<O>> {
        self.files
            .get(path)
            .and_then(|id| self.artifacts.get(id))
            .and_then(Retained::prepared)
    }
    /// Publish complete validated data, then evict oldest artifacts by bounds.
    pub fn publish(&mut self, source: PreparedSource<O>) {
        self.retain(Retained::Prepared(source));
    }
    /// Publish complete saved bytes for viewing, without exposing backend inputs.
    pub fn publish_saved(&mut self, source: SavedSource) {
        self.retain(Retained::Saved(source));
    }
    /// A failed freshness operation retires every alias of its saved observation.
    pub fn discard_saved(&mut self, path: &str) {
        if let Some(id) = self
            .files
            .get(path)
            .filter(|id| matches!(self.artifacts.get(*id), Some(Retained::Saved(_))))
            .cloned()
        {
            self.remove(&id);
        }
    }
    fn retain(&mut self, source: Retained<O>) {
        let id = source.publication().reference().revision.clone();
        if let Some(old) = self.artifacts.remove(&id) {
            self.bytes = self.bytes.saturating_sub(old.publication().size());
        }
        self.order.retain(|old| old != &id);
        // Retention is at most 128 MiB before one <=64 MiB insertion.
        self.bytes = self.bytes.saturating_add(source.publication().size());
        for path in source.original().files.keys() {
            self.files.insert(path.clone(), id.clone());
        }
        if source.prepared().is_some() {
            self.current = Some(id.clone());
        } else if self.current.as_ref() == Some(&id) {
            self.current = None;
        }
        self.order.push_back(id.clone());
        self.artifacts.insert(id, source);
        self.prune();
        while self.artifacts.len() > MAX_ARTIFACTS || self.bytes > MAX_BYTES {
            let Some(oldest) = self.order.front().cloned() else {
                break;
            };
            self.remove(&oldest);
        }
    }
    /// Explicit invalidation cannot be undone by a delayed view query.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    /// A failed/current read does not discard independent retained tabs.
    pub fn clear_current(&mut self) {
        self.current = None;
        self.prune();
    }
    /// Preparing a selected file removes its retained association.
    pub fn forget_file(&mut self, path: &str) {
        self.files.remove(path);
        self.prune();
    }
    /// Invalidate every retained graph whose original bytes no longer match.
    /// The adapter uses the returned paths to discard their old progress state.
    pub fn observe(&mut self, observation: &file::FileView) -> Vec<String> {
        let revision = match &observation.reading {
            file::Reading::Ready { content } => content.revision(),
            file::Reading::Unavailable { .. } => None,
        };
        let invalid: BTreeSet<_> = self
            .artifacts
            .iter()
            .filter_map(|(id, source)| {
                let original = source.original().files.get(&observation.path)?;
                (!revision.is_some_and(|r| {
                    r == format!("sha256:{:x}", Sha256::digest(original.as_bytes()))
                }))
                .then(|| id.clone())
            })
            .collect();
        let paths = self
            .files
            .iter()
            .filter(|(_, id)| invalid.contains(*id))
            .map(|(p, _)| p.clone())
            .collect();
        for id in invalid {
            self.remove(&id);
        }
        paths
    }
    /// Return existing immutable bytes only for an exact retained publication.
    pub fn bytes(&self, revision: &ArtifactId<Artifact>) -> Option<Arc<[u8]>> {
        self.artifacts.get(revision)?.publication().bytes(revision)
    }
    /// Derive the small index without exposing the original backend records.
    pub fn index(&self) -> Index {
        Index {
            current: self.current().map(|s| s.publication().reference().clone()),
            files: self
                .files
                .iter()
                .filter_map(|(path, id)| {
                    self.artifacts
                        .get(id)
                        .map(|s| (path.clone(), s.publication().reference().clone()))
                })
                .collect(),
        }
    }
    fn remove(&mut self, id: &ArtifactId<Artifact>) {
        if let Some(source) = self.artifacts.remove(id) {
            self.bytes = self.bytes.saturating_sub(source.publication().size());
        }
        self.files.retain(|_, value| value != id);
        self.order.retain(|value| value != id);
        if self.current.as_ref() == Some(id) {
            self.current = None;
        }
    }
    fn prune(&mut self) {
        let used: BTreeSet<_> = self
            .files
            .values()
            .cloned()
            .chain(self.current.iter().cloned())
            .collect();
        let unused: Vec<_> = self
            .artifacts
            .keys()
            .filter(|id| !used.contains(*id))
            .cloned()
            .collect();
        for id in unused {
            self.remove(&id);
        }
    }
}
impl<O: super::Projection> Serialize for Sources<O> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.index().serialize(serializer)
    }
}

/// Identity context of prepared source/graph publications, distinct from source snapshots.
pub enum Artifact {}
impl super::publication::Domain for Artifact {
    const PREFIX: &'static [u8] = b"meden.source.view.v1\0";
}
impl super::publication::Data for super::SourcePage {
    type Domain = Artifact;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::snapshots::SourceRevision;
    #[derive(Clone)]
    struct Snapshot {
        source: crate::analysis::sources::Snapshot,
        external: Vec<String>,
    }
    impl crate::analysis::sources::Source for Snapshot {
        fn sources(&self) -> &crate::analysis::sources::Snapshot {
            &self.source
        }
    }
    impl crate::view::Projection for Snapshot {
        fn page(&self) -> Result<crate::view::SourcePage> {
            let mut page = crate::view::Projection::page(&self.source)?;
            page.external.clone_from(&self.external);
            Ok(page)
        }
    }
    use anyhow::{Context as _, Result};
    use proptest::prelude::*;

    fn original(files: &[(&str, &str)]) -> Result<Snapshot> {
        let files: BTreeMap<String, String> = files
            .iter()
            .map(|(p, s)| (p.to_string(), s.to_string()))
            .collect();
        Ok(Snapshot {
            source: crate::analysis::sources::Snapshot {
                revision: SourceRevision::from_digest(Sha256::digest(serde_json::to_vec(&files)?)),
                files,
            },
            external: Vec::new(),
        })
    }
    fn observation(path: &str, text: &str) -> Result<file::FileView> {
        Ok(file::FileView {
            path: path.into(),
            navigation: file::Navigation::Declarations,
            reading: file::Reading::Ready {
                content: file::Content::Text {
                    revision: format!("sha256:{:x}", Sha256::digest(text.as_bytes())),
                    bytes: u32::try_from(text.len())?,
                    text: super::super::SourceText::new(text.into())?,
                },
            },
        })
    }
    #[test]
    fn status_is_small_and_changed_input_invalidates_all_related_originals() -> Result<()> {
        let large = format!("-- {}\ndef target := 1\n", "original text".repeat(30_000));
        let mut sources = Sources::default();
        let bundle = PreparedSource::new(original(&[
            ("A.lean", &large),
            ("B.lean", "def b := target\n"),
        ])?)?;
        let id = bundle.publication().reference().revision.clone();
        sources.publish(bundle);
        let bytes = sources.bytes(&id).context("original body")?;
        sources.publish(PreparedSource::new(original(&[(
            "C.lean",
            "def c := 3\n",
        )])?)?);
        assert!(serde_json::to_vec(&sources)?.len() < 1000);
        assert!(bytes.len() > 300_000);
        assert!(Arc::ptr_eq(
            &bytes,
            &sources.bytes(&id).context("same frozen body")?
        ));
        assert!(sources.observe(&observation("A.lean", &large)?).is_empty());
        let removed = sources.observe(&observation("A.lean", "def changed := 2\n")?);
        assert_eq!(removed, vec!["A.lean", "B.lean"]);
        assert!(sources.bytes(&id).is_none());
        assert!(sources.file("A.lean").is_none() && sources.file("B.lean").is_none());
        assert!(sources.file("C.lean").is_some() && sources.current().is_some());
        let old: serde_json::Value = serde_json::from_slice(&bytes)?;
        assert_eq!(
            old.pointer("/data/files/A.lean").and_then(|v| v.as_str()),
            Some(large.as_str())
        );
        sources.clear();
        assert!(sources.current().is_none() && sources.file("C.lean").is_none());
        Ok(())
    }
    #[test]
    fn presentation_changes_receive_an_identity_even_with_the_same_source_and_graph() -> Result<()>
    {
        let mut raw = original(&[("A.lean", "def a := 1\n")])?;
        raw.external = vec!["First".into()];
        let first = PreparedSource::new(raw.clone())?;
        let before = first.publication().reference().revision.clone();
        raw.external = vec!["Second".into()];
        let second = PreparedSource::new(raw)?;
        assert_eq!(first.source.revision, second.source.revision);
        assert_ne!(before, second.publication().reference().revision);
        let mut sources = Sources::default();
        sources.publish(first);
        sources.publish(second);
        assert!(sources.bytes(&before).is_none());
        Ok(())
    }
    #[test]
    fn serialized_retention_evicts_oldest_complete_views_without_partial_bodies() -> Result<()> {
        let mut sources = Sources::default();
        let text = format!("-- {}\ndef a := 1\n", "z".repeat(850_000));
        let mut first = None;
        for batch in 0..6 {
            let paths: Vec<_> = (0..32).map(|i| format!("B{batch}F{i}.lean")).collect();
            // Keep the original 16 MiB scope bound while filling the byte budget.
            let rows: Vec<_> = paths
                .iter()
                .take(19)
                .map(|p| (p.as_str(), text.as_str()))
                .collect();
            let view = PreparedSource::new(original(&rows)?)?;
            if first.is_none() {
                first = Some(view.publication().reference().revision.clone());
            }
            sources.publish(view);
            assert!(sources.bytes <= MAX_BYTES && sources.artifacts.len() <= MAX_ARTIFACTS);
        }
        // 6x16 MiB remains below 128 MiB; lowering a private test budget would
        // mirror implementation. Continue real bounded scopes until eviction.
        for batch in 6..10 {
            let paths: Vec<_> = (0..19).map(|i| format!("B{batch}F{i}.lean")).collect();
            let rows: Vec<_> = paths.iter().map(|p| (p.as_str(), text.as_str())).collect();
            sources.publish(PreparedSource::new(original(&rows)?)?);
        }
        assert!(sources.bytes(first.as_ref().context("first")?).is_none());
        assert!(sources.bytes <= MAX_BYTES && sources.current().is_some());
        Ok(())
    }

    #[test]
    fn saved_views_drop_backend_inputs_and_promote_only_through_complete_preparation() -> Result<()>
    {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct Backend {
            source: crate::analysis::sources::Snapshot,
            dropped: Arc<AtomicUsize>,
        }
        impl crate::analysis::sources::Source for Backend {
            fn sources(&self) -> &crate::analysis::sources::Snapshot {
                &self.source
            }
        }
        impl crate::view::Projection for Backend {
            fn page(&self) -> Result<crate::view::SourcePage> {
                crate::view::Projection::page(&self.source)
            }
        }
        impl Drop for Backend {
            fn drop(&mut self) {
                self.dropped.fetch_add(1, Ordering::SeqCst);
            }
        }
        let original = original(&[("A.lean", "def a := 1\n"), ("B.lean", "def b := 2\n")])?;
        let dropped = Arc::new(AtomicUsize::new(0));
        let saved = {
            let backend = Backend {
                source: original.source.clone(),
                dropped: Arc::clone(&dropped),
            };
            SavedSource::new(&backend)?
        };
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
        let mut sources = Sources::<Snapshot>::default();
        sources.publish_saved(saved);
        let id = sources
            .index()
            .files
            .get("A.lean")
            .context("saved reference")?
            .revision
            .clone();
        let bytes = sources.bytes(&id).context("saved bytes")?;
        assert!(
            sources.current().is_none()
                && sources.file("A.lean").is_none()
                && sources.file("B.lean").is_none()
        );
        assert!(sources.index().current.is_none());
        sources.publish(PreparedSource::new(original.clone())?);
        assert_eq!(sources.index().current.context("prepared")?.revision, id);
        assert_eq!(&*sources.bytes(&id).context("same bytes")?, &*bytes);
        assert!(sources.file("A.lean").is_some() && sources.file("B.lean").is_some());
        sources.publish_saved(SavedSource::new(&original)?);
        assert!(sources.current().is_none() && sources.file("A.lean").is_none());
        Ok(())
    }
    #[test]
    fn failed_or_changed_saved_views_retire_all_aliases_without_discarding_other_tabs() -> Result<()>
    {
        let mut sources = Sources::<Snapshot>::default();
        sources.publish(PreparedSource::new(original(&[(
            "Other.lean",
            "def o := 1\n",
        )])?)?);
        let cached = original(&[("A.lean", "def a := 1\n"), ("B.lean", "def b := 2\n")])?;
        sources.publish_saved(SavedSource::new(&cached)?);
        let id = sources
            .index()
            .files
            .get("A.lean")
            .context("saved")?
            .revision
            .clone();
        sources.discard_saved("A.lean");
        assert!(!sources.index().files.contains_key("B.lean") && sources.bytes(&id).is_none());
        assert!(sources.file("Other.lean").is_some());
        sources.publish_saved(SavedSource::new(&cached)?);
        assert_eq!(
            sources.observe(&observation("B.lean", "def b := 3\n")?),
            vec!["A.lean", "B.lean"]
        );
        assert!(sources.bytes(&id).is_none() && sources.file("Other.lean").is_some());
        sources.publish(PreparedSource::new(cached)?);
        sources.discard_saved("A.lean");
        assert!(sources.file("A.lean").is_some());
        Ok(())
    }
    #[test]
    fn saved_and_prepared_views_share_the_same_bounded_eviction_owner() -> Result<()> {
        let mut sources = Sources::<Snapshot>::default();
        let mut ids = Vec::new();
        for n in 0..80 {
            let path = format!("F{n}.lean");
            let raw = original(&[(&path, "def a := 1\n")])?;
            if n % 2 == 0 {
                sources.publish_saved(SavedSource::new(&raw)?);
            } else {
                sources.publish(PreparedSource::new(raw)?);
            }
            ids.push(
                sources
                    .index()
                    .files
                    .get(&path)
                    .context("reference")?
                    .revision
                    .clone(),
            );
        }
        assert_eq!(sources.artifacts.len(), MAX_ARTIFACTS);
        for (i, id) in ids.iter().enumerate() {
            assert_eq!(sources.bytes(id).is_some(), i >= 16);
        }
        assert!(sources.bytes <= MAX_BYTES);
        Ok(())
    }
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]
        #[test]
        fn finite_publication_traces_preserve_latest_paths_and_refuse_evicted_ids(actions in prop::collection::vec((0u8..5,0u8..90),1..180)) {
            let mut sources=Sources::default();
            let mut expected:Vec<(String,ArtifactId<Artifact>)>=Vec::new();
            let mut current:Option<(String,ArtifactId<Artifact>)>=None;
            let mut ids=Vec::new();
            // Exercise count eviction before random clears: all these small,
            // distinct artifacts fit the byte budget, so it cannot mask a
            // missing 64-artifact guard.
            let prelude = (0u8..65).map(|key| (0, key));
            for (operation,key) in prelude.chain(actions) {
                let path=format!("F{key}.lean");
                match operation {
                    0|1 => {
                        let raw=original(&[(&path,"def a := 1\n")]).map_err(|e|TestCaseError::fail(e.to_string()))?;
                        let source=PreparedSource::new(raw).map_err(|e|TestCaseError::fail(e.to_string()))?;
                        let id=source.publication().reference().revision.clone();
                        expected.retain(|(p,_)|p!=&path);
                        expected.push((path.clone(),id.clone()));
                        if expected.len()>MAX_ARTIFACTS { expected.remove(0); }
                        ids.push(id); sources.publish(source);
                        current=expected.last().cloned();
                    }
                    2=>{sources.observe(&observation(&path,"def changed := 9\n").map_err(|e|TestCaseError::fail(e.to_string()))?); expected.retain(|(p,_)|p!=&path);if current.as_ref().is_some_and(|(p,_)|p==&path){current=None;}}
                    3=>{sources.clear();expected.clear();current=None;}
                    _=>{sources.forget_file(&path);expected.retain(|(p,_)|p!=&path);}
                }
                let index=sources.index();
                let actual:BTreeMap<_,_>=index.files.into_iter().map(|(p,r)|(p,r.revision)).collect();
                let reference:BTreeMap<_,_>=expected.iter().cloned().collect();
                prop_assert_eq!(actual,reference);
                prop_assert!(sources.artifacts.len()<=MAX_ARTIFACTS && sources.bytes<=MAX_BYTES);
                for id in &ids {
                    let retained=expected.iter().any(|(_,r)|r==id)||current.as_ref().is_some_and(|(_,r)|r==id);
                    prop_assert_eq!(sources.bytes(id).is_some(),retained);
                }
            }
        }
    }
}
