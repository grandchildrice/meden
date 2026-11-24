//! Owned application identities. Decoding checks representation, never authority.
//! The existing controller still compares original sessions, revisions and bytes.
use anyhow::Result;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::{borrow::Cow, num::NonZeroU64};

/// Original project/export session spelling, distinct from a contract digest.
///
/// ```compile_fail
/// use meden_app::identity::{ContractId, SessionId};
/// let contract: ContractId = SessionId::parse("host session").unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionId(String);
impl SessionId {
    /// Decode the original bounded opaque spelling without normalization.
    pub fn parse(value: &str) -> Result<Self> {
        anyhow::ensure!(
            !value.is_empty() && value.len() <= 8192,
            "Invalid session identity"
        );
        Ok(Self(value.into()))
    }
    /// Preserve the host's existing hexadecimal representation of its bytes.
    pub fn from_host_bytes(bytes: &[u8]) -> Result<Self> {
        anyhow::ensure!(
            !bytes.is_empty() && bytes.len() <= 4096,
            "Invalid host session identity"
        );
        Ok(Self(bytes.iter().map(|b| format!("{b:02x}")).collect()))
    }
    /// Explicit original spelling for transport or diagnostics.
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// Original controller parameter bytes; no decoding or normalization occurs.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}
impl Serialize for SessionId {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for SessionId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}
impl JsonSchema for SessionId {
    fn schema_name() -> Cow<'static, str> {
        "SessionId".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({"type":"string","minLength":1,"maxLength":8192})
    }
}
impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Digest of the exact canonical reviewed contract, never a proof receipt.
///
/// ```compile_fail
/// use meden_app::identity::ContractId;
/// let unchecked = ContractId("unvalidated text".into());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractId(String);
impl ContractId {
    /// Decode a complete lowercase digest without changing its bytes.
    pub fn parse(value: &str) -> Result<Self> {
        anyhow::ensure!(
            value.len() == 64
                && value
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "Invalid contract identity"
        );
        Ok(Self(value.into()))
    }
    /// Observe exactly the canonical bytes supplied by the original agreement.
    pub fn from_canonical(bytes: &[u8]) -> Self {
        Self(format!("{:x}", Sha256::digest(bytes)))
    }
    /// Explicit transport/diagnostic projection; it grants no acceptance.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl Serialize for ContractId {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for ContractId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}
impl JsonSchema for ContractId {
    fn schema_name() -> Cow<'static, str> {
        "ContractId".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({"type":"string","pattern":"^[a-f0-9]{64}$"})
    }
}
impl std::fmt::Display for ContractId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Positive original contract generation, distinct from transport/review counters.
///
/// ```compile_fail
/// use meden_app::identity::RequirementRevision;
/// fn accept_revision(_: RequirementRevision) {}
/// accept_revision(1_u64);
/// ```
///
/// ```
/// use meden_app::identity::{ContractId, RequirementRevision, SessionId};
/// use meden_app::project::Reference;
/// let original = Reference {
///     revision: RequirementRevision::new(u64::MAX)?,
///     session: SessionId::parse("original host λ")?,
///     contract: ContractId::from_canonical(b"original reviewed bytes"),
/// };
/// let view = meden_app::view::evidence::Reference::new(&original);
/// assert!(view.original() == original);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequirementRevision(NonZeroU64);
impl RequirementRevision {
    /// Observe the exact existing controller generation; zero means no reference.
    pub fn new(value: u64) -> Result<Self> {
        Ok(Self(NonZeroU64::new(value).ok_or_else(|| {
            anyhow::anyhow!("Requirement revision must be positive")
        })?))
    }
    /// Original controller integer, without a replacement or increment.
    pub fn value(self) -> u64 {
        self.0.get()
    }
}
impl Serialize for RequirementRevision {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(self.value())
    }
}
impl<'de> Deserialize<'de> for RequirementRevision {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::new(u64::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}
impl std::fmt::Display for RequirementRevision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value().fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn host_bytes_keep_the_original_encoding_and_limits() -> Result<()> {
        assert_eq!(
            SessionId::from_host_bytes(b"\0\xffA\xce\xbb")?.as_str(),
            "00ff41cebb"
        );
        assert!(SessionId::from_host_bytes(&[]).is_err());
        assert!(SessionId::from_host_bytes(&vec![0; 4097]).is_err());
        assert_eq!(
            SessionId::from_host_bytes(&vec![0; 4096])?.as_str().len(),
            8192
        );
        assert!(SessionId::parse("").is_err());
        assert!(SessionId::parse(&"x".repeat(8193)).is_err());
        assert!(SessionId::parse(&"𐐀".repeat(2049)).is_err());
        Ok(())
    }

    #[test]
    fn contract_digest_binds_exact_bytes_and_refuses_other_spellings() -> Result<()> {
        let original = ContractId::from_canonical(b"abc");
        assert_eq!(
            original.as_str(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_ne!(original, ContractId::from_canonical(b"abc\n"));
        for invalid in [
            original.as_str().to_uppercase(),
            format!("sha256:{original}"),
            "a".repeat(63),
            "g".repeat(64),
        ] {
            assert!(ContractId::parse(&invalid).is_err());
            assert!(serde_json::from_value::<ContractId>(serde_json::json!(invalid)).is_err());
        }
        assert_eq!(
            serde_json::from_value::<ContractId>(serde_json::json!(original.as_str()))?,
            original
        );
        assert!(RequirementRevision::new(0).is_err());
        assert!(serde_json::from_str::<RequirementRevision>("0").is_err());
        Ok(())
    }

    proptest! {
        #[test]
        fn opaque_sessions_and_numeric_journal_revisions_are_lossless(
            chars in proptest::collection::vec(any::<char>(), 1..=2048),
            revision in 1_u64..=u64::MAX,
        ) {
            let text: String = chars.into_iter().collect();
            let session = SessionId::parse(&text).unwrap();
            let decoded: SessionId = serde_json::from_str(&serde_json::to_string(&session).unwrap()).unwrap();
            prop_assert_eq!(decoded.as_bytes(), text.as_bytes());
            let revision = RequirementRevision::new(revision).unwrap();
            let encoded = serde_json::to_string(&revision).unwrap();
            prop_assert_eq!(&encoded, &revision.value().to_string());
            prop_assert_eq!(serde_json::from_str::<RequirementRevision>(&encoded).unwrap(), revision);
        }
    }
}
