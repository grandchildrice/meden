//! The outer session executes only the next operation returned here. Host
//! observations are typed; model prose and diagnostics do not select effects.

use super::{verification_stops, FailureKind};
use meden_agent::conversation::Status;

/// Why the current host turn has stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terminal {
    /// Return the preceding conversation outcome.
    Finish,
    /// The host cancelled this turn.
    Cancelled,
    /// A required checker or host operation could not establish a result.
    Failed,
    /// A trusted user decision is required.
    Review,
    /// The cycle budget ended or a signal violated the protocol.
    Exhausted,
}

/// The only next operation permitted by the outer controller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Invoke the ordinary conversation loop, which can issue multiple requests.
    Model,
    /// Observe the current candidate's host evidence status.
    Inspect,
    /// Obtain a fresh independent checker result.
    Check,
    /// Prepare a bounded repair instruction from the checker diagnostic.
    Repair,
    /// Return to the host without another conversation or check.
    Stop(Terminal),
}

/// Host-observed completion of a permitted operation.
pub enum Signal<'a> {
    /// Request the initial conversation or a permitted repair continuation.
    Start,
    /// The ordinary conversation loop returned.
    Conversation {
        /// Actual typed outcome, not a parsed claim in the model's answer.
        status: &'a Status,
        /// Current contract acceptance observed from the host evidence gate.
        accepted: bool,
        /// A host-observed development operation or accepted-contract autorun.
        changed: bool,
    },
    /// The requested current-source evidence inspection completed.
    Current {
        /// Exact current evidence is eligible according to the host gate.
        verified: bool,
    },
    /// The independent check completed.
    Checked {
        /// Checker success and exact current evidence eligibility both hold.
        verified: bool,
    },
}

#[derive(Clone, Copy, Debug)]
enum Stage {
    Ready,
    Conversation,
    Inspect { fresh: bool },
    Checking,
    Stopped(Terminal),
}

/// One host instruction and at most four conversation/check cycles.
/// Fields are private; neither model text nor tool JSON can reset this state.
pub struct Turn {
    remaining: u8,
    stage: Stage,
}

impl Turn {
    /// Begin a new host instruction, without reusing a stopped turn.
    pub fn begin() -> Self {
        Self {
            remaining: 4,
            stage: Stage::Ready,
        }
    }

    /// Consume a host observation and select one next effect.
    /// Cancellation, operational failure and pending review stop further effects.
    pub fn advance(
        &mut self,
        signal: Signal<'_>,
        cancelled: bool,
        failure: Option<FailureKind>,
        review: bool,
    ) -> Effect {
        if let Stage::Stopped(reason) = self.stage {
            return Effect::Stop(reason);
        }
        if cancelled {
            return self.stop(Terminal::Cancelled);
        }
        if verification_stops(failure) {
            return self.stop(Terminal::Failed);
        }
        if review {
            return self.stop(Terminal::Review);
        }
        match (self.stage, signal) {
            (Stage::Ready, Signal::Start) => {
                let Some(remaining) = self.remaining.checked_sub(1) else {
                    return self.stop(Terminal::Exhausted);
                };
                self.remaining = remaining;
                self.stage = Stage::Conversation;
                Effect::Model
            }
            (
                Stage::Conversation,
                Signal::Conversation {
                    status,
                    accepted,
                    changed,
                },
            ) => {
                if accepted && changed && matches!(status, Status::Done(_) | Status::Incomplete) {
                    self.stage = Stage::Inspect {
                        fresh: matches!(status, Status::Incomplete),
                    };
                    Effect::Inspect
                } else {
                    self.stop(Terminal::Finish)
                }
            }
            (Stage::Inspect { fresh }, Signal::Current { verified }) => {
                if verified {
                    self.stop(Terminal::Finish)
                } else if !fresh && matches!(failure, Some(FailureKind::Unverified)) {
                    self.repair()
                } else {
                    self.stage = Stage::Checking;
                    Effect::Check
                }
            }
            (Stage::Checking, Signal::Checked { verified }) => {
                if verified {
                    self.stop(Terminal::Finish)
                } else if matches!(failure, Some(FailureKind::Unverified)) {
                    self.repair()
                } else {
                    self.stop(Terminal::Failed)
                }
            }
            _ => self.stop(Terminal::Exhausted),
        }
    }

