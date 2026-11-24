//! Frozen bounded view bytes. Content identities confer no execution authority.
use anyhow::Result;
use schemars::JsonSchema;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{self, Write};
use std::sync::Arc;

/// Complete envelope budget; no prefix is published on failure.
pub const MAX_BYTES: usize = 67_108_864;

/// Bound a status diagnostic by UTF-8 bytes, with an explicit shortening marker.
pub(crate) fn diagnostic(message: &str, marker: &str) -> String {
    if message.len() <= 8000 {
        return message.to_owned();
    }
    let bound = 8000usize.saturating_sub(marker.len());
    let mut result = message
        .char_indices()
        .take_while(|(i, c)| i.saturating_add(c.len_utf8()) <= bound)
        .map(|(_, c)| c)
        .collect::<String>();
    result.push_str(marker);
    result
}

struct Bounded(Vec<u8>);
impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.0.len().saturating_add(bytes.len()) > MAX_BYTES {
            return Err(io::Error::other(
                "View exceeds 64 MiB; no partial publication",
            ));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Exact immutable presentation identity, independent of evidence/source IDs.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    /// Domain-separated identity of every serialized data byte.
    #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
    pub revision: String,
}
/// Shared wire framing; each data role retains its own generated schema.
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Envelope<T> {
    /// Exact publication requested by the viewer.
    #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
    pub revision: String,
    /// Original complete projection, never an authority token.
    pub data: T,
}
/// Own cached bytes instead of serializing a view on every status read.
#[derive(Clone)]
pub struct Publication {
    reference: Reference,
    bytes: Arc<[u8]>,
}
impl Publication {
    /// Serialize once, hash the complete data and freeze its bounded envelope.
    pub fn new(domain: &[u8], data: &impl Serialize) -> Result<Self> {
        Self::freeze(domain, data, None)
    }
    /// An identical revision reuses its original allocation.
    pub fn update(&self, domain: &[u8], data: &impl Serialize) -> Result<Self> {
        Self::freeze(domain, data, Some(self))
    }
    fn freeze(domain: &[u8], data: &impl Serialize, previous: Option<&Self>) -> Result<Self> {
        let mut output = Bounded(Vec::new());
        serde_json::to_writer(&mut output, data)?;
        let payload = output.0;
        let mut hash = Sha256::new();
        hash.update(domain);
        hash.update(&payload);
        let revision = format!("sha256:{:x}", hash.finalize());
        if let Some(previous) = previous.filter(|p| p.reference.revision == revision) {
            return Ok(previous.clone());
        }
        // Only a generated ASCII digest enters this fixed JSON frame. Original
        // data is already serialized by serde; it is never interpolated as text.
        let mut bytes = format!("{{\"revision\":\"{revision}\",\"data\":").into_bytes();
        anyhow::ensure!(
            bytes.len().saturating_add(payload.len()) < MAX_BYTES,
            "View envelope exceeds 64 MiB"
        );
        bytes.extend_from_slice(&payload);
        bytes.push(b'}');
        Ok(Self {
            reference: Reference { revision },
            bytes: bytes.into(),
        })
    }
    /// Read the complete immutable content identity.
    pub fn reference(&self) -> &Reference {
        &self.reference
    }
    /// Clone a frozen body only when the original requested identity matches.
    pub fn bytes(&self, revision: &str) -> Option<Arc<[u8]>> {
        (self.reference.revision == revision).then(|| Arc::clone(&self.bytes))
    }
    /// Retained serialized bytes, for the owning context's eviction budget.
    pub fn size(&self) -> usize {
        self.bytes.len()
    }
}
impl Serialize for Publication {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.reference.serialize(serializer)
    }
}

/// One latest immutable body per key, shared by reader/directory owners.
/// Retention is at most 64 entries / 64 MiB of serialized envelopes.
#[derive(Default)]
pub struct Table {
    values: std::collections::BTreeMap<String, Publication>,
    order: std::collections::VecDeque<String>,
    bytes: usize,
}
impl Table {
    /// Publish already-frozen complete data and retire the previous key value.
    pub fn publish(&mut self, key: String, value: Publication) {
        self.remove(&key);
        self.bytes = self.bytes.saturating_add(value.size());
        self.order.push_back(key.clone());
        self.values.insert(key, value);
        while self.values.len() > 64 || self.bytes > MAX_BYTES {
            let Some(oldest) = self.order.front().cloned() else {
                break;
            };
            self.remove(&oldest);
        }
    }
    /// Exact retained bytes only, without materialization or I/O.
    pub fn bytes(&self, revision: &str) -> Option<Arc<[u8]>> {
        self.values.values().find_map(|value| value.bytes(revision))
    }
    /// Small immutable references; complete data stays out of status.
    pub fn references(&self) -> std::collections::BTreeMap<String, Reference> {
        self.values
            .iter()
            .map(|(key, value)| (key.clone(), value.reference().clone()))
            .collect()
    }
    /// Explicit refresh retires all keys and publications.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    fn remove(&mut self, key: &str) {
        if let Some(old) = self.values.remove(key) {
            self.bytes = self.bytes.saturating_sub(old.size());
        }
        self.order.retain(|entry| entry != key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context as _;
    #[test]
    fn framing_preserves_escaped_text_and_domain_changes_identity() -> Result<()> {
        let data = serde_json::json!({"text":"\"\\\nλ日本語\u{0000}","array":[true,null,19]});
        let original = Publication::new(b"test.one\0", &data)?;
        let revision = &original.reference().revision;
        let bytes = original.bytes(revision).context("body")?;
        let expected = serde_json::to_vec(&Envelope {
            revision: revision.clone(),
            data: &data,
        })?;
        assert_eq!(bytes.as_ref(), expected);
        let mut hash = Sha256::new();
        hash.update(b"test.one\0");
        hash.update(serde_json::to_vec(&data)?);
        assert_eq!(*revision, format!("sha256:{:x}", hash.finalize()));
        let same = original.update(b"test.one\0", &data)?;
        assert!(Arc::ptr_eq(&bytes, &same.bytes(revision).context("same")?));
        assert_ne!(
            revision,
            &Publication::new(b"test.two\0", &data)?.reference().revision
        );
        assert!(original.bytes("obsolete").is_none());
        Ok(())
    }
}
