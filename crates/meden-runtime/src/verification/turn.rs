//! The outer session executes only the next operation returned here. Host
//! observations are typed; model prose and diagnostics do not select effects.

use super::{verification_stops, FailureKind};

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
pub enum Signal {
    /// Request the initial conversation or a permitted repair continuation.
    Start,
    /// The ordinary conversation loop returned.
    Conversation {
        /// Typed conversation classification; `Some(true)` requires a fresh check.
        inspection: Option<bool>,
        /// Current contract acceptance observed from the host evidence gate.
        accepted: bool,
        /// A host-observed development operation or accepted-contract autorun.
        changed: bool,
        /// A proposal tool failed during this conversation. File edits cannot
        /// clear this observation or make the failed draft accepted.
        proposal_failed: bool,
    },
    /// The requested current-source evidence inspection completed.
    Current {
        /// Exact evidence eligibility, or a classified source-inspection failure.
        result: Result<bool, FailureKind>,
    },
    /// The independent check completed.
    Checked {
        /// Exact evidence eligibility after checking, or the check/probe failure.
        result: Result<bool, FailureKind>,
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
        signal: Signal,
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
                    inspection,
                    accepted,
                    changed,
                    proposal_failed,
                },
            ) => {
                if accepted && changed {
                    if let Some(fresh) = inspection {
                        self.stage = Stage::Inspect { fresh };
                        Effect::Inspect
                    } else {
                        self.stop(Terminal::Finish)
                    }
                } else if !accepted && proposal_failed && inspection.is_some() {
                    self.repair()
                } else {
                    self.stop(Terminal::Finish)
                }
            }
            (Stage::Inspect { fresh }, Signal::Current { result }) => match result {
                Ok(true) => self.stop(Terminal::Finish),
                Ok(false) if !fresh && matches!(failure, Some(FailureKind::Unverified)) => {
                    self.repair()
                }
                Ok(false) | Err(FailureKind::Unverified) => {
                    self.stage = Stage::Checking;
                    Effect::Check
                }
                Err(FailureKind::Unavailable | FailureKind::Inconclusive) => {
                    self.stop(Terminal::Failed)
                }
            },
            (Stage::Checking, Signal::Checked { result }) => match result {
                Ok(true) => self.stop(Terminal::Finish),
                Err(FailureKind::Unverified) => self.repair(),
                Ok(false) | Err(FailureKind::Unavailable | FailureKind::Inconclusive) => {
                    self.stop(Terminal::Failed)
                }
            },
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
    use meden_agent::conversation::Status;

    fn policy_case(status: &Status, accepted: bool, changed: bool, next: Option<Effect>) {
        let mut turn = Turn::begin();
        assert_eq!(
            turn.advance(Signal::Start, false, None, false),
            Effect::Model
        );
        assert_eq!(
            turn.advance(
                Signal::Conversation {
                    inspection: status.inspection(),
                    accepted,
                    changed,
                    proposal_failed: false,
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
                    Signal::Current { result: Ok(false) },
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
    fn proposal_repair_requires_a_failed_proposal_and_shares_the_turn_budget() {
        for (inspection, accepted, changed, proposal_failed, expected) in [
            // Completed and exhausted proposal attempts can recover even after
            // file edits cleared an earlier implementation-failure diagnostic.
            (Some(false), false, false, true, Effect::Repair),
            (Some(true), false, true, true, Effect::Repair),
            // No observation, failed transport, and accepted unchanged work
            // cannot be redirected into proposal repair.
            (
                Some(false),
                false,
                true,
                false,
                Effect::Stop(Terminal::Finish),
            ),
            (
                Some(true),
                false,
                true,
                false,
                Effect::Stop(Terminal::Finish),
            ),
            (None, false, true, true, Effect::Stop(Terminal::Finish)),
            (
                Some(false),
                true,
                false,
                true,
                Effect::Stop(Terminal::Finish),
            ),
            // A failed replacement does not displace accepted-source checking.
            (Some(false), true, true, true, Effect::Inspect),
        ] {
            let mut turn = Turn {
                remaining: 2,
                stage: Stage::Conversation,
            };
            assert_eq!(
                turn.advance(
                    Signal::Conversation {
                        inspection,
                        accepted,
                        changed,
                        proposal_failed,
                    },
                    false,
                    None,
                    false,
                ),
                expected
            );
        }
        let mut turn = Turn::begin();
        for round in 0..4 {
            assert_eq!(
                turn.advance(Signal::Start, false, None, false),
                Effect::Model
            );
            assert_eq!(
                turn.advance(
                    Signal::Conversation {
                        inspection: Some(round % 2 == 0),
                        accepted: false,
                        changed: true,
                        proposal_failed: true,
                    },
                    false,
                    None,
                    false,
                ),
                if round < 3 {
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
                        inspection: Status::Incomplete.inspection(),
                        accepted: true,
                        changed: true,
                        proposal_failed: false,
                    },
                    false,
                    None,
                    false
                ),
                Effect::Inspect
            );
            assert_eq!(
                turn.advance(
                    Signal::Current { result: Ok(false) },
                    false,
                    Some(FailureKind::Unverified),
                    false
                ),
                Effect::Check
            );
            assert_eq!(
                turn.advance(
                    Signal::Checked {
                        result: Err(FailureKind::Unverified)
                    },
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
            turn.advance(Signal::Checked { result: Ok(true) }, false, None, false),
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
                    inspection: Status::Done("proved".into()).inspection(),
                    accepted: true,
                    changed: true,
                    proposal_failed: false,
                },
                false,
                None,
                false
            ),
            Effect::Inspect
        );
        assert_eq!(
            turn.advance(Signal::Current { result: Ok(false) }, false, None, false),
            Effect::Check
        );
        assert_eq!(
            turn.advance(
                Signal::Checked { result: Ok(false) },
                false,
                Some(FailureKind::Unverified),
                false
            ),
            Effect::Stop(Terminal::Failed)
        );
    }

    #[test]
    fn failed_source_probes_cannot_reuse_an_old_rejection() {
        for fresh in [false, true] {
            for kind in [FailureKind::Unavailable, FailureKind::Inconclusive] {
                for failure in [None, Some(FailureKind::Unverified)] {
                    let mut turn = Turn {
                        remaining: 3,
                        stage: Stage::Inspect { fresh },
                    };
                    assert_eq!(
                        turn.advance(Signal::Current { result: Err(kind) }, false, failure, false),
                        Effect::Stop(Terminal::Failed)
                    );
                    assert_eq!(
                        turn.advance(Signal::Start, false, None, false),
                        Effect::Stop(Terminal::Failed)
                    );
                    let mut turn = Turn {
                        remaining: 3,
                        stage: Stage::Checking,
                    };
                    assert_eq!(
                        turn.advance(Signal::Checked { result: Err(kind) }, false, failure, false),
                        Effect::Stop(Terminal::Failed)
                    );
                }
            }
            let mut turn = Turn {
                remaining: 3,
                stage: Stage::Inspect { fresh },
            };
            assert_eq!(
                turn.advance(
                    Signal::Current {
                        result: Err(FailureKind::Unverified)
                    },
                    false,
                    Some(FailureKind::Unverified),
                    false
                ),
                Effect::Check
            );
        }
    }
}
