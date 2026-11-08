//! Pure export-review control kernel, extracted and refined in Lean.

/// Lifecycle of one immutable proposal. Terminal states cannot be reopened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Prepared by a model tool; no authority to send.
    Pending,
    /// Exposed through a trusted host review operation.
    Reviewed,
    /// Declined by the host.
    Rejected,
    /// Dispatch reserved before any external effect.
    Attempted,
    /// A response arrived, without granting another attempt.
    ResponseReceived,
    /// Delivery may have occurred; no retry is authorized.
    DeliveryUncertain,
}

impl Phase {
    /// Presentation only; authority uses the enum, not these labels.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Reviewed => "reviewed",
            Self::Rejected => "rejected",
            Self::Attempted => "attempted",
            Self::ResponseReceived => "response_received",
            Self::DeliveryUncertain => "delivery_uncertain",
        }
    }
}

/// Origin assigned by the host call site, never parsed from model arguments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Untrusted model proposal.
    Model,
    /// Trusted user/controller channel.
    Host,
    /// Result of the host's actual transport call.
    Transport,
}

/// Proposed control operation; its origin is checked independently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportEvent {
    /// Initialize a new prepared proposal.
    Propose,
    /// Inspect exact immutable inputs.
    Review,
    /// Decline the proposal.
    Reject,
    /// Request a single dispatch after review.
    Approve,
    /// Actual transport returned a response.
    ResponseReceived,
    /// Actual transport failed or delivery is uncertain.
    DeliveryUncertain,
}

/// Complete transition result, including the only authority to dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decision {
    /// Updated control state, stored before dispatch.
    pub phase: Phase,
    /// Whether this operation is valid in this state and origin.
    pub accepted: bool,
    /// True exactly for the first host approval of a reviewed proposal.
    pub dispatch: bool,
}

/// Denial preserves the state and emits no dispatch. A model/transport event
/// cannot act as a host approval, and review cannot reopen a terminal state.
pub fn step(phase: Phase, origin: Origin, event: ExportEvent) -> Decision {
    let next = match (phase, origin, event) {
        (Phase::Pending, Origin::Model, ExportEvent::Propose) => Phase::Pending,
        (Phase::Pending, Origin::Host, ExportEvent::Review) => Phase::Reviewed,
        (_, Origin::Host, ExportEvent::Review) => phase,
        (Phase::Pending | Phase::Reviewed, Origin::Host, ExportEvent::Reject) => Phase::Rejected,
        (Phase::Reviewed, Origin::Host, ExportEvent::Approve) => {
            return Decision {
                phase: Phase::Attempted,
                accepted: true,
                dispatch: true,
            };
        }
        (Phase::Attempted, Origin::Transport, ExportEvent::ResponseReceived) => {
            Phase::ResponseReceived
        }
        (Phase::Attempted, Origin::Transport, ExportEvent::DeliveryUncertain) => {
            Phase::DeliveryUncertain
        }
        _ => {
            return Decision {
                phase,
                accepted: false,
                dispatch: false,
            }
        }
    };
    Decision {
        phase: next,
        accepted: true,
        dispatch: false,
    }
}

/// One frozen request and its review/reservation state. No operation replaces
/// the request or reopens an attempted proposal.
#[derive(Debug)]
pub struct State {
    phase: Phase,
    session: Vec<u8>,
    request: FrozenRequest,
}

/// Owned destination and bytes, with no mutable access or interior mutability.
#[derive(Debug)]
pub struct FrozenRequest {
    target: String,
    body: Vec<u8>,
}

impl FrozenRequest {
    /// Destination supplied at preparation, without reparsing a description.
    pub fn target(&self) -> &String {
        &self.target
    }

    /// Original body bytes supplied at preparation.
    pub fn body(&self) -> &[u8] {
        &self.body
    }
}

/// A single dispatch borrowing the original frozen request. Its private
/// constructor, absence of `Clone` and consuming accessor prevent callers
/// from manufacturing or duplicating dispatch authority.
///
/// ```compile_fail
/// use meden_core::export_control::{ExportEvent, Origin, State};
/// let mut state = State::new(b"session".to_vec(), "https://example.test/".into(), vec![42]);
/// state.advance(Origin::Host, ExportEvent::Review, 0, 0, b"session");
/// let effect = state.advance(Origin::Host, ExportEvent::Approve, 0, 1, b"session")
///     .effect.unwrap();
/// let duplicate = effect.clone();
/// ```
#[derive(Debug)]
pub struct Dispatch<'a> {
    request: &'a FrozenRequest,
}

impl<'a> Dispatch<'a> {
    /// Consume the authorization to access its exact immutable request.
    pub fn into_request(self) -> &'a FrozenRequest {
        self.request
    }
}

/// Complete lifecycle result. Only the kernel constructs a dispatch.
#[derive(Debug)]
pub struct Outcome<'a> {
    /// Whether the review/lifecycle operation was valid.
    pub accepted: bool,
    /// Exact immutable inputs for one authorized effect, if any.
    pub effect: Option<Dispatch<'a>>,
}

