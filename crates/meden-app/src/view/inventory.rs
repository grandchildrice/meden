//! Generic immutable navigation observations; never compilation or evidence.
use super::{
    publication::{Publication, Reference},
    Kind,
};
use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A source declaration's navigation data, independent of its source language.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Symbol {
    /// Original observed formal name.
    #[schemars(length(min = 1, max = 1_048_576))]
    pub name: String,
    /// One-based original line.
    #[schemars(range(min = 1, max = 1_048_577))]
    pub line: u32,
    /// Generic presentation role, never proof strength.
    pub kind: Kind,
    /// Original declaration spelling for a format-specific label.
    pub declaration: String,
    /// Lexically observed visibility.
    pub private: bool,
    /// Lexical name candidates, not elaborated dependencies.
    pub references: Vec<String>,
    /// A separately prepared description/search observation, when available.
    pub semantic: Option<crate::explanation::Explanation>,
    /// Explicit target metadata, not a proof receipt.
    pub main_theorem: bool,
}
impl TryFrom<crate::adapters::source::Symbol> for Symbol {
    type Error = anyhow::Error;
    fn try_from(original: crate::adapters::source::Symbol) -> Result<Self> {
        let kind = match original.kind.as_str() {
            "theorem" => Kind::Theorem,
            "definition" => Kind::Definition,
            "axiom" => Kind::Axiom,
            _ => anyhow::bail!("Unknown navigation role"),
        };
        anyhow::ensure!(
            !original.name.is_empty() && original.name.len() <= 1_048_576,
            "Invalid navigation name"
        );
        let line = u32::try_from(original.line)?;
        anyhow::ensure!((1..=1_048_577).contains(&line), "Invalid navigation line");
        Ok(Self {
            name: original.name,
            line,
            kind,
            declaration: original.declaration,
            private: original.private,
            references: original.references,
            semantic: original.semantic,
            main_theorem: original.main_theorem,
        })
    }
}
/// Bounded metadata and navigation; complete source text is a separate query.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct File {
    /// Original relative workspace path.
    #[schemars(length(min = 1, max = 4096))]
    pub path: String,
    /// Complete text digest when indexed; empty for unindexed navigation.
    #[schemars(regex(pattern = r"^(?:[a-f0-9]{64})?$"))]
    pub sha256: String,
    /// Canonical decimal avoids loss of large metadata sizes in JavaScript.
    #[schemars(regex(pattern = r"^(0|[1-9][0-9]{0,19})$"))]
    pub bytes: String,
    /// Lexical counts, never compiler results.
    #[schemars(range(max = 1_048_576))]
    pub declarations: u32,
    /// Lexical theorem count.
    #[schemars(range(max = 1_048_576))]
    pub theorems: u32,
    /// Original import name candidates.
    pub imports: Vec<String>,
    /// Retained navigation declarations.
    pub symbols: Vec<Symbol>,
    /// False includes unread/unsupported inputs; it never asserts readability.
    pub indexed: bool,
}
impl File {
    /// Validate complete metadata at the presentation boundary.
    pub fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            !self.path.is_empty()
                && self.path.len() <= 4096
                && !self.path.contains('\0')
                && !self.path.starts_with('/')
                && self
                    .path
                    .split('/')
                    .all(|p| !p.is_empty() && p != "." && p != ".."),
            "Invalid inventory path"
        );
        let bytes = self.bytes.parse::<u64>()?;
        anyhow::ensure!(
            bytes.to_string() == self.bytes,
            "Invalid inventory byte count"
        );
        anyhow::ensure!(
            self.theorems <= self.declarations && self.declarations <= 1_048_576,
            "Invalid navigation counts"
        );
        if self.indexed {
            anyhow::ensure!(
                bytes <= 1_048_576
                    && self.sha256.len() == 64
                    && self
                        .sha256
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                "Invalid indexed text metadata"
            );
        } else {
            anyhow::ensure!(
                self.sha256.is_empty()
                    && self.declarations == 0
                    && self.theorems == 0
                    && self.imports.is_empty()
                    && self.symbols.is_empty(),
                "Unindexed inventory contains navigation"
            );
        }
        anyhow::ensure!(
            self.symbols.len() <= usize::try_from(self.declarations)?,
            "Navigation exceeds its declaration count"
        );
        for symbol in &self.symbols {
            anyhow::ensure!(
                !symbol.name.is_empty()
                    && symbol.name.len() <= 1_048_576
                    && (1..=1_048_577).contains(&symbol.line)
                    && symbol.semantic.as_ref().is_none_or(
                        crate::explanation::Explanation::<crate::explanation::Check>::valid
                    ),
                "Invalid navigation symbol"
            );
        }
        Ok(())
    }
}
/// Complete bounded inventory projection; individual prepared chunks may enrich it.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    /// Prepared navigation-cache generation, as a lossless canonical decimal.
    /// The complete envelope ID binds this hint and all original metadata.
    #[schemars(regex(pattern = r"^(0|[1-9][0-9]{0,19})$"))]
    pub generation: String,
    /// Complete source inventory, in canonical path order.
    #[schemars(length(max = 250_000))]
    pub files: Vec<File>,
    /// Explicitly excluded directory names.
    pub excluded: Vec<String>,
    /// Large inventories load already-prepared navigation in bounded chunks.
    pub inventory_only: bool,
}
/// Complete generated chunk; revision binds its parent inventory scope.
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Chunk {
    /// Exact parent publication requested by the browser.
    #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
    pub revision: String,
    /// Complete selected file and import-only neighbors, never source prefixes.
    #[schemars(length(min = 1, max = 32))]
    pub files: Vec<File>,
}
/// Frozen bytes prepared before a status reference becomes visible.
#[derive(Clone)]
pub struct Prepared(Publication);
impl Prepared {
    /// Validate the complete projection and serialize it once outside host locks.
    pub fn new(snapshot: &Snapshot) -> Result<Self> {
        let _timing = meden_runtime::metrics::Span::start("view.inventory.materialize");
        anyhow::ensure!(
            snapshot.files.len() <= 250_000
                && snapshot.files.windows(2).all(|p| p
                    .first()
                    .zip(p.get(1))
                    .is_some_and(|(a, b)| a.path < b.path)),
            "Invalid inventory order/count"
        );
        for file in &snapshot.files {
            file.validate()?;
        }
        let generation = snapshot.generation.parse::<u64>()?;
        anyhow::ensure!(
            generation.to_string() == snapshot.generation,
            "Invalid inventory generation"
        );
        Ok(Self(Publication::new(
            b"meden.inventory.view.v1\0",
            snapshot,
        )?))
    }
    /// Small status identity; it confers no execution/evidence authority.
    pub fn reference(&self) -> &Reference {
        self.0.reference()
    }
    /// Exact immutable bytes only; querying cannot prepare anything.
    pub fn bytes(&self, revision: &str) -> Option<Arc<[u8]>> {
        self.0.bytes(revision)
    }
}

