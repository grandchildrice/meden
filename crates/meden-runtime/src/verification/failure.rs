use std::fmt;
use std::io;

use serde::Serialize;

use crate::process::Termination;

/// Host-attributed check failure, never decoded from model/result text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    /// A completed check did not establish the candidate's contract. This is
    /// not a theorem that the contract is false; proof automation can fail too.
    Unverified,
    /// Prepared resources, host I/O or the checker protocol are unavailable.
    Unavailable,
    /// Cancellation or a resource limit prevented a conclusive check.
    Inconclusive,
}

impl FailureKind {
    /// Only a completed rejection may trigger automatic source repair.
    pub fn repairable(self) -> bool {
        self == Self::Unverified
    }
}

/// Operational failures stop automatic repair until a new host instruction.
pub fn verification_stops(failure: Option<FailureKind>) -> bool {
    matches!(failure, Some(kind) if !kind.repairable())
}

/// Classify an unsuccessful worker from process facts, not its diagnostic text.
#[allow(clippy::verbose_bit_mask)] // Keep the Unix wait-word protocol explicit.
pub fn failed_worker(termination: &Termination, truncated: bool) -> FailureKind {
    if truncated {
        FailureKind::Inconclusive
    } else {
        match termination {
            Termination::Exited(status) => {
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt as _;
                    // The Unix wait word records a signal in its low 7 bits.
                    if status.into_raw() & 127 == 0 {
                        FailureKind::Unverified
                    } else {
                        FailureKind::Inconclusive
                    }
                }
                #[cfg(not(unix))]
                {
                    let _ = status;
                    FailureKind::Inconclusive
                }
            }
            Termination::TimedOut | Termination::Cancelled | Termination::OutputLimit => {
                FailureKind::Inconclusive
            }
        }
    }
}

/// Typed failure plus bounded human-readable diagnostic. The message grants
/// no authority and is not used to determine the failure kind.
#[derive(Clone, Debug)]
pub struct Failure {
    /// Trusted adapter classification, independent of diagnostic wording.
    pub kind: FailureKind,
    message: String,
}

impl Failure {
    /// Attach display text to an already classified failure.
    pub fn new(kind: FailureKind, message: impl fmt::Display) -> Self {
        Self {
            kind,
            message: message.to_string(),
        }
    }

    /// Mark a host resource or protocol error as unavailable.
    pub fn unavailable(message: impl fmt::Display) -> Self {
        Self::new(FailureKind::Unavailable, message)
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl std::error::Error for Failure {}

impl From<io::Error> for Failure {
    fn from(error: io::Error) -> Self {
        Self::unavailable(error)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt as _;
    use std::process::ExitStatus;

    #[test]
    fn worker_limits_and_signals_are_not_repairable() {
        for termination in [
            Termination::Exited(ExitStatus::from_raw(101 << 8)),
            Termination::Exited(ExitStatus::from_raw(9)),
            Termination::TimedOut,
            Termination::Cancelled,
            Termination::OutputLimit,
        ] {
            assert_eq!(failed_worker(&termination, true), FailureKind::Inconclusive);
            assert_eq!(
                failed_worker(&termination, false).repairable(),
                matches!(termination, Termination::Exited(code) if code.code().is_some())
            );
        }
        assert!(!FailureKind::Unavailable.repairable());
    }
}
