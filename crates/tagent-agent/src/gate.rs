//! The three checks every tool call passes before it runs.
//!
//! 1. **Capability.** The call's requirements must be granted by the policy.
//!    This is `tagent-core` and says *is this kind of action allowed at all?*
//! 2. **Integrity.** Every argument that steers the call must come from
//!    trusted sources. This says *may this data decide what happens?* and is
//!    what stops prompt injection: a web page may say whatever it likes, but
//!    its words carry `Tool("fetch")` and so cannot choose a recipient.
//! 3. **Confidentiality.** If the call discloses its arguments to someone,
//!    every argument must permit that reader. This says *may this data go
//!    there?* and is what stops exfiltration.
//!
//! None of the three consults a model. A refusal is a fact about labels and
//! policy, so it holds however convincingly the input argues otherwise.
//!
//! Specified in `lean/TAgentSpec/Flow.lean`.

use std::collections::BTreeSet;

use tagent_core::{check, Policy};
use thiserror::Error;

use crate::label::{Label, Source};
use crate::tool::{Args, Tool};

/// Why a call was refused.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum Refusal {
    /// No tool of that name is registered.
    #[error("no tool named {name:?}")]
    UnknownTool {
        /// The name asked for.
        name: String,
    },

    /// A declared parameter had no argument.
    #[error("{tool}: missing argument {param:?}")]
    MissingArgument {
        /// Tool name.
        tool: String,
        /// Parameter name.
        param: String,
    },

    /// The policy does not grant what this call needs.
    #[error("{tool}: {}", reasons.join("; "))]
    Policy {
        /// Tool name.
        tool: String,
        /// One line per unmet requirement.
        reasons: Vec<String>,
    },

    /// Untrusted data reached a parameter that steers the call.
    #[error(
        "{tool}: argument {param:?} steers this call but carries untrusted data from {}",
        sources.iter().map(|s| format!("{s:?}")).collect::<Vec<_>>().join(", ")
    )]
    UntrustedControl {
        /// Tool name.
        tool: String,
        /// The steering parameter.
        param: String,
        /// The sources that are not trusted.
        sources: Vec<Source>,
    },

    /// The parameter naming the disclosure target was not text.
    #[error("{tool}: argument {param:?} names who this discloses to, so it must be text")]
    DisclosureTargetNotText {
        /// Tool name.
        tool: String,
        /// The parameter that should have been text.
        param: String,
    },

    /// An argument may not be read by the principal this call discloses to.
    #[error("{tool}: argument {param:?} may not be disclosed to {principal:?}")]
    Disclosure {
        /// Tool name.
        tool: String,
        /// The argument that may not be disclosed.
        param: String,
        /// Who the call would have disclosed it to.
        principal: String,
    },
}

impl Refusal {
    /// A description of this refusal that is safe to show the planning model.
    ///
    /// A refusal naturally wants to quote what went wrong, but some of that
    /// is argument *values*: the principal of a [`Refusal::Disclosure`], or
    /// the scope inside a [`Refusal::Policy`] reason, can both be derived
    /// from untrusted data. Feeding those back to the planner would launder
    /// untrusted content into a model whose output the runtime trusts, which
    /// is exactly the invariant the interpreter depends on.
    ///
    /// So this names the rule, the tool and the parameter, and never a value.
    /// [`std::fmt::Display`] stays verbose for the operator's log, which is
    /// not an input to anything.
    pub fn safe_summary(&self) -> String {
        match self {
            Refusal::UnknownTool { name } => {
                format!("there is no tool called {name:?}")
            }
            Refusal::MissingArgument { tool, param } => {
                format!("{tool} needs an argument for {param:?}")
            }
            Refusal::Policy { tool, .. } => {
                format!("the policy does not allow what {tool} was asked to do")
            }
            Refusal::UntrustedControl { tool, param, .. } => format!(
                "{tool}: {param:?} decides what this call does, so it cannot come from \
                 data a tool returned; write the value yourself or take it from the user"
            ),
            Refusal::DisclosureTargetNotText { tool, param } => {
                format!("{tool}: {param:?} must be text")
            }
            Refusal::Disclosure { tool, param, .. } => {
                format!("{tool}: {param:?} may not be disclosed to that recipient")
            }
        }
    }
}

/// What the gate needs in order to rule on a call.
pub struct GateConfig {
    /// The capability policy.
    pub policy: Policy,
    /// The sources trusted to steer actions.
    pub trusted: BTreeSet<Source>,
}

