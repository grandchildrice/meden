//! Frozen processor parameters, host approval and typed inference dispatch.

/// Exclusive maximum grant lifetime, shared with host presentation.
pub const APPROVAL_LIFETIME_MILLIS: u64 = 28_800_000;

/// ProcessorOrigin is assigned by the trusted host call site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessorOrigin {
    /// Trusted local UI/configuration operation.
    Host,
    /// Untrusted model, repository or tool output.
    Model,
}

/// Approval is issued once. Revocation and observed expiry are terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessorPhase {
    /// No processor grant has been issued.
    Pending,
    /// The fixed grant deadline has been issued.
    Granted,
    /// Closed by revocation or an observed expired dispatch.
    Revoked,
}

/// Exact prepared local engine/weights paths, without a URL interpretation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalParameters {
    engine: Vec<u8>,
    weights: Vec<u8>,
}

impl LocalParameters {
    /// Freeze the exact host-selected engine and weights paths.
    pub fn new(engine: Vec<u8>, weights: Vec<u8>) -> Self {
        Self { engine, weights }
    }
    /// Exact engine path bytes.
    pub fn engine(&self) -> &[u8] {
        &self.engine
    }
    /// Exact weights path bytes.
    pub fn weights(&self) -> &[u8] {
        &self.weights
    }
}

/// HTTP parameters are stored separately, with exact byte equality and no
/// JSON description to reinterpret after approval. UTF-8 decoding is checked
/// by the runtime; invalid bytes cannot produce a request.
#[derive(Debug, PartialEq, Eq)]
pub struct HttpParameters {
    provider: Vec<u8>,
    endpoint: Vec<u8>,
    model: Vec<u8>,
    credential: Option<Vec<u8>>,
}

// Keep the actual snapshot operation extractable without an opaque
// standard-library Option::clone. Every byte vector is copied exactly.
impl Clone for HttpParameters {
    #[allow(clippy::manual_map)] // The explicit Option branch is part of extraction.
    fn clone(&self) -> Self {
        Self {
            provider: self.provider.clone(),
            endpoint: self.endpoint.clone(),
            model: self.model.clone(),
            credential: match &self.credential {
                Some(key) => Some(key.clone()),
                None => None,
            },
        }
    }
}

impl HttpParameters {
    /// Freeze the exact provider, full inference URL, model and credential.
    pub fn new(
        provider: Vec<u8>,
        endpoint: Vec<u8>,
        model: Vec<u8>,
        credential: Option<Vec<u8>>,
    ) -> Self {
        Self {
            provider,
            endpoint,
            model,
            credential,
        }
    }
    /// Host-selected provider identity, for binding/review.
    pub fn provider(&self) -> &[u8] {
        &self.provider
    }
    /// Complete inference URL, with no suffix added by the executor.
    pub fn endpoint(&self) -> &[u8] {
        &self.endpoint
    }
    /// Exact API model name.
    pub fn model(&self) -> &[u8] {
        &self.model
    }
    /// Exact optional credential. Never include it in host presentation.
    pub fn credential(&self) -> Option<&[u8]> {
        match &self.credential {
            Some(key) => Some(key),
            None => None,
        }
    }
}

/// One connection, fixed for the entire authorization lifecycle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Binding {
    /// Confined local process and prepared weights.
    Local(LocalParameters),
    /// Explicitly approved external or loopback HTTP processor.
    Http(HttpParameters),
}

/// Authority for one local exchange. No public constructor or clone.
#[derive(Debug)]
pub struct LocalCall {
    parameters: LocalParameters,
}

impl LocalCall {
    /// Consume this call and check the actual local resource configuration.
    pub fn consume_for(self, actual: &LocalParameters) -> bool {
        self.parameters == *actual
    }
    /// Consume this call and return its original local parameters.
    pub fn into_parameters(self) -> LocalParameters {
        self.parameters
    }
}

/// Authority for one HTTP exchange. The HTTP executor consumes this value.
/// A local authorization has a different type and cannot be passed instead.
///
/// ```compile_fail
/// use meden_core::processor::{HttpCall, LocalCall};
/// fn send_http(_: HttpCall) {}
/// fn local_is_not_http(local: LocalCall) { send_http(local); }
/// ```
#[derive(Debug)]
pub struct HttpCall {
    parameters: HttpParameters,
}

impl HttpCall {
    /// Consume this call and return its original HTTP parameters.
    pub fn into_parameters(self) -> HttpParameters {
        self.parameters
    }
}

/// The only dispatch values accepted by the inference adapters.
#[derive(Debug)]
pub enum Dispatch {
    /// A local call cannot be used by the HTTP executor.
    Local(LocalCall),
    /// One exchange using the original approved HTTP parameters.
    Http(HttpCall),
}

/// No mutable configuration accessor, permit import or reset operation.
#[derive(Debug)]
pub struct State {
    session: Vec<u8>,
    binding: Binding,
    phase: ProcessorPhase,
    expires: u64,
}

impl State {
    /// Prepare a connection without authority to infer.
    pub fn new(session: Vec<u8>, binding: Binding) -> Self {
        Self {
            session,
            binding,
            phase: ProcessorPhase::Pending,
            expires: 0,
        }
    }
    /// Immutable configuration used by every returned dispatch.
    pub fn binding(&self) -> &Binding {
        &self.binding
    }
    /// Current host approval lifecycle.
    pub fn phase(&self) -> ProcessorPhase {
        self.phase
    }
    /// Exclusive fixed deadline in the caller's monotonic clock domain.
    pub fn expires(&self) -> u64 {
        self.expires
    }

