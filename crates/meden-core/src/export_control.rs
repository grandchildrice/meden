//! Pure export-review control kernel, extracted and refined in Lean.

use crate::transfer::{self, Permit, Request};

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
pub enum Event {
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
pub fn step(phase: Phase, origin: Origin, event: Event) -> Decision {
    let next = match (phase, origin, event) {
        (Phase::Pending, Origin::Model, Event::Propose) => Phase::Pending,
        (Phase::Pending, Origin::Host, Event::Review) => Phase::Reviewed,
        (_, Origin::Host, Event::Review) => phase,
        (Phase::Pending | Phase::Reviewed, Origin::Host, Event::Reject) => Phase::Rejected,
        (Phase::Reviewed, Origin::Host, Event::Approve) => {
            return Decision {
                phase: Phase::Attempted,
                accepted: true,
                dispatch: true,
            };
        }
        (Phase::Attempted, Origin::Transport, Event::ResponseReceived) => Phase::ResponseReceived,
        (Phase::Attempted, Origin::Transport, Event::DeliveryUncertain) => Phase::DeliveryUncertain,
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
    request: Request,
}

/// The executor receives the permitted request itself, rather than choosing
/// bytes after a Boolean authorization. This type has no deserializer.
#[derive(Debug)]
pub struct Outcome {
    /// Whether the review/lifecycle operation was valid.
    pub accepted: bool,
    /// Exact immutable inputs for one authorized effect, if any.
    pub effect: Option<Request>,
}

impl State {
    /// A model proposal starts without review or dispatch authority.
    pub fn new(request: Request) -> Self {
        Self {
            phase: Phase::Pending,
            request,
        }
    }

    /// Host presentation of the same request used for dispatch.
    pub fn request(&self) -> &Request {
        &self.request
    }

    /// Host presentation of the kernel's lifecycle, without a cached Boolean.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Commit the review transition before emitting any effect. A host
    /// approval creates and spends one exact permit in this same transition;
    /// even a refused/uncertain attempt cannot recreate dispatch authority.
    pub fn advance(
        &mut self,
        origin: Origin,
        event: Event,
        now: u64,
        expires: u64,
        session: &[u8],
    ) -> Outcome {
        let decision = step(self.phase, origin, event);
        self.phase = decision.phase;
        let effect = if decision.dispatch {
            let mut permit = Permit {
                request: self.request.clone(),
                expires,
                spent: false,
            };
            if transfer::reserve(now, session, &mut permit, &self.request) {
                Some(self.request.clone())
            } else {
                None
            }
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
    use super::*;

    #[test]
    fn effects_are_frozen_reviewed_session_bound_and_single_use() {
        let request = Request {
            session: b"session".to_vec(),
            recipient: b"https://approved.example/path".to_vec(),
            request: b"complete frozen request".to_vec(),
        };
        for origin in [Origin::Model, Origin::Host, Origin::Transport] {
            let mut state = State::new(request.clone());
            assert_eq!(state.phase(), Phase::Pending);
            assert!(
                !state
                    .advance(Origin::Model, Event::Review, 0, 0, b"session")
                    .accepted
            );
            assert!(state
                .advance(Origin::Host, Event::Approve, 0, 10, b"session")
                .effect
                .is_none());
            assert!(
                state
                    .advance(Origin::Host, Event::Review, 0, 0, b"session")
                    .accepted
            );
            let out = state.advance(origin, Event::Approve, 9, 10, b"session");
            if origin == Origin::Host {
                assert_eq!(out.effect, Some(request.clone()));
                assert_eq!(state.phase(), Phase::Attempted);
                state.advance(
                    Origin::Transport,
                    Event::DeliveryUncertain,
                    0,
                    0,
                    b"session",
                );
                state.advance(Origin::Host, Event::Review, 0, 0, b"session");
                assert!(state
                    .advance(Origin::Host, Event::Approve, 9, 10, b"session")
                    .effect
                    .is_none());
            } else {
                assert!(out.effect.is_none());
                assert_eq!(state.phase(), Phase::Reviewed);
            }
            assert_eq!(state.request(), &request);
        }
        for (now, session) in [(10, b"session".as_slice()), (9, b"foreign".as_slice())] {
            let mut state = State::new(request.clone());
            state.advance(Origin::Host, Event::Review, 0, 0, b"session");
            assert!(state
                .advance(Origin::Host, Event::Approve, now, 10, session)
                .effect
                .is_none());
            assert_eq!(state.phase(), Phase::Attempted);
            assert!(state
                .advance(Origin::Host, Event::Approve, 9, 10, b"session")
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
                let result = step(phase, origin, Event::Approve);
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
                    Event::Propose,
                    Event::Review,
                    Event::Approve,
                    Event::ResponseReceived,
                    Event::DeliveryUncertain,
                    Event::Reject,
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