impl GateConfig {
    /// A configuration with the usual baseline trust.
    pub fn new(policy: Policy) -> Self {
        GateConfig {
            policy,
            trusted: crate::label::baseline_trust(),
        }
    }

    /// This configuration, additionally trusting a tool's output to steer.
    #[must_use]
    pub fn trusting_tool(mut self, name: &str) -> Self {
        self.trusted.insert(Source::tool(name));
        self
    }
}

/// The integrity rule, over the two facts it actually depends on: for each
/// argument, does it steer, and is its label trusted?
///
/// Stating it this way keeps the rule free of heap data, so it can be
/// model-checked exhaustively. Mirrors `TAgentSpec.integrityOk`.
pub fn integrity_ok_flags(args: &[(bool, bool)]) -> bool {
    args.iter().all(|(control, trusted)| !control || *trusted)
}

/// The confidentiality rule, over whether each argument permits the reader.
///
/// Mirrors `TAgentSpec.disclosureOk`.
pub fn disclosure_ok_flags(permits: &[bool]) -> bool {
    permits.iter().all(|p| *p)
}

/// The integrity rule on labels: may this set of arguments steer?
///
/// `args` pairs each argument's "does it steer" flag with its label.
pub fn integrity_ok(trusted: &BTreeSet<Source>, args: &[(bool, Label)]) -> bool {
    let flags: Vec<(bool, bool)> = args
        .iter()
        .map(|(control, label)| (*control, label.trusted_for_control(trusted)))
        .collect();
    integrity_ok_flags(&flags)
}

/// The confidentiality rule on labels: may all of this go to `principal`?
pub fn disclosure_ok(principal: &str, labels: &[Label]) -> bool {
    let permits: Vec<bool> = labels.iter().map(|l| l.allows_reader(principal)).collect();
    disclosure_ok_flags(&permits)
}

/// Rule on a call. `Ok(())` means it may run.
pub fn admit(tool: &dyn Tool, args: &Args, config: &GateConfig) -> Result<(), Refusal> {
    let sig = tool.signature();
    let name = sig.name.clone();

    for param in &sig.params {
        if !args.contains_key(&param.name) {
            return Err(Refusal::MissingArgument {
                tool: name,
                param: param.name.clone(),
            });
        }
    }

    // 1. Capability.
    let decision = check(&tool.requirements(args), &config.policy);
    if decision.denied() {
        return Err(Refusal::Policy {
            tool: name,
            reasons: decision.reasons,
        });
    }

    // 2. Integrity: nothing untrusted may steer. The loop finds which
    // argument to name in the refusal; `integrity_ok` is the rule itself.
    debug_assert_eq!(
        integrity_ok(
            &config.trusted,
            &sig.params
                .iter()
                .filter_map(|p| args.get(&p.name).map(|v| (p.control, v.deep_label())))
                .collect::<Vec<_>>(),
        ),
        sig.control_params().all(|p| args
            .get(&p.name)
            .is_none_or(|v| v.deep_label().trusted_for_control(&config.trusted))),
    );
    for param in sig.control_params() {
        let Some(value) = args.get(&param.name) else {
            continue;
        };
        let label = value.deep_label();
        if !label.trusted_for_control(&config.trusted) {
            return Err(Refusal::UntrustedControl {
                tool: name,
                param: param.name.clone(),
                sources: label.untrusted_sources(&config.trusted),
            });
        }
    }

    // 3. Confidentiality: nothing may go to a reader it does not permit.
    if let Some(target_param) = &sig.discloses_to {
        let Some(target) = args.get(target_param) else {
            return Err(Refusal::MissingArgument {
                tool: name,
                param: target_param.clone(),
            });
        };
        let Some(principal) = target.as_str() else {
            return Err(Refusal::DisclosureTargetNotText {
                tool: name,
                param: target_param.clone(),
            });
        };
        for (param, value) in args {
            if param == target_param {
                continue;
            }
            if !value.deep_label().allows_reader(principal) {
                return Err(Refusal::Disclosure {
                    tool: name,
                    param: param.clone(),
                    principal: principal.to_owned(),
                });
            }
        }
    }

    Ok(())
}

/// The label to put on a call's result: everything its arguments carried,
/// plus the tool itself as a new source.
pub fn result_label(tool_name: &str, args: &Args) -> Label {
    args.values()
        .fold(Label::runtime(), |acc, v| acc.join(&v.deep_label()))
        .with_source(Source::tool(tool_name))
}

#[cfg(test)]
mod tests {
    // Tests fail by panicking; that is their job.
    #![allow(clippy::panic)]