    /// Only the first trusted approval may establish the fixed deadline.
    pub fn approve(&mut self, origin: ProcessorOrigin, now: u64) -> bool {
        if origin != ProcessorOrigin::Host || self.phase != ProcessorPhase::Pending {
            return false;
        }
        let Some(expires) = now.checked_add(APPROVAL_LIFETIME_MILLIS) else {
            return false;
        };
        self.expires = expires;
        self.phase = ProcessorPhase::Granted;
        true
    }

    /// This also closes a proposal that has not yet received approval.
    pub fn revoke(&mut self, origin: ProcessorOrigin) -> bool {
        if origin != ProcessorOrigin::Host {
            return false;
        }
        self.phase = ProcessorPhase::Revoked;
        true
    }

    /// Presentation only; actual exchanges obtain a fresh typed dispatch.
    pub fn is_authorized(&self, session: &[u8], now: u64) -> bool {
        let expected: &[u8] = &self.session;
        self.phase == ProcessorPhase::Granted && expected == session && now < self.expires
    }

    /// Snapshot the original parameters under the caller's lock. No I/O runs
    /// under that lock, so host revocation can stop the local process while
    /// an already permitted exchange is active. No failure grants a fallback.
    pub fn dispatch(&mut self, session: &[u8], now: u64) -> Option<Dispatch> {
        if self.phase == ProcessorPhase::Granted && now >= self.expires {
            self.phase = ProcessorPhase::Revoked;
        }
        if !self.is_authorized(session, now) {
            return None;
        }
        match &self.binding {
            Binding::Local(parameters) => Some(Dispatch::Local(LocalCall {
                parameters: parameters.clone(),
            })),
            Binding::Http(parameters) => Some(Dispatch::Http(HttpCall {
                parameters: parameters.clone(),
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    fn http() -> Binding {
        Binding::Http(HttpParameters::new(
            b"provider".to_vec(),
            b"https://approved.test/v1/chat/completions".to_vec(),
            b"model".to_vec(),
            Some(b"credential".to_vec()),
        ))
    }

    #[test]
    fn approval_scope_frozen_parameters_and_terminal_revocation() {
        let local_parameters = LocalParameters::new(b"engine".to_vec(), b"weights".to_vec());
        assert_eq!(local_parameters.engine(), b"engine");
        assert_eq!(local_parameters.weights(), b"weights");
        for binding in [
            http(),
            Binding::Local(LocalParameters::new(
                b"engine".to_vec(),
                b"weights".to_vec(),
            )),
        ] {
            let mut state = State::new(b"session".to_vec(), binding.clone());
            assert!(state.dispatch(b"session", 0).is_none());
            assert!(!state.approve(ProcessorOrigin::Model, 0));
            assert_eq!(state.phase(), ProcessorPhase::Pending);
            assert!(state.approve(ProcessorOrigin::Host, 7));
            assert_eq!(state.expires(), 7 + APPROVAL_LIFETIME_MILLIS);
            assert!(!state.approve(ProcessorOrigin::Host, 8));
            assert!(!state.revoke(ProcessorOrigin::Model));
            assert!(state.dispatch(b"foreign", 8).is_none());
            for _ in 0..2 {
                let actual = match state.dispatch(b"session", 8).expect("approved") {
                    Dispatch::Http(call) => Binding::Http(call.into_parameters()),
                    Dispatch::Local(call) => {
                        assert!(matches!(binding, Binding::Local(_)), "HTTP became local");
                        Binding::Local(call.into_parameters())
                    }
                };
                assert_eq!(actual, binding);
            }
            assert_eq!(state.binding(), &binding);
            if let Binding::Local(expected) = &binding {
                let local_call = |state: &mut State| {
                    state
                        .dispatch(b"session", 8)
                        .and_then(|dispatch| match dispatch {
                            Dispatch::Local(call) => Some(call),
                            Dispatch::Http(_) => None,
                        })
                        .expect("approved local call")
                };
                assert!(local_call(&mut state).consume_for(expected));
                assert!(!local_call(&mut state).consume_for(&LocalParameters::new(
                    b"other".to_vec(),
                    b"weights".to_vec()
                )));
            }
            assert!(state.revoke(ProcessorOrigin::Host));
            assert!(!state.approve(ProcessorOrigin::Host, 0));
            assert!(state.dispatch(b"session", 0).is_none());
            assert_eq!(state.binding(), &binding);
        }
        let mut pending = State::new(vec![], http());
        assert!(pending.revoke(ProcessorOrigin::Host));
        assert!(!pending.approve(ProcessorOrigin::Host, 0));
        let mut overflow = State::new(vec![], http());
        assert!(!overflow.approve(ProcessorOrigin::Host, u64::MAX));
        assert_eq!(overflow.phase(), ProcessorPhase::Pending);
        let mut expired = State::new(vec![], http());
        assert!(expired.approve(ProcessorOrigin::Host, 0));
        assert!(expired.is_authorized(&[], APPROVAL_LIFETIME_MILLIS - 1));
        assert!(!expired.is_authorized(&[], APPROVAL_LIFETIME_MILLIS));
        assert!(expired.dispatch(&[], APPROVAL_LIFETIME_MILLIS).is_none());
        assert!(!expired.approve(ProcessorOrigin::Host, 0));
        assert!(expired.dispatch(&[], 0).is_none());
    }
}