fn validate_revision(revision: &str) -> Result<()> {
    let digest = revision.strip_prefix("sha256:").unwrap_or("");
    anyhow::ensure!(
        digest.len() == 64
            && digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "Invalid inventory identity"
    );
    Ok(())
}

/// Bounded cached chunks owned by one immutable parent inventory.
pub struct Chunks {
    parent: Reference,
    values: std::collections::BTreeMap<String, Publication>,
    order: std::collections::VecDeque<String>,
    bytes: usize,
}
impl Chunks {
    /// Freeze the original parent, independently of subsequent host inventory.
    pub fn new(parent: Reference) -> Result<Self> {
        validate_revision(&parent.revision)?;
        Ok(Self {
            parent,
            values: std::collections::BTreeMap::new(),
            order: std::collections::VecDeque::new(),
            bytes: 0,
        })
    }
    /// Only already-frozen bytes; an ordinary cache hit does no artifact I/O.
    pub fn bytes(&self, path: &str) -> Option<Arc<[u8]>> {
        let value = self.values.get(path)?;
        value.bytes(&value.reference().revision)
    }
    /// Validate a complete parent-bound chunk and retain its immutable envelope.
    pub fn publish(&mut self, path: &str, chunk: &Chunk) -> Result<Arc<[u8]>> {
        anyhow::ensure!(
            chunk.revision == self.parent.revision
                && !chunk.files.is_empty()
                && chunk.files.len() <= 32
                && chunk.files.iter().any(|f| f.path == path),
            "Invalid inventory chunk binding"
        );
        let mut paths = std::collections::BTreeSet::new();
        for file in &chunk.files {
            file.validate()?;
            anyhow::ensure!(paths.insert(&file.path), "Duplicate inventory chunk file");
        }
        let value = Publication::new(b"meden.inventory.chunk.v1\0", chunk)?;
        let bytes = value
            .bytes(&value.reference().revision)
            .ok_or_else(|| anyhow::anyhow!("Missing prepared chunk"))?;
        self.remove(path);
        self.bytes = self.bytes.saturating_add(value.size());
        self.order.push_back(path.into());
        self.values.insert(path.into(), value);
        while self.values.len() > 64 || self.bytes > 67_108_864 {
            let Some(oldest) = self.order.front().cloned() else {
                break;
            };
            self.remove(&oldest);
        }
        Ok(bytes)
    }
    fn remove(&mut self, path: &str) {
        if let Some(value) = self.values.remove(path) {
            self.bytes = self.bytes.saturating_sub(value.size());
        }
        self.order.retain(|p| p != path);
    }
}

