//! Immutable history publications. Imported records remain presentation data.
use crate::records::{Message, View};
use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Publication byte budget, independent of parser/checker/evidence authority.
pub const MAX_BYTES: usize = 67_108_864;

/// Original tool observations, without model roles or execution authority.
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Log {
    /// Host persistence/operation notice.
    Notice {
        /// Original diagnostic.
        text: String,
    },
    /// Original prepared tool call, for inspection only.
    Call {
        /// Original call identity.
        id: String,
        /// Model-facing tool label.
        tool: String,
        /// Original serialized arguments.
        text: String,
    },
    /// Original completed result, for inspection only.
    Result {
        /// Original call identity.
        id: String,
        /// Model-facing tool label.
        tool: String,
        /// Original output.
        text: String,
        /// Observed failure, never an evidence verdict.
        error: bool,
    },
}

/// Conversation and portable-work context, excluded from frequent status reads.
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Data {
    /// Original portable document and its adapter diagnostics.
    pub portable: View,
    /// Current native transcript; it cannot restore kernel grants.
    pub messages: Vec<Message>,
    /// Bounded original tool record sequence.
    #[schemars(length(max = 128))]
    pub logs: Vec<Log>,
}

/// A host publication is available by exact revision or explicitly unavailable.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Reference {
    /// Complete serialized original data is retained.
    Ready {
        /// Content identity, distinct from authority or a receipt.
        #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
        revision: String,
        /// Native transcript count for lightweight startup/navigation state.
        messages: u32,
    },
    /// No incomplete or unbounded publication is admitted.
    Unavailable {
        /// Host diagnostic, independent of any proof outcome.
        message: String,
    },
}

/// Wire envelope for a single immutable publication.
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Snapshot<'a> {
    /// Exact original data identity requested by the viewer.
    #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
    pub revision: String,
    /// Original application records, never current proof authority.
    pub data: &'a Data,
}

/// Sole mutable owner. Only original data mutations rematerialize its bytes.
/// Serialization of this owner emits its lightweight reference, never its data.
pub struct History {
    data: Data,
    reference: Reference,
    bytes: Option<Arc<[u8]>>,
}
impl History {
    /// Freeze the initial projection without restoring authority.
    pub fn new(data: Data) -> Self {
        let mut result = Self {
            data,
            reference: Reference::Unavailable {
                message: "History has not been prepared".into(),
            },
            bytes: None,
        };
        result.edit(|_| {});
        result
    }
    /// Read original records for persistence; callers cannot mutate retained data.
    pub fn data(&self) -> &Data {
        &self.data
    }
    /// One mutation boundary owns both records and their publication.
    pub fn edit(&mut self, edit: impl FnOnce(&mut Data)) {
        edit(&mut self.data);
        let excess = self.data.logs.len().saturating_sub(128);
        self.data.logs.drain(..excess);
        self.refresh();
    }
    /// Share already serialized bytes only for the exact current revision.
    pub fn bytes(&self, revision: &str) -> Option<Arc<[u8]>> {
        match &self.reference {
            Reference::Ready {
                revision: current, ..
            } if current == revision => self.bytes.clone(),
            _ => None,
        }
    }
    fn refresh(&mut self) {
        let _timing = meden_runtime::metrics::Span::start("view.history.materialize");
        let result = (|| -> Result<_> {
            let payload = serde_json::to_vec(&self.data)?;
            anyhow::ensure!(
                payload.len() <= MAX_BYTES,
                "History view exceeds 64 MiB; original records are retained"
            );
            let mut hash = Sha256::new();
            hash.update(b"meden.history.view.v1\0");
            hash.update(&payload);
            let revision = format!("sha256:{hash:x}", hash = hash.finalize());
            let messages = u32::try_from(self.data.messages.len())?;
            if matches!(&self.reference,Reference::Ready {revision:current,..} if current==&revision)
            {
                if let Some(bytes) = &self.bytes {
                    return Ok((self.reference.clone(), Arc::clone(bytes)));
                }
            }
            let bytes = serde_json::to_vec(&Snapshot {
                revision: revision.clone(),
                data: &self.data,
            })?;
            anyhow::ensure!(
                bytes.len() <= MAX_BYTES,
                "History envelope exceeds 64 MiB; original records are retained"
            );
            Ok((
                Reference::Ready { revision, messages },
                Arc::<[u8]>::from(bytes),
            ))
        })();
        match result {
            Ok((reference, bytes)) => {
                self.reference = reference;
                self.bytes = Some(bytes);
            }
            Err(error) => {
                self.reference = Reference::Unavailable {
                    message: error.to_string(),
                };
                self.bytes = None;
            }
        }
    }
}
impl Serialize for History {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.reference.serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::Speaker;
    use anyhow::Context as _;

