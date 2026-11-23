//! Session-bound model port. Implementations must dispatch through their actual
//! consuming processor binding; these metadata/status methods grant no authority.
use meden_agent::conversation::ChatModel;
use serde::{Deserialize, Serialize};

/// Retained diagnostic identity, never a processor grant or executable address.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    /// Selected model identifier.
    pub model: String,
    /// The transport's frozen display destination.
    pub endpoint: String,
}

/// Application-facing implementation, registered by trusted host composition.
/// The transport still enforces the actual session, exact binding, expiry and
/// revocation on every request. A true status here cannot authorize a request.
pub trait Model: Send + Sync {
    /// Diagnostic model label bound by this implementation.
    fn model(&self) -> &str;
    /// Diagnostic destination bound by this implementation.
    fn endpoint(&self) -> &str;
    /// Observe the current grant without renewing or replacing it.
    fn is_authorized(&self, session: &[u8]) -> bool;
    /// Borrow a session-bound caller; no approval operation is exposed here.
    fn for_session<'a>(&'a self, session: &'a [u8]) -> Box<dyn ChatModel + 'a>;
}

/// Capture only the selected implementation's diagnostic labels.
pub fn binding(model: &dyn Model) -> Binding {
    Binding {
        model: model.model().into(),
        endpoint: model.endpoint().into(),
    }
}

/// Status preflight; the real transport remains authoritative on dispatch.
pub fn authorized(judge: Option<&dyn Model>, session: &[u8]) -> bool {
    judge.is_none_or(|model| model.is_authorized(session))
}
