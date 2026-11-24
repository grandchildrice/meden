//! Frozen bounded view bytes. Content identities confer no execution authority.
use anyhow::Result;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Write};
use std::sync::Arc;
use std::{borrow::Cow, marker::PhantomData};

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

/// One application-owned publication context. A new view supplies its own type.
pub trait Domain {
    /// Original domain separator, fixed by its owning context.
    const PREFIX: &'static [u8];
}
/// A prepared DTO chooses its publication context, never a caller's byte label.
pub trait Data: Serialize {
    /// Context of this exact DTO family, including borrowed projections.
    type Domain: Domain;
}
impl<T: Data> Data for &T {
    type Domain = T::Domain;
}

/// Validated content identity in one view context; never execution authority.
/// Explicit decoding cannot establish that the corresponding body is retained.
///
/// A reader identity cannot be used to query the source owner:
/// ```compile_fail
/// use meden_app::view::{publication::ArtifactId, readers, sources};
/// fn wrong(owner: &sources::Sources, id: &ArtifactId<readers::Artifact>) {
///     owner.bytes(id);
/// }
/// ```
/// The matching source identity is accepted:
/// ```
/// use meden_app::view::{publication::ArtifactId, sources};
/// fn read(owner: &sources::Sources, id: &ArtifactId<sources::Artifact>) {
///     owner.bytes(id);
/// }
/// ```
pub struct ArtifactId<D> {
    value: String,
    domain: PhantomData<fn() -> D>,
}
impl<D> ArtifactId<D> {
    /// Decode a canonical digest at an external boundary. Owners still compare
    /// the exact originally published identity before returning any bytes.
    pub fn parse(value: &str) -> Result<Self> {
        let digest = value.strip_prefix("sha256:").unwrap_or("");
        anyhow::ensure!(
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "Invalid artifact identity"
        );
        Ok(Self {
            value: value.into(),
            domain: PhantomData,
        })
    }
    /// Explicitly erase the context for transport or diagnostics. Decoding this
    /// spelling under another context cannot establish ownership or authority.
    pub fn as_str(&self) -> &str {
        &self.value
    }
}
impl<D> Clone for ArtifactId<D> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            domain: PhantomData,
        }
    }
}
impl<D> std::fmt::Debug for ArtifactId<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}
impl<D> std::fmt::Display for ArtifactId<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}
impl<D> PartialEq for ArtifactId<D> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl<D> Eq for ArtifactId<D> {}
impl<D> PartialOrd for ArtifactId<D> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<D> Ord for ArtifactId<D> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}
impl<D> Serialize for ArtifactId<D> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.value)
    }
}
impl<'de, D> Deserialize<'de> for ArtifactId<D> {
    fn deserialize<S: Deserializer<'de>>(deserializer: S) -> Result<Self, S::Error> {
        Self::parse(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}
impl<D> JsonSchema for ArtifactId<D> {
    fn schema_name() -> Cow<'static, str> {
        "ArtifactId".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({"type":"string","pattern":"^sha256:[a-f0-9]{64}$"})
    }
}
/// Exact immutable presentation reference in its original context.
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields, bound(serialize = ""))]
#[schemars(bound = "", rename = "ArtifactRef")]
pub struct ArtifactRef<D> {
    /// Domain-separated identity of every serialized data byte.
    pub revision: ArtifactId<D>,
}
impl<D> Clone for ArtifactRef<D> {
    fn clone(&self) -> Self {
        Self {
            revision: self.revision.clone(),
        }
    }
}
impl<D> std::fmt::Debug for ArtifactRef<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArtifactRef")
            .field("revision", &self.revision)
            .finish()
    }
}
/// Shared wire framing; each data role retains its own generated schema.
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(bound = "T: Data + JsonSchema")]
pub struct Envelope<T: Data> {
    /// Exact publication requested by the viewer.
    pub revision: ArtifactId<T::Domain>,
    /// Original complete projection, never an authority token.
    pub data: T,
}
/// Own cached bytes instead of serializing a view on every status read.
/// A DTO cannot be frozen under a different owner's context:
/// ```compile_fail
/// use meden_app::view::{directory, publication::Publication, readers};
/// fn wrong(data: &directory::Directory) -> anyhow::Result<Publication<readers::Artifact>> {
///     Publication::new(data)
/// }
/// ```
/// The original DTO family and context agree:
/// ```
/// use meden_app::view::{directory, publication::Publication};
/// fn freeze(data: &directory::Directory) -> anyhow::Result<Publication<directory::Artifact>> {
///     Publication::new(data)
/// }
/// ```
pub struct Publication<D> {
    reference: ArtifactRef<D>,
    bytes: Arc<[u8]>,
}
impl<D: Domain> Publication<D> {
    /// Serialize once, hash the complete data and freeze its bounded envelope.
    pub fn new(data: &impl Data<Domain = D>) -> Result<Self> {
        Self::freeze(data, None)
    }
    /// An identical revision reuses its original allocation.
    pub fn update(&self, data: &impl Data<Domain = D>) -> Result<Self> {
        Self::freeze(data, Some(self))
    }
    fn freeze(data: &impl Data<Domain = D>, previous: Option<&Self>) -> Result<Self> {
        let mut output = Bounded(Vec::new());
        serde_json::to_writer(&mut output, data)?;
        let payload = output.0;
        let mut hash = Sha256::new();
        hash.update(D::PREFIX);
        hash.update(&payload);
        let revision = ArtifactId {
            value: format!("sha256:{:x}", hash.finalize()),
            domain: PhantomData,
        };
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
            reference: ArtifactRef { revision },
            bytes: bytes.into(),
        })
    }
    /// Read the complete immutable content identity.
    pub fn reference(&self) -> &ArtifactRef<D> {
        &self.reference
    }
    /// Clone a frozen body only when the original requested identity matches.
    pub fn bytes(&self, revision: &ArtifactId<D>) -> Option<Arc<[u8]>> {
        (&self.reference.revision == revision).then(|| Arc::clone(&self.bytes))
    }
    /// Retained serialized bytes, for the owning context's eviction budget.
    pub fn size(&self) -> usize {
        self.bytes.len()
    }
}
impl<D> Clone for Publication<D> {
    fn clone(&self) -> Self {
        Self {
            reference: self.reference.clone(),
            bytes: Arc::clone(&self.bytes),
        }
    }
}
impl<D> Serialize for Publication<D> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.reference.serialize(serializer)
    }
}

