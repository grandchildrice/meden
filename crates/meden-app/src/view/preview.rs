//! Frozen descriptions of actual static captures. These are neither proof
//! evidence nor authority to read a workspace or start a browser server.
use super::publication::{Envelope, Publication};
use crate::preview::Bundle;
use anyhow::Result;
use schemars::JsonSchema;
use serde::Serialize;
use std::sync::{Arc, Mutex};

/// A bounded literal browser observation, never an asset's contents.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct Diagnostic(#[schemars(length(max = 2400))] String);

/// Original complete captured-bundle description and host-provided frame URL.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Data {
    #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
    pub(crate) frame: String,
    #[schemars(length(min = 1, max = 4096))]
    pub(crate) root: String,
    #[schemars(length(min = 1, max = 4096))]
    pub(crate) entry: String,
    #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
    pub(crate) revision: String,
    #[schemars(url, length(min = 1, max = 16384))]
    pub(crate) url: String,
    #[schemars(range(min = 1, max = 256))]
    pub(crate) file_count: u32,
    #[schemars(range(max = 16_777_216))]
    pub(crate) byte_count: u32,
    #[schemars(length(max = 20))]
    pub(crate) errors: Vec<Diagnostic>,
}
/// The body revision includes the original asset revision and frame URL.
pub type Snapshot = Envelope<Data>;

/// Small status, independently serialized without inspecting assets or errors.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Status {
    /// No preview has been captured in this host session.
    Idle,
    /// Complete frozen description available at exactly this revision.
    Ready {
        /// Original frame/bundle context; diagnostic updates do not replace it.
        #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
        frame: String,
        /// Content identity, separate from the captured asset-manifest identity.
        #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
        revision: String,
    },
    /// An incomplete or superseded description is not served.
    Unavailable {
        /// Bounded literal observation failure.
        #[schemars(length(max = 8000))]
        message: String,
    },
}

struct State {
    data: Option<Data>,
    publication: Option<Publication>,
    status: Status,
}
impl State {
    fn unavailable(&mut self, message: &str) {
        self.publication = None;
        self.status = Status::Unavailable {
            message: super::publication::diagnostic(message, "… (diagnostic shortened)"),
        };
    }
    fn publish(&mut self) -> Result<()> {
        let _span = meden_runtime::metrics::Span::start("view.preview.materialize");
        let data = self
            .data
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Preview observation missing"))?;
        let result = match &self.publication {
            Some(previous) => previous.update(b"meden.preview.v1\0", data),
            None => Publication::new(b"meden.preview.v1\0", data),
        };
        match result {
            Ok(value) => {
                self.status = Status::Ready {
                    frame: data.frame.clone(),
                    revision: value.reference().revision.clone(),
                };
                self.publication = Some(value);
                Ok(())
            }
            Err(error) => {
                self.unavailable(&error.to_string());
                Err(error)
            }
        }
    }
}
/// Host capture/asset observations advance this application-owned description.
/// It cannot serve bytes, launch workers or create an I/O authority.
pub struct Previews(Arc<Mutex<State>>);
impl Default for Previews {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(State {
            data: None,
            publication: None,
            status: Status::Idle,
        })))
    }
}
impl Previews {
    /// Read-only handle used by transports and frequent status polling.
    pub fn reader(&self) -> Reader {
        Reader(Arc::clone(&self.0))
    }

    /// Retire the previous frame description before another capture attempt.
    pub fn clear(&self) {
        if let Ok(mut state) = self.0.lock() {
            state.data = None;
            state.publication = None;
            state.status = Status::Idle;
        }
    }
    /// Publish only metadata from the original successfully prepared bundle.
    /// The actual URL's origin and listener remain host adapter obligations.
    pub fn install(&self, bundle: &Bundle, url: String) -> Result<()> {
        let mut state = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("Preview view unavailable"))?;
        match bundle.description(url) {
            Ok(data) => {
                state.data = Some(data);
                state.publish()
            }
            Err(error) => {
                state.data = None;
                state.unavailable(&error.to_string());
                Err(error)
            }
        }
    }
    /// Original host-observed missing asset, bounded like the existing adapter.
    /// Old diagnostics cannot recover an unavailable or cleared description.
    pub fn missing(&self, path: &str) {
        let Ok(mut state) = self.0.lock() else { return };
        if !matches!(state.status, Status::Ready { .. }) {
            return;
        }
        let Some(data) = state.data.as_mut() else {
            return;
        };
        if data.errors.len() == 20 {
            return;
        }
        let mut chars = path.chars();
        let mut text = chars.by_ref().take(500).collect::<String>();
        if chars.next().is_some() {
            text.push_str("… (path shortened)");
        }
        data.errors
            .push(Diagnostic(format!("Missing preview asset: {text}")));
        let _ = state.publish();
    }
}
/// Query-only capability to the latest immutable description bytes.
#[derive(Clone)]
pub struct Reader(Arc<Mutex<State>>);
impl Reader {
    /// No asset/source reads, serialization or worker calls on this path.
    pub fn bytes(&self, revision: &str) -> Option<Arc<[u8]>> {
        self.0.lock().ok()?.publication.as_ref()?.bytes(revision)
    }
}
impl Serialize for Reader {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0.lock() {
            Ok(state) => state.status.serialize(serializer),
            Err(_) => Status::Unavailable {
                message: "Preview view unavailable".into(),
            }
            .serialize(serializer),
        }
    }
}
