//! Exact, single-use outbound authorization, modelled in
//! `lean/MedenSpec/Transfer.lean`. This is a decision kernel, not a network
//! boundary: the host must control permit creation and mediate every send.

use serde::{Deserialize, Serialize};

/// Identity of one immutable prepared outbound operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    /// Host-created session identity.
    pub session: Vec<u8>,
    /// Effective destination/audience, assigned by the host adapter.
    pub recipient: Vec<u8>,
    /// Canonical complete request, including the exact payload.
    pub request: Vec<u8>,
}

/// Host-issued permission for one exact request before an exclusive deadline.
/// This type must never be deserialized from model/tool output as authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permit {
    /// Exact prepared operation approved by the host/user.
    pub request: Request,
    /// Exclusive deadline in the host session's monotonic time domain.
    pub expires: u64,
    /// Reservation is permanent, even if network delivery later fails.
    pub spent: bool,
}

/// Check all authorization conditions without changing state.
pub fn allowed(now: u64, session: &[u8], permit: &Permit, request: &Request) -> bool {
    let request_session: &[u8] = &request.session;
    request_session == session
        && permit.request == *request
        && now < permit.expires
        && !permit.spent
}

/// Atomically reserve under the caller's lock, before any external effect.
/// The returned permission is valid only for the supplied immutable request.
pub fn reserve(now: u64, session: &[u8], permit: &mut Permit, request: &Request) -> bool {
    let accepted = allowed(now, session, permit, request);
    permit.spent |= accepted;
    accepted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> Request {
        Request {
            session: b"session-a".to_vec(),
            recipient: b"recipient-a".to_vec(),
            request: b"exact bytes".to_vec(),
        }
    }

    #[test]
    fn changing_session_recipient_or_payload_cannot_spend_a_permit() {
        let expected = request();
        let permit = Permit {
            request: expected.clone(),
            expires: 10,
            spent: false,
        };
        for changed in [
            Request {
                session: b"other-session".to_vec(),
                ..expected.clone()
            },
            Request {
                recipient: b"other-recipient".to_vec(),
                ..expected.clone()
            },
            Request {
                request: b"encoded secret or different bytes".to_vec(),
                ..expected.clone()
            },
        ] {
            let mut state = permit.clone();
            assert!(!reserve(0, b"session-a", &mut state, &changed));
            assert_eq!(state, permit);
        }
        let mut state = permit.clone();
        assert!(!reserve(10, b"session-a", &mut state, &expected));
        assert_eq!(state, permit);
        assert!(reserve(9, b"session-a", &mut state, &expected));
        assert!(!reserve(0, b"session-a", &mut state, &expected));
    }
}
