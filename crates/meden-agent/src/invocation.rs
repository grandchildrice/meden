//! One prepared implementation/argument pair, notified before dispatch.
//! JSON decoding and capability validation supply the frozen preparation;
//! this controller neither resolves tools nor changes their arguments.

use crate::conversation::{ExecutionError, Status};
use crate::flow::interrupt;

#[derive(Clone, Copy)]
pub(crate) enum Rule {
    UnknownTool,
    InvalidArguments,
    MissingArgument,
    Policy,
}

impl Rule {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::UnknownTool => "unknown_tool",
            Self::InvalidArguments => "invalid_arguments",
            Self::MissingArgument => "missing_argument",
            Self::Policy => "policy",
        }
    }
}

pub(crate) struct Refusal {
    pub(crate) rule: Rule,
    pub(crate) reason: String,
}

pub(crate) enum Notice<'a> {
    Allowed,
    Refused(&'a Refusal),
    Invalid,
}

/// No cloning, mutable payload access or replacement after preparation.
pub(crate) struct Attempt<T> {
    checked: Option<Result<T, Refusal>>,
    announced: bool,
}

impl<T> Attempt<T> {
    pub(crate) fn new(checked: Result<T, Refusal>) -> Self {
        Self {
            checked: Some(checked),
            announced: false,
        }
    }

    pub(crate) fn announce(&mut self) -> Notice<'_> {
        if self.announced {
            self.checked = None;
            return Notice::Invalid;
        }
        self.announced = true;
        match &self.checked {
            Some(Ok(_)) => Notice::Allowed,
            Some(Err(refusal)) => Notice::Refused(refusal),
            None => Notice::Invalid,
        }
    }

    pub(crate) fn dispatch(
        &mut self,
        cancelled: bool,
        host_stop: Option<Status>,
    ) -> Result<T, ExecutionError> {
        let checked = self.checked.take();
        if !self.announced {
            return Err(ExecutionError::Stopped(Status::Incomplete));
        }
        match checked {
            Some(Ok(request)) => match interrupt(cancelled, host_stop) {
                None => Ok(request),
                Some(status) => Err(ExecutionError::Stopped(status)),
            },
            Some(Err(refusal)) => Err(ExecutionError::Refused(refusal.reason)),
            None => Err(ExecutionError::Stopped(Status::Incomplete)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_stop_exact_payload_and_consumption_share_one_controller() {
        let mut attempt = Attempt::new(Ok(("original implementation", "frozen arguments")));
        assert!(matches!(attempt.announce(), Notice::Allowed));
        assert!(matches!(
            attempt.dispatch(false, None),
            Ok(("original implementation", "frozen arguments"))
        ));
        assert!(matches!(
            attempt.dispatch(false, None),
            Err(ExecutionError::Stopped(Status::Incomplete))
        ));
        for (cancelled, stop, expected) in [
            (true, None, Status::Cancelled),
            (true, Some(Status::NeedsReview), Status::Cancelled),
            (false, Some(Status::NeedsReview), Status::NeedsReview),
            (
                false,
                Some(Status::Failed("checker unavailable".into())),
                Status::Failed("checker unavailable".into()),
            ),
        ] {
            let mut attempt = Attempt::new(Ok("frozen"));
            assert!(matches!(attempt.announce(), Notice::Allowed));
            assert!(
                matches!(attempt.dispatch(cancelled, stop), Err(ExecutionError::Stopped(status)) if status == expected)
            );
            assert!(matches!(
                attempt.dispatch(false, None),
                Err(ExecutionError::Stopped(Status::Incomplete))
            ));
        }
        let mut skipped = Attempt::new(Ok("never notified"));
        assert!(matches!(
            skipped.dispatch(false, None),
            Err(ExecutionError::Stopped(Status::Incomplete))
        ));
        assert!(matches!(skipped.announce(), Notice::Invalid));
        assert!(skipped.dispatch(false, None).is_err());
        let mut repeated = Attempt::new(Ok("invalid protocol"));
        assert!(matches!(repeated.announce(), Notice::Allowed));
        assert!(matches!(repeated.announce(), Notice::Invalid));
        assert!(repeated.dispatch(false, None).is_err());
        let mut refused: Attempt<&str> = Attempt::new(Err(Refusal {
            rule: Rule::Policy,
            reason: "original refusal".into(),
        }));
        assert!(
            matches!(refused.announce(), Notice::Refused(r) if matches!(r.rule, Rule::Policy) && r.reason == "original refusal")
        );
        assert!(
            matches!(refused.dispatch(false, None), Err(ExecutionError::Refused(reason)) if reason == "original refusal")
        );
        assert!(refused.dispatch(false, None).is_err());
    }
}