/// One latest immutable body per key, shared by reader/directory owners.
/// Retention is at most 64 entries / 64 MiB of serialized envelopes.
/// ```compile_fail
/// use meden_app::view::{publication::{Publication, Table}, readers, sources};
/// fn wrong(owner: &mut Table<readers::Artifact>, source: Publication<sources::Artifact>) {
///     owner.publish("same-path".into(), source);
/// }
/// ```
pub struct Table<D> {
    values: std::collections::BTreeMap<String, Publication<D>>,
    order: std::collections::VecDeque<String>,
    bytes: usize,
}
impl<D> Default for Table<D> {
    fn default() -> Self {
        Self {
            values: Default::default(),
            order: Default::default(),
            bytes: 0,
        }
    }
}
impl<D: Domain> Table<D> {
    /// Publish already-frozen complete data and retire the previous key value.
    pub fn publish(&mut self, key: String, value: Publication<D>) {
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
    pub fn bytes(&self, revision: &ArtifactId<D>) -> Option<Arc<[u8]>> {
        self.values.values().find_map(|value| value.bytes(revision))
    }
    /// Small immutable references; complete data stays out of status.
    pub fn references(&self) -> std::collections::BTreeMap<String, ArtifactRef<D>> {
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
    use proptest::prelude::*;
    enum One {}
    enum Two {}
    impl Domain for One {
        const PREFIX: &'static [u8] = b"test.one\0";
    }
    impl Domain for Two {
        const PREFIX: &'static [u8] = b"test.two\0";
    }
    #[derive(Serialize)]
    #[serde(transparent)]
    struct First(serde_json::Value);
    #[derive(Serialize)]
    #[serde(transparent)]
    struct Second(serde_json::Value);
    impl Data for First {
        type Domain = One;
    }
    impl Data for Second {
        type Domain = Two;
    }
    #[test]
    fn framing_preserves_escaped_text_and_original_domain_identity() -> Result<()> {
        let data =
            First(serde_json::json!({"text":"\"\\\nλ日本語\u{0000}","array":[true,null,19]}));
        let original = Publication::new(&data)?;
        let revision = &original.reference().revision;
        let bytes = original.bytes(revision).context("body")?;
        let expected = serde_json::to_vec(&Envelope {
            revision: revision.clone(),
            data: &data,
        })?;
        assert_eq!(bytes.as_ref(), expected);
        let mut hash = Sha256::new();
        hash.update(One::PREFIX);
        hash.update(serde_json::to_vec(&data)?);
        assert_eq!(revision.as_str(), format!("sha256:{:x}", hash.finalize()));
        let same = original.update(&data)?;
        assert!(Arc::ptr_eq(&bytes, &same.bytes(revision).context("same")?));
        let other = Publication::new(&Second(data.0.clone()))?;
        assert_ne!(revision.as_str(), other.reference().revision.as_str());
        // An explicit wire decode is permitted, but is never proof of ownership.
        let foreign = ArtifactId::<Two>::parse(revision.as_str())?;
        assert!(other.bytes(&foreign).is_none());
        assert!(ArtifactId::<One>::parse("obsolete").is_err());
        Ok(())
    }
    #[test]
    fn malformed_or_noncanonical_wire_values_never_decode() -> Result<()> {
        for value in [
            "".into(),
            "sha256:obsolete".into(),
            format!("sha256:{}", "A".repeat(64)),
            format!("sha256:{}", "0".repeat(63)),
            format!(" sha256:{}", "0".repeat(64)),
            format!("sha256:{}\n", "0".repeat(64)),
            format!("sha256:{}", "０".repeat(64)),
        ] {
            assert!(ArtifactId::<One>::parse(&value).is_err());
            assert!(serde_json::from_value::<ArtifactId<One>>(serde_json::json!(value)).is_err());
        }
        for value in [
            serde_json::json!(1),
            serde_json::json!(null),
            serde_json::json!({"revision": "0".repeat(64)}),
        ] {
            assert!(serde_json::from_value::<ArtifactId<One>>(value).is_err());
        }
        Ok(())
    }
    proptest! {
        #[test]
        fn every_observed_digest_round_trips_without_changing_context_or_bytes(digest in any::<[u8;32]>()) {
            let text = format!("sha256:{}", digest.iter().map(|b| format!("{b:02x}")).collect::<String>());
            let original = ArtifactId::<One>::parse(&text).unwrap();
            let encoded = serde_json::to_string(&original).unwrap();
            let decoded: ArtifactId<One> = serde_json::from_str(&encoded).unwrap();
            prop_assert_eq!(decoded, original);
            prop_assert_eq!(encoded, serde_json::to_string(&text).unwrap());
        }
    }
}
