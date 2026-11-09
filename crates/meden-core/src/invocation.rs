//! Policy admission, notification and one-use dispatch of a frozen request.

use crate::{allowed, Policy, Requirement};

/// A notification about one frozen preparation.
pub enum Notice<'a, F> {
    /// The declared requirements were admitted by the policy.
    Allowed,
    /// Preparation or policy admission failed.
    Refused(&'a F),
    /// Preparation was consumed or the notification protocol was repeated.
    Invalid,
}

/// A dispatch failure, without interpreting host-specific status values.
/// The private standard sum preserves four disjoint outcomes across adapters.
#[derive(Debug, PartialEq, Eq)]
pub struct Error<F, S>(Result<Option<S>, Option<F>>);

impl<F, S> Error<F, S> {
    /// The original preparation or policy refusal.
    pub fn refused(refusal: F) -> Self {
        Self(Err(Some(refusal)))
    }
    /// Cancellation observed after notification.
    pub fn cancelled() -> Self {
        Self(Ok(None))
    }
    /// A host stop observed after notification.
    pub fn stopped(status: S) -> Self {
        Self(Ok(Some(status)))
    }
    /// Missing notification or preparation already consumed.
    pub fn invalid() -> Self {
        Self(Err(None))
    }
    /// Consume the failure: Ok(None) is cancellation, Ok(Some) a host stop,
    /// Err(Some) a refusal and Err(None) an invalid invocation protocol.
    pub fn into_parts(self) -> Result<Option<S>, Option<F>> {
        self.0
    }
}

/// A request can only enter this state through actual policy evaluation.
/// There is no cloning, mutable request access, or serialized constructor.
///
/// ```compile_fail
/// use meden_core::invocation::Attempt;
/// let forged = Attempt::<&str, &str> { checked: Some(Ok("unchecked")), announced: true };
/// ```
///
/// ```compile_fail
/// use meden_core::{invocation::Attempt, Policy};
/// let issued = Attempt::new(Ok(("request", vec![])), &Policy::default(), "denied");
/// let duplicate = issued.clone();
/// ```
pub struct Attempt<T, F> {
    checked: Option<Result<T, F>>,
    announced: bool,
}

impl<T, F> Attempt<T, F> {
    /// Bind the exact request to its declared requirements and policy result.
    /// Failed preparation never becomes executable, regardless of the policy.
    pub fn new(
        preparation: Result<(T, Vec<Requirement>), F>,
        rules: &Policy,
        policy_refusal: F,
    ) -> Self {
        let checked = match preparation {
            Ok((request, requirements)) => {
                if allowed(&requirements, rules) {
                    Ok(request)
                } else {
                    Err(policy_refusal)
                }
            }
            Err(refusal) => Err(refusal),
        };
        Self {
            checked: Some(checked),
            announced: false,
        }
    }

    /// Notify once. A repeated notification discards the preparation.
    pub fn announce(&mut self) -> Notice<'_, F> {
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

    /// Consume preparation on every attempt; return the exact request at most once.
    pub fn dispatch<S>(&mut self, cancelled: bool, host_stop: Option<S>) -> Result<T, Error<F, S>> {
        let checked = self.checked.take();
        if !self.announced {
            return Err(Error::invalid());
        }
        match checked {
            Some(Ok(request)) => {
                if cancelled {
                    Err(Error::cancelled())
                } else if let Some(status) = host_stop {
                    Err(Error::stopped(status))
                } else {
                    Ok(request)
                }
            }
            Some(Err(refusal)) => Err(Error::refused(refusal)),
            None => Err(Error::invalid()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Capability, Grant};

    #[test]
    fn policy_notification_exact_payload_stop_and_consumption_share_one_controller() {
        let rules = Policy {
            name: "scoped".into(),
            grants: vec![Grant::new(Capability::FsRead, Some("/public/*"))],
            denies: vec![Capability::NetEgress],
        };
        let payload = ("original implementation", "frozen arguments");
        let prepare = || {
            Attempt::new(
                Ok((
                    payload,
                    vec![Requirement::new(Capability::FsRead, Some("/public/🦀"))],
                )),
                &rules,
                "policy refusal",
            )
        };
        let mut attempt = prepare();
        assert!(matches!(attempt.announce(), Notice::Allowed));
        assert_eq!(attempt.dispatch(false, None::<&str>), Ok(payload));
        assert_eq!(attempt.dispatch(false, None::<&str>), Err(Error::invalid()));
        for (cancelled, stop, expected) in [
            (true, None, Error::cancelled()),
            (true, Some("review"), Error::cancelled()),
            (false, Some("review"), Error::stopped("review")),
        ] {
            let mut attempt = prepare();
            assert!(matches!(attempt.announce(), Notice::Allowed));
            assert_eq!(attempt.dispatch(cancelled, stop), Err(expected));
            assert_eq!(attempt.dispatch(false, None::<&str>), Err(Error::invalid()));
        }
        let mut skipped = prepare();
        assert_eq!(skipped.dispatch(false, None::<&str>), Err(Error::invalid()));
        assert!(matches!(skipped.announce(), Notice::Invalid));
        let mut repeated = prepare();
        assert!(matches!(repeated.announce(), Notice::Allowed));
        assert!(matches!(repeated.announce(), Notice::Invalid));
        assert_eq!(
            repeated.dispatch(false, None::<&str>),
            Err(Error::invalid())
        );
        for requirements in [
            vec![Requirement::new(Capability::FsRead, Some("/private/key"))],
            vec![Requirement::new(
                Capability::NetEgress,
                Some("unapproved.example"),
            )],
        ] {
            let mut refused = Attempt::new(Ok((payload, requirements)), &rules, "policy refusal");
            assert!(matches!(
                refused.announce(),
                Notice::Refused(&"policy refusal")
            ));
            assert_eq!(
                refused.dispatch(true, Some("review")),
                Err(Error::refused("policy refusal"))
            );
            assert_eq!(refused.dispatch(false, None::<&str>), Err(Error::invalid()));
        }
        let mut malformed: Attempt<(&str, &str), _> = Attempt::new(
            Err("original preparation refusal"),
            &rules,
            "policy refusal",
        );
        assert!(matches!(
            malformed.announce(),
            Notice::Refused(&"original preparation refusal")
        ));
        assert_eq!(
            malformed.dispatch(false, None::<&str>),
            Err(Error::refused("original preparation refusal"))
        );
    }
}