/// Inventory availability is separate from compiler jobs and source readability.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// Original host capture is active.
    Loading,
    /// The complete prepared inventory is queryable.
    Ready,
    /// An edit requires an explicit/new host observation.
    Stale,
    /// No complete new inventory was published.
    Failed,
}
/// Small generated progress/reference projection, never the inventory body.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Status {
    /// Host-observed state.
    pub state: State,
    /// Completed file observations.
    #[schemars(range(max = 250_000))]
    pub scanned: usize,
    /// Known source-file count.
    #[schemars(range(max = 250_000))]
    pub total: usize,
    /// Exact complete publication, if available.
    #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
    pub revision: Option<String>,
    /// Local bounded diagnostic, never an outcome classifier.
    pub error: Option<String>,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            state: State::Loading,
            scanned: 0,
            total: 0,
            revision: None,
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context as _;

    fn unread(path: &str) -> File {
        File {
            path: path.into(),
            sha256: String::new(),
            bytes: "0".into(),
            declarations: 0,
            theorems: 0,
            imports: vec![],
            symbols: vec![],
            indexed: false,
        }
    }
    fn input(files: Vec<File>) -> Snapshot {
        Snapshot {
            generation: "0".into(),
            files,
            excluded: vec!["target".into()],
            inventory_only: true,
        }
    }
    #[test]
    fn inventory_is_complete_immutable_and_rejects_ambiguous_or_malformed_metadata() -> Result<()> {
        let snapshot = input(vec![unread("A.lean"), unread("Z.rs")]);
        let prepared = Prepared::new(&snapshot)?;
        let id = &prepared.reference().revision;
        let bytes = prepared.bytes(id).context("complete inventory")?;
        assert!(Arc::ptr_eq(
            &bytes,
            &prepared.bytes(id).context("same publication")?
        ));
        let decoded: serde_json::Value = serde_json::from_slice(&bytes)?;
        assert_eq!(
            decoded
                .pointer("/data/files/1/path")
                .and_then(|v| v.as_str()),
            Some("Z.rs")
        );
        assert!(prepared.bytes("sha256:obsolete").is_none());
        for files in [
            vec![unread("B.lean"), unread("A.lean")],
            vec![unread("A.lean"), unread("A.lean")],
            vec![unread("../outside.lean")],
        ] {
            assert!(Prepared::new(&input(files)).is_err());
        }
        let mut limit = unread("huge.lean");
        limit.bytes = u64::MAX.to_string();
        Prepared::new(&input(vec![limit.clone()]))?;
        limit.bytes = "18446744073709551616".into();
        assert!(Prepared::new(&input(vec![limit.clone()])).is_err());
        limit.bytes = "01".into();
        assert!(Prepared::new(&input(vec![limit])).is_err());
        let mut false_index = unread("wrong.lean");
        false_index.declarations = 1;
        assert!(Prepared::new(&input(vec![false_index])).is_err());
        Ok(())
    }
    #[test]
    fn chunk_cache_binds_parent_path_and_whole_body_and_evicts_oldest_entries() -> Result<()> {
        let parent = Prepared::new(&input(vec![unread("A.lean")]))?
            .reference()
            .clone();
        let mut chunks = Chunks::new(parent.clone())?;
        let chunk = Chunk {
            revision: parent.revision.clone(),
            files: vec![unread("A.lean")],
        };
        let original = chunks.publish("A.lean", &chunk)?;
        assert!(Arc::ptr_eq(
            &original,
            &chunks.bytes("A.lean").context("cache hit")?
        ));
        let envelope: serde_json::Value = serde_json::from_slice(&original)?;
        assert_eq!(
            envelope.pointer("/data/revision").and_then(|v| v.as_str()),
            Some(parent.revision.as_str())
        );
        assert!(chunks.publish("B.lean", &chunk).is_err());
        assert!(chunks
            .publish(
                "A.lean",
                &Chunk {
                    revision: "sha256:".to_string() + &"b".repeat(64),
                    files: vec![unread("A.lean")]
                }
            )
            .is_err());
        assert!(chunks
            .publish(
                "A.lean",
                &Chunk {
                    revision: parent.revision.clone(),
                    files: vec![unread("A.lean"), unread("A.lean")]
                }
            )
            .is_err());
        for i in 0..64 {
            let path = format!("F{i}.lean");
            chunks.publish(
                &path,
                &Chunk {
                    revision: parent.revision.clone(),
                    files: vec![unread(&path)],
                },
            )?;
        }
        assert!(chunks.bytes("A.lean").is_none());
        assert_eq!(chunks.values.len(), 64);
        assert!(chunks.bytes <= 67_108_864);
        Ok(())
    }
    #[test]
    fn chunk_byte_budget_evicts_complete_bodies_before_the_count_limit() -> Result<()> {
        let parent = Prepared::new(&input(vec![]))?.reference().clone();
        let mut chunks = Chunks::new(parent.clone())?;
        let reference = "reference".repeat(900_000);
        for i in 0..10 {
            let path = format!("L{i}.lean");
            let mut file = unread(&path);
            file.indexed = true;
            file.sha256 = "a".repeat(64);
            file.bytes = "1048576".into();
            file.declarations = 1;
            file.symbols.push(Symbol {
                name: "item".into(),
                line: 1,
                kind: Kind::Definition,
                declaration: "def".into(),
                private: false,
                references: vec![reference.clone()],
                semantic: None,
                main_theorem: false,
            });
            chunks.publish(
                &path,
                &Chunk {
                    revision: parent.revision.clone(),
                    files: vec![file],
                },
            )?;
            assert!(chunks.bytes <= 67_108_864);
        }
        assert!(chunks.bytes("L0.lean").is_none());
        assert!(chunks.values.len() < 64);
        Ok(())
    }
}