impl State {
    /// A model proposal starts without review or dispatch authority.
    pub fn new(session: Vec<u8>, target: String, body: Vec<u8>) -> Self {
        Self {
            phase: Phase::Pending,
            session,
            request: FrozenRequest { target, body },
        }
    }

    /// Host presentation of the same request used for dispatch.
    pub fn request(&self) -> &FrozenRequest {
        &self.request
    }

    /// Host presentation of the kernel's lifecycle, without a cached Boolean.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Commit the review transition before emitting any effect. A host
    /// approval borrows the original request in this same transition;
    /// even a refused/uncertain attempt cannot recreate dispatch authority.
    pub fn advance(
        &mut self,
        origin: Origin,
        event: ExportEvent,
        now: u64,
        expires: u64,
        session: &[u8],
    ) -> Outcome<'_> {
        let decision = step(self.phase, origin, event);
        self.phase = decision.phase;
        let request_session: &[u8] = &self.session;
        let effect = if decision.dispatch && request_session == session && now < expires {
            Some(Dispatch {
                request: &self.request,
            })
        } else {
            None
        };
        Outcome {
            accepted: decision.accepted,
            effect,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    fn state() -> State {
        State::new(
            b"session".to_vec(),
            "https://approved.example/path".into(),
            b"complete frozen request".to_vec(),
        )
    }

    #[test]
    fn effects_are_frozen_reviewed_session_bound_and_single_use() {
        for origin in [Origin::Model, Origin::Host, Origin::Transport] {
            let mut state = state();
            assert_eq!(state.phase(), Phase::Pending);
            assert!(
                !state
                    .advance(Origin::Model, ExportEvent::Review, 0, 0, b"session")
                    .accepted
            );
            assert!(state
                .advance(Origin::Host, ExportEvent::Approve, 0, 10, b"session")
                .effect
                .is_none());
            assert!(
                state
                    .advance(Origin::Host, ExportEvent::Review, 0, 0, b"session")
                    .accepted
            );
            let out = state.advance(origin, ExportEvent::Approve, 9, 10, b"session");
            if origin == Origin::Host {
                let request = out.effect.expect("authorized").into_request();
                assert_eq!(request.target(), "https://approved.example/path");
                assert_eq!(request.body(), b"complete frozen request");
                assert_eq!(state.phase(), Phase::Attempted);
                state.advance(
                    Origin::Transport,
                    ExportEvent::DeliveryUncertain,
                    0,
                    0,
                    b"session",
                );
                state.advance(Origin::Host, ExportEvent::Review, 0, 0, b"session");
                assert!(state
                    .advance(Origin::Host, ExportEvent::Approve, 9, 10, b"session")
                    .effect
                    .is_none());
            } else {
                assert!(out.effect.is_none());
                assert_eq!(state.phase(), Phase::Reviewed);
            }
            assert_eq!(state.request().target(), "https://approved.example/path");
            assert_eq!(state.request().body(), b"complete frozen request");
        }
        for (now, session) in [(10, b"session".as_slice()), (9, b"foreign".as_slice())] {
            let mut state = state();
            state.advance(Origin::Host, ExportEvent::Review, 0, 0, b"session");
            assert!(state
                .advance(Origin::Host, ExportEvent::Approve, now, 10, session)
                .effect
                .is_none());
            assert_eq!(state.phase(), Phase::Attempted);
            assert!(state
                .advance(Origin::Host, ExportEvent::Approve, 9, 10, b"session")
                .effect
                .is_none());
        }
    }

    #[test]
    fn dispatch_requires_review_host_origin_and_a_fresh_attempt() {
        for origin in [Origin::Model, Origin::Host, Origin::Transport] {
            for phase in [
                Phase::Pending,
                Phase::Reviewed,
                Phase::Rejected,
                Phase::Attempted,
                Phase::ResponseReceived,
                Phase::DeliveryUncertain,
            ] {
                let result = step(phase, origin, ExportEvent::Approve);
                assert_eq!(
                    result.dispatch,
                    phase == Phase::Reviewed && origin == Origin::Host
                );
                if result.dispatch {
                    assert_eq!(result.phase, Phase::Attempted);
                }
            }
        }
        for initial in [
            Phase::Pending,
            Phase::Reviewed,
            Phase::Rejected,
            Phase::Attempted,
            Phase::ResponseReceived,
            Phase::DeliveryUncertain,
        ] {
            let mut phase = initial;
            let mut attempts = 0;
            for _ in 0..4 {
                for event in [
                    ExportEvent::Propose,
                    ExportEvent::Review,
                    ExportEvent::Approve,
                    ExportEvent::ResponseReceived,
                    ExportEvent::DeliveryUncertain,
                    ExportEvent::Reject,
                ] {
                    for origin in [Origin::Model, Origin::Host, Origin::Transport] {
                        let result = step(phase, origin, event);
                        phase = result.phase;
                        if result.dispatch {
                            attempts += 1;
                        }
                        assert!(attempts <= 1, "a resolved proposal was reopened");
                    }
                }
            }
        }
    }
}
