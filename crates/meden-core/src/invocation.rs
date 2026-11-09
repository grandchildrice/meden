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

/// An admitted request retains the exact immutable policy used to check it.
/// Only the invocation constructor can bind these fields. This is neither
/// cloneable nor deserializable; consumption returns that original pair.
///
/// ```compile_fail
/// use meden_core::{Policy, invocation::Call};
/// let rules = Policy::default();
/// let forged = Call { request: "unchecked", rules: &rules };
/// ```
///
/// ```compile_fail
/// use meden_core::{Policy, invocation::Attempt};
/// let mut rules = Policy::default();
/// let mut call = Attempt::with_policy(Ok(("request", vec![])), &rules, "denied");
/// rules.grants.clear();
/// call.announce();
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Call<'p, T> {
    request: T,
    rules: &'p Policy,
}

impl<'p, T> Call<'p, T> {
    /// Consume the admitted request together with its original policy context.
    pub fn into_parts(self) -> (T, &'p Policy) {
        (self.request, self.rules)
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
/// let rules = Policy::default();
/// let issued = Attempt::with_policy(Ok(("request", vec![])), &rules, "denied");
/// let duplicate = issued.clone();
/// ```
pub struct Attempt<T, F> {
    checked: Option<Result<T, F>>,
    announced: bool,
}

impl<T, F> Attempt<T, F> {
    /// Check a preparation while freezing its exact policy reference in the
    /// returned call. Dispatch retains this same immutable context.
    pub fn with_policy(
        preparation: Result<(T, Vec<Requirement>), F>,
        rules: &Policy,
        policy_refusal: F,
    ) -> Attempt<Call<'_, T>, F> {
        match preparation {
            Ok((request, requirements)) => Attempt::new(
                Ok((Call { request, rules }, requirements)),
                rules,
                policy_refusal,
            ),
            Err(refusal) => Attempt::new(Err(refusal), rules, policy_refusal),
        }
    }

    /// Bind the exact request to its declared requirements and policy result.
    /// Failed preparation never becomes executable, regardless of the policy.
    fn new(
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
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::{Capability, Grant};

    #[test]
    fn the_admitted_call_retains_its_original_immutable_policy_context() {
        use crate::file::{Operation, Request};
        let rules = Policy {
            name: "original context".into(),
            grants: vec![Grant::new(Capability::FsRead, Some("/public/*"))],
            denies: vec![Capability::FsWrite],
        };
        let mut broader = rules.clone();
        broader.name = "other context".into();
        broader
            .grants
            .push(Grant::new(Capability::FsRead, Some("/private/*")));
        broader.denies.clear();
        let request = || {
            Request::new(
                Operation::Read,
                "/public/link".into(),
                ("original payload", 17),
            )
        };
        let mut attempt = Attempt::with_policy(Ok(request().prepare()), &rules, "denied");
        assert!(matches!(attempt.announce(), Notice::Allowed));
        let call = attempt.dispatch(false, None::<()>).expect("admitted call");
        let (original, context) = call.into_parts();
        assert!(std::ptr::eq(context, std::ptr::from_ref(&rules)));
        assert_eq!(context, &rules);
        assert_ne!(context, &broader);
        assert!(original
            .bind("/private/secret".into(), "original resource")
            .consume(context)
            .is_none());
        assert!(request()
            .bind("/private/secret".into(), "original resource")
            .consume(&broader)
            .is_some());
        assert!(
            matches!(attempt.dispatch(false, None::<()>), Err(error) if error == Error::invalid())
        );

        let mut second =
            Attempt::with_policy(Ok((("second payload", 29), vec![])), &broader, "denied");
        assert!(matches!(second.announce(), Notice::Allowed));
        let ((payload, value), context) = second
            .dispatch(false, None::<()>)
            .expect("second call")
            .into_parts();
        assert_eq!((payload, value), ("second payload", 29));
        assert!(std::ptr::eq(context, std::ptr::from_ref(&broader)));

        let mut refused = Attempt::with_policy(
            Ok((
                "not admitted",
                vec![crate::Requirement::new(
                    Capability::FsWrite,
                    Some("/public/note"),
                )],
            )),
            &rules,
            "policy refusal",
        );
        assert!(matches!(
            refused.announce(),
            Notice::Refused(&"policy refusal")
        ));
        assert_eq!(
            refused.dispatch(false, None::<()>),
            Err(Error::refused("policy refusal"))
        );
        let mut failed: Attempt<Call<'_, &str>, &str> =
            Attempt::with_policy(Err("preparation refusal"), &rules, "policy refusal");
        assert!(matches!(
            failed.announce(),
            Notice::Refused(&"preparation refusal")
        ));
        assert_eq!(
            failed.dispatch(false, None::<()>),
            Err(Error::refused("preparation refusal"))
        );
    }

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