    use super::*;
    use crate::label::Readers;
    use crate::tool::{ParamSpec, ToolError, ToolSig};
    use crate::value::{Data, Value};
    use tagent_core::{Capability, Requirement};

    struct SendEmail(ToolSig);

    impl Tool for SendEmail {
        fn signature(&self) -> &ToolSig {
            &self.0
        }
        fn requirements(&self, args: &Args) -> Vec<Requirement> {
            let domain = args
                .get("to")
                .and_then(Value::as_str)
                .and_then(|s| s.split('@').next_back().map(str::to_owned))
                .unwrap_or_default();
            vec![Requirement::with_evidence(
                Capability::NetEgress,
                Some(&domain),
                "send_email.to",
            )]
        }
        fn call(&self, _args: &Args) -> Result<Value, ToolError> {
            Ok(Value::literal(Data::Unit))
        }
    }

    fn send_email() -> SendEmail {
        SendEmail(
            ToolSig::new(
                "send_email",
                vec![ParamSpec::control("to"), ParamSpec::data("body")],
            )
            .disclosing_to("to"),
        )
    }

    fn config() -> GateConfig {
        GateConfig::new(
            Policy::from_labels("mail", &["net.egress[*.example.com]"], &[]).unwrap_or_default(),
        )
    }

    fn args(to: Value, body: Value) -> Args {
        [("to".to_owned(), to), ("body".to_owned(), body)]
            .into_iter()
            .collect()
    }

    #[test]
    fn a_well_formed_call_is_admitted() {
        let a = args(
            Value::str("bob@mail.example.com", Label::user()),
            Value::str("hello", Label::user()),
        );
        assert_eq!(admit(&send_email(), &a, &config()), Ok(()));
    }

    #[test]
    fn the_policy_still_applies() {
        let a = args(
            Value::str("bob@other.invalid", Label::user()),
            Value::str("hello", Label::user()),
        );
        let err = admit(&send_email(), &a, &config());
        assert!(matches!(err, Err(Refusal::Policy { .. })), "{err:?}");
    }

    /// The injection scenario: a fetched page names the recipient.
    #[test]
    fn fetched_data_cannot_choose_the_recipient() {
        let from_page = Label::public([Source::tool("fetch")]);
        let a = args(
            Value::str("bob@mail.example.com", from_page),
            Value::str("hello", Label::user()),
        );
        match admit(&send_email(), &a, &config()) {
            Err(Refusal::UntrustedControl { param, sources, .. }) => {
                assert_eq!(param, "to");
                assert_eq!(sources, vec![Source::tool("fetch")]);
            }
            other => panic!("expected an untrusted-control refusal, got {other:?}"),
        }
    }

    /// Trust is a deployment decision, so a trusted tool may steer.
    #[test]
    fn a_trusted_tool_may_choose_the_recipient() {
        let from_contacts = Label::public([Source::tool("contacts")]);
        let a = args(
            Value::str("bob@mail.example.com", from_contacts),
            Value::str("hello", Label::user()),
        );
        let cfg = config().trusting_tool("contacts");
        assert_eq!(admit(&send_email(), &a, &cfg), Ok(()));
    }

    /// The exfiltration scenario: a secret body to someone who may not read it.
    #[test]
    fn a_secret_cannot_be_disclosed_to_an_outsider() {
        let secret = Label {
            sources: [Source::tool("vault")].into_iter().collect(),
            readers: Readers::only(["alice@mail.example.com"]),
        };
        let a = args(
            Value::str("bob@mail.example.com", Label::user()),
            Value::str("the passphrase", secret),
        );
        match admit(&send_email(), &a, &config()) {
            Err(Refusal::Disclosure {
                param, principal, ..
            }) => {
                assert_eq!(param, "body");
                assert_eq!(principal, "bob@mail.example.com");
            }
            other => panic!("expected a disclosure refusal, got {other:?}"),
        }
    }

    #[test]
    fn a_secret_may_go_to_a_permitted_reader() {
        let secret = Label {
            sources: [Source::tool("vault")].into_iter().collect(),
            readers: Readers::only(["alice@mail.example.com"]),
        };
        let a = args(
            Value::str("alice@mail.example.com", Label::user()),
            Value::str("the passphrase", secret),
        );
        assert_eq!(admit(&send_email(), &a, &config()), Ok(()));
    }

    #[test]
    fn a_nested_secret_is_found_too() {
        let secret = Label {
            sources: [].into_iter().collect(),
            readers: Readers::only(["alice@mail.example.com"]),
        };
        let body = Value::new(
            Data::List(vec![Value::str("the passphrase", secret)]),
            Label::user(),
        );
        let a = args(Value::str("bob@mail.example.com", Label::user()), body);
        assert!(matches!(
            admit(&send_email(), &a, &config()),
            Err(Refusal::Disclosure { .. })
        ));
    }