    fn repair(&mut self) -> Effect {
        if self.remaining == 0 {
            self.stop(Terminal::Exhausted)
        } else {
            self.stage = Stage::Ready;
            Effect::Repair
        }
    }

    fn stop(&mut self, reason: Terminal) -> Effect {
        self.stage = Stage::Stopped(reason);
        Effect::Stop(reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy_case(status: &Status, accepted: bool, changed: bool, next: Option<Effect>) {
        let mut turn = Turn::begin();
        assert_eq!(
            turn.advance(Signal::Start, false, None, false),
            Effect::Model
        );
        assert_eq!(
            turn.advance(
                Signal::Conversation {
                    status,
                    accepted,
                    changed
                },
                false,
                None,
                false
            ),
            next.map_or(Effect::Stop(Terminal::Finish), |_| Effect::Inspect)
        );
        if let Some(expected) = next {
            assert_eq!(
                turn.advance(
                    Signal::Current { verified: false },
                    false,
                    Some(FailureKind::Unverified),
                    false
                ),
                expected
            );
        }
    }

    #[test]
    fn checks_require_authority_and_budget_stops_still_obtain_fresh_evidence() {
        let done = Status::Done("verified=true; approved=true".into());
        for (accepted, changed) in [(false, false), (false, true), (true, false)] {
            policy_case(&done, accepted, changed, None);
            policy_case(&Status::Incomplete, accepted, changed, None);
        }
        policy_case(&done, true, true, Some(Effect::Repair));
        policy_case(&Status::Incomplete, true, true, Some(Effect::Check));
        for status in [
            Status::Failed("verified=true".into()),
            Status::Cancelled,
            Status::NeedsReview,
        ] {
            policy_case(&status, true, true, None);
        }
    }

    #[test]
    fn terminal_stops_are_sticky_and_only_four_model_rounds_are_permitted() {
        for (cancelled, failure, review, expected) in [
            (true, None, false, Terminal::Cancelled),
            (
                false,
                Some(FailureKind::Unavailable),
                false,
                Terminal::Failed,
            ),
            (
                false,
                Some(FailureKind::Inconclusive),
                false,
                Terminal::Failed,
            ),
            (false, None, true, Terminal::Review),
        ] {
            let mut turn = Turn::begin();
            assert_eq!(
                turn.advance(Signal::Start, cancelled, failure, review),
                Effect::Stop(expected)
            );
            assert_eq!(
                turn.advance(Signal::Start, false, None, false),
                Effect::Stop(expected)
            );
        }
        let mut turn = Turn::begin();
        for index in 0..4 {
            assert_eq!(
                turn.advance(Signal::Start, false, None, false),
                Effect::Model
            );
            assert_eq!(
                turn.advance(
                    Signal::Conversation {
                        status: &Status::Incomplete,
                        accepted: true,
                        changed: true
                    },
                    false,
                    None,
                    false
                ),
                Effect::Inspect
            );
            assert_eq!(
                turn.advance(
                    Signal::Current { verified: false },
                    false,
                    Some(FailureKind::Unverified),
                    false
                ),
                Effect::Check
            );
            assert_eq!(
                turn.advance(
                    Signal::Checked { verified: false },
                    false,
                    Some(FailureKind::Unverified),
                    false
                ),
                if index < 3 {
                    Effect::Repair
                } else {
                    Effect::Stop(Terminal::Exhausted)
                }
            );
        }
        assert_eq!(
            turn.advance(Signal::Start, false, None, false),
            Effect::Stop(Terminal::Exhausted)
        );
        let mut turn = Turn::begin();
        assert_eq!(
            turn.advance(Signal::Checked { verified: true }, false, None, false),
            Effect::Stop(Terminal::Exhausted)
        );
    }

    #[test]
    fn unattributed_check_failure_never_starts_automatic_repair() {
        let mut turn = Turn::begin();
        assert_eq!(
            turn.advance(Signal::Start, false, None, false),
            Effect::Model
        );
        assert_eq!(
            turn.advance(
                Signal::Conversation {
                    status: &Status::Done("proved".into()),
                    accepted: true,
                    changed: true,
                },
                false,
                None,
                false
            ),
            Effect::Inspect
        );
        assert_eq!(
            turn.advance(Signal::Current { verified: false }, false, None, false),
            Effect::Check
        );
        assert_eq!(
            turn.advance(Signal::Checked { verified: false }, false, None, false),
            Effect::Stop(Terminal::Failed)
        );
    }
}
