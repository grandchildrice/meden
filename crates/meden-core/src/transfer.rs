//! Exact outbound authorization predicate, modelled in
//! `lean/MedenSpec/Transfer.lean`. This is a decision kernel, not a network
//! boundary: the host must control permit creation and mediate every send.

use serde::{Deserialize, Serialize};

/// Identity of one immutable approved operation or processor binding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    /// Host-created session identity.
    pub session: Vec<u8>,
    /// Effective destination/audience, assigned by the host adapter.
    pub recipient: Vec<u8>,
    /// Canonical approved binding; its interpretation belongs to the adapter.
    pub request: Vec<u8>,
}

/// Host-issued permission for one exact binding before an exclusive deadline.
/// This type must never be deserialized from model/tool output as authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permit {
    /// Exact prepared operation approved by the host/user.
    pub request: Request,
    /// Exclusive deadline in the host session's monotonic time domain.
    pub expires: u64,
    /// Revoked or consumed permission cannot authorize another operation.
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
    fn exact_session_request_deadline_and_active_permission_are_required() {
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
            assert!(!allowed(0, b"session-a", &permit, &changed));
        }
        assert!(!allowed(0, b"foreign", &permit, &expected));
        assert!(!allowed(10, b"session-a", &permit, &expected));
        assert!(allowed(9, b"session-a", &permit, &expected));
        let revoked = Permit {
            spent: true,
            ..permit
        };
        assert!(!allowed(0, b"session-a", &revoked, &expected));
    }
}
