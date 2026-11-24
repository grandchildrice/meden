//! Exact delivery observations. Decoding these references grants no review or
//! dispatch authority; the existing application/kernel still checks originals.
use super::sequence::Sequence;
use crate::exports;
use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Original session and bounded queue index, never a send capability.
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    #[schemars(length(min = 1, max = 8192))]
    session: String,
    #[serde(deserialize_with = "index")]
    #[schemars(range(max = 7))]
    id: u8,
}
fn index<'de, D: serde::Deserializer<'de>>(decoder: D) -> Result<u8, D::Error> {
    let value = u8::deserialize(decoder)?;
    if value > 7 {
        return Err(serde::de::Error::custom(
            "Export index exceeds the eight-entry queue",
        ));
    }
    Ok(value)
}
impl Reference {
    fn new(value: &exports::Reference) -> Result<Self> {
        anyhow::ensure!(
            value.id < 8 && !value.session.is_empty() && value.session.len() <= 8192,
            "Invalid export reference"
        );
        Ok(Self {
            session: value.session.clone(),
            id: u8::try_from(value.id)?,
        })
    }
    /// Recover the original index/session before the unchanged host comparison.
    pub fn original(self) -> Result<exports::Reference> {
        anyhow::ensure!(
            !self.session.is_empty() && self.session.len() <= 8192,
            "Invalid export reference"
        );
        Ok(exports::Reference {
            session: self.session,
            id: usize::from(self.id),
        })
    }
}
/// Description derived from the original immutable request and observed phase.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    #[serde(flatten)]
    reference: Reference,
    #[schemars(length(max = 8192))]
    source: String,
    #[schemars(regex(pattern = r"^[a-f0-9]{64}$"))]
    contract: Option<String>,
    #[schemars(length(min = 1, max = 8192))]
    target: String,
    #[schemars(range(max = 16_777_216))]
    bytes: u32,
    #[schemars(regex(pattern = r"^[a-f0-9]{64}$"))]
    payload_sha256: String,
    state: exports::State,
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl Summary {
    pub(crate) fn new(value: &exports::Summary) -> Result<Self> {
        anyhow::ensure!(
            value.source.len() <= 8192
                && !value.target.is_empty()
                && value.target.len() <= 8192
                && value.bytes <= 16_777_216
                && digest(&value.payload_sha256)
                && value.contract.as_ref().is_none_or(|c| digest(c)),
            "Invalid export observation"
        );
        Ok(Self {
            reference: Reference::new(&value.reference)?,
            source: value.source.clone(),
            contract: value.contract.clone(),
            target: value.target.clone(),
            bytes: u32::try_from(value.bytes)?,
            payload_sha256: value.payload_sha256.clone(),
            state: value.state,
        })
    }
}
/// Frozen host review page. Serial comparison uses the original exact u64.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Review {
    serial: Sequence,
    summary: Summary,
    #[schemars(range(max = 16_777_216))]
    offset: u32,
    #[schemars(regex(pattern = r"^(?:[0-9a-f]{2})*$"), length(max = 4096))]
    body_hex: String,
}
impl Review {
    pub(crate) fn new(
        serial: u64,
        summary: &exports::Summary,
        offset: usize,
        body_hex: &str,
    ) -> Result<Self> {
        anyhow::ensure!(
            offset <= 16_777_216
                && (offset == 0 || offset < summary.bytes)
                && body_hex.len() <= 4096
                && body_hex.len() % 2 == 0
                && body_hex
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                && body_hex.len() / 2 <= summary.bytes.saturating_sub(offset),
            "Invalid complete export review page"
        );
        Ok(Self {
            serial: Sequence::new(serial)?,
            summary: Summary::new(summary)?,
            offset: u32::try_from(offset)?,
            body_hex: body_hex.into(),
        })
    }
}
/// Original observed result; an uncertain attempt never gains another send.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    summary: Summary,
    #[schemars(range(min = 100, max = 999))]
    http_status: Option<u16>,
}
impl Outcome {
    pub(crate) fn new(summary: &exports::Summary, http_status: Option<u16>) -> Result<Self> {
        anyhow::ensure!(
            http_status.is_none_or(|code| (100..=999).contains(&code)),
            "Invalid observed HTTP status"
        );
        Ok(Self {
            summary: Summary::new(summary)?,
            http_status,
        })
    }
}
/// All original delivery observations at an application boundary.
pub struct Snapshot {
    /// At most eight proposals from the actual session queue.
    pub exports: Vec<Summary>,
    /// Original selected host review, if still retained.
    pub review: Option<Review>,
    /// Last actual attempt/rejection result.
    pub outcome: Option<Outcome>,
}