    #[test]
    fn missing_and_malformed_arguments_are_refused() {
        let only_to: Args = [(
            "to".to_owned(),
            Value::str("bob@mail.example.com", Label::user()),
        )]
        .into_iter()
        .collect();
        assert!(matches!(
            admit(&send_email(), &only_to, &config()),
            Err(Refusal::MissingArgument { .. })
        ));

        // A non-text disclosure target. The capability check runs first, so
        // this needs a policy that grants the call before the confidentiality
        // rule can be the thing that objects.
        let permissive =
            GateConfig::new(Policy::from_labels("any", &["net.egress"], &[]).unwrap_or_default());
        let a = args(
            Value::literal(Data::Int(7)),
            Value::str("hello", Label::user()),
        );
        assert!(matches!(
            admit(&send_email(), &a, &permissive),
            Err(Refusal::DisclosureTargetNotText { .. })
        ));
    }

    /// The three checks run capability, then integrity, then confidentiality,
    /// so the reason a call is refused is the first rule it breaks.
    #[test]
    fn the_checks_run_in_order() {
        // Breaks all three at once: unknown domain, tainted recipient,
        // restricted body.
        let a = args(
            Value::str("bob@other.invalid", Label::public([Source::tool("fetch")])),
            Value::str(
                "x",
                Label {
                    sources: [].into_iter().collect(),
                    readers: Readers::only(["alice"]),
                },
            ),
        );
        assert!(matches!(
            admit(&send_email(), &a, &config()),
            Err(Refusal::Policy { .. })
        ));
    }

    /// A refusal shown to the planner must not quote argument values, or an
    /// injected page could reach the planner through the error message.
    #[test]
    fn a_safe_summary_never_quotes_a_value() {
        let secret_value = "attacker@evil.invalid";
        let refusals = [
            Refusal::Disclosure {
                tool: "send_email".to_owned(),
                param: "body".to_owned(),
                principal: secret_value.to_owned(),
            },
            Refusal::Policy {
                tool: "send_email".to_owned(),
                reasons: vec![format!("net.egress[{secret_value}] is not granted")],
            },
            Refusal::UntrustedControl {
                tool: "send_email".to_owned(),
                param: "to".to_owned(),
                sources: vec![Source::tool("fetch")],
            },
        ];
        for r in &refusals {
            assert!(
                !r.safe_summary().contains(secret_value),
                "leaked a value: {}",
                r.safe_summary()
            );
            // The operator's log may still say everything.
            assert!(!r.to_string().is_empty());
        }
        assert!(refusals[0].safe_summary().contains("send_email"));
    }

    #[test]
    fn the_label_rules_agree_with_the_flag_rules() {
        let trusted = crate::label::baseline_trust();
        let clean = Label::user();
        let dirty = Label::public([Source::tool("fetch")]);
        let restricted = Label {
            sources: [].into_iter().collect(),
            readers: Readers::only(["alice"]),
        };
        for control_a in [false, true] {
            for control_b in [false, true] {
                for (a, b) in [(&clean, &clean), (&clean, &dirty), (&dirty, &dirty)] {
                    let labelled = [(control_a, a.clone()), (control_b, b.clone())];
                    let flags = [
                        (control_a, a.trusted_for_control(&trusted)),
                        (control_b, b.trusted_for_control(&trusted)),
                    ];
                    assert_eq!(
                        integrity_ok(&trusted, &labelled),
                        integrity_ok_flags(&flags)
                    );
                }
            }
        }
        for labels in [
            vec![clean.clone()],
            vec![restricted.clone()],
            vec![clean.clone(), restricted.clone()],
        ] {
            let permits: Vec<bool> = labels.iter().map(|l| l.allows_reader("bob")).collect();
            assert_eq!(disclosure_ok("bob", &labels), disclosure_ok_flags(&permits));
        }
    }

    #[test]
    fn a_result_is_labelled_with_the_tool_and_its_arguments() {
        let a = args(
            Value::str("bob@mail.example.com", Label::user()),
            Value::str("hello", Label::public([Source::tool("fetch")])),
        );
        let l = result_label("send_email", &a);
        assert!(l.sources.contains(&Source::User));
        assert!(l.sources.contains(&Source::tool("fetch")));
        assert!(l.sources.contains(&Source::tool("send_email")));
    }
}
