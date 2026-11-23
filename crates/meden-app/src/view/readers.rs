//! Frozen complete file observations. Queries never start a worker or compiler.
use super::{
    file::FileView,
    publication::{Publication, Reference},
};
use anyhow::Result;
use schemars::JsonSchema;
use serde::Serialize;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

const MAX_FILES: usize = 64;
const MAX_BYTES: usize = 67_108_864;

/// Small status index; text and failure details live in their own publication.
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Index {
    /// Latest complete observation of each retained logical path.
    pub files: BTreeMap<String, Reference>,
}

/// Frozen outside the host's shared status lock.
pub struct Prepared {
    path: String,
    publication: Publication,
}
impl Prepared {
    /// Validate the complete input and freeze it once; no partial text is kept.
    pub fn new(view: FileView) -> Result<Self> {
        anyhow::ensure!(
            !view.path.is_empty() && view.path.len() <= 4096 && !view.path.contains('\0'),
            "Invalid reader path"
        );
        match &view.reading {
            super::file::Reading::Ready { content } => content.validate()?,
            super::file::Reading::Unavailable { message, .. } => anyhow::ensure!(
                message.chars().count() <= 8000,
                "Reader diagnostic exceeds its bound"
            ),
        }
        let publication = Publication::new(b"meden.reader.view.v1\0", &view)?;
        Ok(Self {
            path: view.path,
            publication,
        })
    }
}

/// One latest immutable observation per path, with bounded serialized retention.
#[derive(Default)]
pub struct Readers {
    files: BTreeMap<String, Publication>,
    order: VecDeque<String>,
    bytes: usize,
}
impl Readers {
    /// Replacement immediately makes the older file publication unqueryable.
    pub fn publish(&mut self, prepared: Prepared) {
        let Prepared { path, publication } = prepared;
        self.remove(&path);
        self.bytes = self.bytes.saturating_add(publication.size());
        self.order.push_back(path.clone());
        self.files.insert(path, publication);
        while self.files.len() > MAX_FILES || self.bytes > MAX_BYTES {
            let Some(oldest) = self.order.front().cloned() else {
                break;
            };
            self.remove(&oldest);
        }
    }
    /// Refresh/restart cannot revive an older browser read.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    /// Query only already-frozen bytes. The returned Arc outlives the lock.
    pub fn bytes(&self, revision: &str) -> Option<Arc<[u8]>> {
        self.files.values().find_map(|p| p.bytes(revision))
    }
    /// Small typed references, never copies of complete file text.
    pub fn index(&self) -> Index {
        Index {
            files: self
                .files
                .iter()
                .map(|(path, p)| (path.clone(), p.reference().clone()))
                .collect(),
        }
    }
    fn remove(&mut self, path: &str) {
        if let Some(old) = self.files.remove(path) {
            self.bytes = self.bytes.saturating_sub(old.size());
        }
        self.order.retain(|p| p != path);
    }
}
impl Serialize for Readers {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.index().serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::file::{Content, Navigation, Reading};
    use anyhow::Context as _;

    fn prepared(path: &str, text: &str) -> Result<Prepared> {
        Prepared::new(FileView {
            path: path.into(),
            navigation: Navigation::SourceOnly,
            reading: Reading::Ready {
                content: Content::from_bytes(text.as_bytes().to_vec())?,
            },
        })
    }
    #[test]
    fn complete_original_bytes_are_separate_from_status_and_replacement_is_terminal() -> Result<()>
    {
        let mut readers = Readers::default();
        let text = "λ日本語\r\n\"literal\"\\\u{1}".repeat(20_000);
        readers.publish(prepared("notes.txt", &text)?);
        let id = readers
            .index()
            .files
            .get("notes.txt")
            .context("notes reference")?
            .revision
            .clone();
        assert!(serde_json::to_vec(&readers)?.len() < 200);
        let bytes = readers.bytes(&id).context("complete body")?;
        assert!(Arc::ptr_eq(
            &bytes,
            &readers.bytes(&id).context("repeated query")?
        ));
        let decoded: serde_json::Value = serde_json::from_slice(&bytes)?;
        assert_eq!(
            decoded
                .pointer("/data/reading/content/text")
                .and_then(|v| v.as_str()),
            Some(text.as_str())
        );
        readers.publish(prepared("other.txt", "independent")?);
        readers.publish(prepared("notes.txt", "changed")?);
        assert!(readers.bytes(&id).is_none());
        assert_eq!(readers.index().files.len(), 2);
        readers.clear();
        assert!(readers.index().files.is_empty());
        assert!(readers.bytes(&id).is_none());
        assert!(Prepared::new(FileView {
            path: "bad.txt".into(),
            navigation: Navigation::SourceOnly,
            reading: Reading::Ready {
                content: Content::Text {
                    revision: "sha256:".to_string() + &"a".repeat(64),
                    bytes: 1,
                    text: super::super::SourceText::new("changed".into())?,
                }
            },
        })
        .is_err());
        Ok(())
    }
    #[test]
    fn count_and_escaped_byte_limits_evict_whole_oldest_files() -> Result<()> {
        let mut readers = Readers::default();
        readers.publish(prepared("first.txt", "original")?);
        let first = readers
            .index()
            .files
            .get("first.txt")
            .context("first reference")?
            .revision
            .clone();
        for i in 0..64 {
            readers.publish(prepared(&format!("F{i}.txt"), "tiny")?);
        }
        assert!(readers.bytes(&first).is_none());
        assert_eq!(readers.index().files.len(), 64);
        readers.clear();
        let text = "\u{1}".repeat(super::super::file::MAX_BYTES);
        readers.publish(prepared("first.txt", &text)?);
        let first = readers
            .index()
            .files
            .get("first.txt")
            .context("first reference")?
            .revision
            .clone();
        for i in 0..12 {
            readers.publish(prepared(&format!("L{i}.txt"), &text)?);
            assert!(readers.bytes <= MAX_BYTES);
        }
        assert!(readers.bytes(&first).is_none());
        assert!(readers.index().files.len() < 64);
        Ok(())
    }
}