    fn data() -> Data {
        Data {
            portable: View {
                document: None,
                error: None,
                lean_available: false,
                lean: "not prepared".into(),
            },
            messages: vec![Message {
                speaker: Speaker::User,
                text: "original user request".into(),
            }],
            logs: Vec::new(),
        }
    }
    fn revision(history: &History) -> Result<String> {
        match &history.reference {
            Reference::Ready { revision, .. } => Ok(revision.clone()),
            Reference::Unavailable { .. } => anyhow::bail!("Unavailable history"),
        }
    }
    #[test]
    fn status_is_small_and_bytes_are_reused_for_exact_original_data() -> Result<()> {
        let mut input = data();
        input.messages.get_mut(0).context("message")?.text = "large original text".repeat(100_000);
        let mut history = History::new(input);
        let id = revision(&history)?;
        let bytes = history.bytes(&id).context("body")?;
        assert!(bytes.len() > 1_000_000);
        let status = serde_json::to_vec(&history)?;
        assert!(status.len() < 256);
        assert!(!String::from_utf8(status)?.contains("large original"));
        let body: serde_json::Value = serde_json::from_slice(&bytes)?;
        assert_eq!(body.get("revision"), Some(&serde_json::json!(id)));
        assert_eq!(
            body.get("data"),
            Some(&serde_json::to_value(history.data())?)
        );
        history.edit(|_| {});
        assert!(Arc::ptr_eq(
            &bytes,
            &history.bytes(&id).context("same revision")?
        ));
        let mut hash = Sha256::new();
        hash.update(b"meden.history.view.v1\0");
        hash.update(serde_json::to_vec(history.data())?);
        assert_eq!(id, format!("sha256:{:x}", hash.finalize()));
        Ok(())
    }
    #[test]
    fn replacing_data_refuses_stale_queries_without_changing_a_frozen_response() -> Result<()> {
        let mut history = History::new(data());
        let original = revision(&history)?;
        let bytes = history.bytes(&original).context("original bytes")?;
        history.edit(|data| {
            data.messages.push(Message {
                speaker: Speaker::Assistant,
                text: "new answer".into(),
            });
        });
        assert_ne!(revision(&history)?, original);
        assert!(history.bytes(&original).is_none());
        let frozen: serde_json::Value = serde_json::from_slice(&bytes)?;
        assert_eq!(
            frozen
                .pointer("/data/messages")
                .and_then(|v| v.as_array())
                .map(Vec::len),
            Some(1)
        );
        assert_eq!(history.data().messages.len(), 2);
        assert!(history.bytes("not a revision").is_none());
        Ok(())
    }
    #[test]
    fn log_bounds_and_oversize_failure_preserve_records_without_a_partial_publication() -> Result<()>
    {
        let mut input = data();
        input.logs = (0..130)
            .map(|i| Log::Notice {
                text: i.to_string(),
            })
            .collect();
        let mut history = History::new(input);
        assert_eq!(history.data().logs.len(), 128);
        assert!(matches!(history.data().logs.first(),Some(Log::Notice {text}) if text=="2"));
        let old = revision(&history)?;
        history.edit(|data| {
            data.messages.push(Message {
                speaker: Speaker::Assistant,
                text: "x".repeat(MAX_BYTES),
            });
        });
        assert!(matches!(history.reference, Reference::Unavailable { .. }));
        assert!(history.bytes(&old).is_none());
        assert_eq!(
            history.data().messages.last().map(|m| m.text.len()),
            Some(MAX_BYTES)
        );
        Ok(())
    }
}
