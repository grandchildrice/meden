//! The planner and the loop around it.
//!
//! A [`Planner`] is asked for a [`Plan`] and nothing else. It is given the
//! user's instruction, the signatures of the available tools, and — on a
//! retry — why the previous attempt was refused, in the sanitised form of
//! [`Refusal::safe_summary`].
//!
//! It is *not* given tool results. That is the invariant the whole design
//! rests on: literals in a plan are trusted to steer actions, so a model that
//! had read an injected web page could launder that page's instructions into
//! a trusted literal. Untrusted data is summarised by a quarantined model
//! instead (see [`Quarantined`]), whose output is labelled with the tool it
//! came from.
//!
//! [`Session::run`] is the loop: ask, run, and if the gate refuses, tell the
//! planner why and ask again, up to a bounded number of attempts.

use crate::gate::{GateConfig, Refusal};
use crate::interp::{Env, EvalError, Interpreter, Trace};
use crate::plan::Plan;
use crate::tool::{Registry, ToolSig};
use crate::value::{Data, Value};

/// What a planner is told.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanRequest {
    /// The user's own instruction, in their words.
    pub instruction: String,
    /// The tools available, with their declarations.
    pub tools: Vec<ToolSig>,
    /// Why earlier attempts were refused, oldest first. Each entry comes from
    /// [`Refusal::safe_summary`] and so quotes no argument value.
    pub feedback: Vec<String>,
}

impl PlanRequest {
    /// A first attempt.
    pub fn new(instruction: &str, tools: Vec<ToolSig>) -> Self {
        PlanRequest {
            instruction: instruction.to_owned(),
            tools,
            feedback: Vec::new(),
        }
    }

    /// The tool declarations, rendered for a prompt.
    ///
    /// Says which parameters steer a call, because that is what the planner
    /// has to respect: those may not be filled from a tool's output.
    pub fn tool_descriptions(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        for sig in &self.tools {
            let _ = write!(out, "- {}(", sig.name);
            for (i, p) in sig.params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&p.name);
                if p.control {
                    out.push_str(": steers this call");
                }
            }
            out.push(')');
            if let Some(target) = &sig.discloses_to {
                let _ = write!(out, " — discloses its arguments to {target}");
            }
            out.push('\n');
        }
        out
    }
}

/// Why a planner could not produce a plan.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PlannerError {
    /// The model's output was not a plan.
    #[error("the planner did not return a usable plan: {0}")]
    NotAPlan(String),
    /// The planner could not be reached.
    #[error("the planner could not be reached: {0}")]
    Unavailable(String),
}

/// Something that writes plans.
pub trait Planner {
    /// Produce a plan for this request.
    fn plan(&self, request: &PlanRequest) -> Result<Plan, PlannerError>;
}

/// Something that reads untrusted text so the planner does not have to.
///
/// Its answer is data, never control: whatever it returns is labelled with
/// the tool whose output it read, so it can be carried, summarised and shown
/// to the user, but it cannot decide what the agent does next.
pub trait Quarantined {
    /// Extract an answer from untrusted text.
    ///
    /// `question` is written by the planner; `text` is untrusted.
    fn extract(&self, question: &str, text: &str) -> Result<Data, PlannerError>;
}

/// A planner that replays plans given to it, for tests and for replaying a
/// recorded session.
pub struct ScriptedPlanner {
    plans: std::cell::RefCell<std::collections::VecDeque<Plan>>,
}

impl ScriptedPlanner {
    /// A planner that returns these plans in order.
    pub fn new<I: IntoIterator<Item = Plan>>(plans: I) -> Self {
        ScriptedPlanner {
            plans: std::cell::RefCell::new(plans.into_iter().collect()),
        }
    }
}

impl Planner for ScriptedPlanner {
    fn plan(&self, _request: &PlanRequest) -> Result<Plan, PlannerError> {
        self.plans
            .borrow_mut()
            .pop_front()
            .ok_or_else(|| PlannerError::NotAPlan("no plans left".to_owned()))
    }
}

/// How a session ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// A plan ran to completion. Carries its final value.
    Done(Value),
    /// Every attempt was refused by the gate. Carries the last refusal.
    Refused(Refusal),
    /// A plan failed for a reason that is not a refusal.
    Failed(EvalError),
    /// The planner could not produce a plan.
    NoPlan(PlannerError),
}

/// What a session produced.
#[derive(Clone, Debug)]
pub struct SessionReport {
    /// How it ended.
    pub outcome: Outcome,
    /// Every attempt's plan and trace, in order.
    pub attempts: Vec<(Plan, Trace)>,
}

impl SessionReport {
    /// Did a plan run to completion?
    pub fn succeeded(&self) -> bool {
        matches!(self.outcome, Outcome::Done(_))
    }

    /// Every call the gate refused, across all attempts.
    pub fn refusals(&self) -> Vec<&Refusal> {
        self.attempts
            .iter()
            .flat_map(|(_, t)| t.refusals())
            .filter_map(|c| c.outcome.as_ref().err())
            .collect()
    }
}

/// One instruction, carried out by asking a planner and running what it says.
pub struct Session<'a, P: Planner> {
    registry: &'a Registry,
    config: &'a GateConfig,
    planner: &'a P,
    max_attempts: usize,
}

impl<'a, P: Planner> Session<'a, P> {
    /// A session that will try at most three times.
    pub fn new(registry: &'a Registry, config: &'a GateConfig, planner: &'a P) -> Self {
        Session {
            registry,
            config,
            planner,
            max_attempts: 3,
        }
    }

    /// This session, with a different attempt limit. Zero means one attempt.
    #[must_use]
    pub fn with_max_attempts(mut self, attempts: usize) -> Self {
        self.max_attempts = attempts.max(1);
        self
    }

    /// Carry out an instruction.
    pub fn run(&self, instruction: &str) -> SessionReport {
        let tools = self
            .registry
            .names()
            .filter_map(|n| self.registry.get(n).map(|t| t.signature().clone()))
            .collect();
        let mut request = PlanRequest::new(instruction, tools);
        let interpreter = Interpreter::new(self.registry, self.config);
        let mut attempts = Vec::new();

        for _ in 0..self.max_attempts {
            let plan = match self.planner.plan(&request) {
                Ok(p) => p,
                Err(e) => {
                    return SessionReport {
                        outcome: Outcome::NoPlan(e),
                        attempts,
                    }
                }
            };
            let (trace, result) = interpreter.run(&plan, &mut Env::new());
            attempts.push((plan, trace));
            match result {
                Ok(value) => {
                    return SessionReport {
                        outcome: Outcome::Done(value),
                        attempts,
                    }
                }
                Err(EvalError::Refused(refusal)) => {
                    // Tell the planner why, without quoting any value.
                    request.feedback.push(refusal.safe_summary());
                    if request.feedback.len() >= self.max_attempts {
                        return SessionReport {
                            outcome: Outcome::Refused(refusal),
                            attempts,
                        };
                    }
                }
                Err(other) => {
                    return SessionReport {
                        outcome: Outcome::Failed(other),
                        attempts,
                    }
                }
            }
        }

        let last = attempts
            .last()
            .and_then(|(_, t)| t.refusals().last().cloned())
            .and_then(|c| c.outcome.err());
        SessionReport {
            outcome: match last {
                Some(r) => Outcome::Refused(r),
                None => Outcome::NoPlan(PlannerError::NotAPlan("ran out of attempts".to_owned())),
            },
            attempts,
        }
    }
}

#[cfg(test)]
mod tests {
    // Tests fail by panicking; that is their job.
    #![allow(clippy::panic)]

    use super::*;
    use crate::label::Label;
    use crate::plan::Expr;
    use crate::tool::{Args, ParamSpec, Tool, ToolError};
    use tagent_core::{Capability, Policy, Requirement};

    struct Fetch(ToolSig);
    impl Tool for Fetch {
        fn signature(&self) -> &ToolSig {
            &self.0
        }
        fn requirements(&self, _args: &Args) -> Vec<Requirement> {
            vec![Requirement::with_evidence(
                Capability::NetEgress,
                Some("blog.example.com"),
                "fetch.url",
            )]
        }
        fn call(&self, _args: &Args) -> Result<Value, ToolError> {
            Ok(Value::new(
                Data::Map(
                    [(
                        "recipient".to_owned(),
                        Value::str("attacker@evil.invalid", Label::runtime()),
                    )]
                    .into_iter()
                    .collect(),
                ),
                Label::runtime(),
            ))
        }
    }

    struct SendEmail(ToolSig);
    impl Tool for SendEmail {
        fn signature(&self) -> &ToolSig {
            &self.0
        }
        fn requirements(&self, _args: &Args) -> Vec<Requirement> {
            vec![Requirement::with_evidence(
                Capability::NetEgress,
                Some("mail.example.com"),
                "send_email.to",
            )]
        }
        fn call(&self, _args: &Args) -> Result<Value, ToolError> {
            Ok(Value::literal(Data::Unit))
        }
    }

    fn registry() -> Registry {
        Registry::new()
            .with(Box::new(Fetch(ToolSig::new(
                "fetch",
                vec![ParamSpec::control("url")],
            ))))
            .with(Box::new(SendEmail(
                ToolSig::new(
                    "send_email",
                    vec![ParamSpec::control("to"), ParamSpec::data("body")],
                )
                .disclosing_to("to"),
            )))
    }

    fn config() -> GateConfig {
        GateConfig::new(
            Policy::from_labels(
                "demo",
                &[
                    "net.egress[blog.example.com]",
                    "net.egress[mail.example.com]",
                ],
                &[],
            )
            .unwrap_or_default(),
        )
    }

    fn injected_plan() -> Plan {
        Plan::new()
            .bind("page", Expr::call("fetch").arg("url", Expr::str("u")))
            .then(
                Expr::call("send_email")
                    .arg("to", Expr::var("page").field("recipient"))
                    .arg("body", Expr::str("hi")),
            )
    }

    fn corrected_plan() -> Plan {
        Plan::new()
            .bind("page", Expr::call("fetch").arg("url", Expr::str("u")))
            .then(
                Expr::call("send_email")
                    .arg("to", Expr::str("bob@mail.example.com"))
                    .arg("body", Expr::str("hi")),
            )
    }

    #[test]
    fn a_refused_plan_is_retried_with_sanitised_feedback() {
        let (reg, cfg) = (registry(), config());
        let planner = ScriptedPlanner::new([injected_plan(), corrected_plan()]);
        let report = Session::new(&reg, &cfg, &planner).run("summarise the page and tell Bob");

        assert!(report.succeeded(), "{:?}", report.outcome);
        assert_eq!(report.attempts.len(), 2);
        assert_eq!(report.refusals().len(), 1);
    }

    #[test]
    fn a_planner_that_keeps_trying_the_same_thing_is_stopped() {
        let (reg, cfg) = (registry(), config());
        let planner = ScriptedPlanner::new([injected_plan(), injected_plan(), injected_plan()]);
        let report = Session::new(&reg, &cfg, &planner).run("do it");

        match &report.outcome {
            Outcome::Refused(Refusal::UntrustedControl { param, .. }) => assert_eq!(param, "to"),
            other => panic!("expected a refusal, got {other:?}"),
        }
        assert_eq!(report.attempts.len(), 3);
    }

    #[test]
    fn the_feedback_the_planner_sees_quotes_no_value() {
        /// A planner that records what it was told.
        struct Recording {
            seen: std::cell::RefCell<Vec<PlanRequest>>,
        }
        impl Planner for Recording {
            fn plan(&self, request: &PlanRequest) -> Result<Plan, PlannerError> {
                self.seen.borrow_mut().push(request.clone());
                Ok(injected_plan())
            }
        }

        let (reg, cfg) = (registry(), config());
        let planner = Recording {
            seen: std::cell::RefCell::new(Vec::new()),
        };
        let _ = Session::new(&reg, &cfg, &planner).run("do it");

        let seen = planner.seen.borrow();
        assert!(seen.len() >= 2, "should have retried");
        for request in seen.iter() {
            for f in &request.feedback {
                assert!(!f.contains("attacker@evil.invalid"), "leaked: {f}");
            }
        }
        assert!(seen.iter().any(|r| !r.feedback.is_empty()));
    }

    #[test]
    fn a_planner_that_cannot_answer_ends_the_session() {
        let (reg, cfg) = (registry(), config());
        let planner = ScriptedPlanner::new([]);
        let report = Session::new(&reg, &cfg, &planner).run("do it");
        assert!(matches!(report.outcome, Outcome::NoPlan(_)));
        assert!(report.attempts.is_empty());
    }

    #[test]
    fn tool_descriptions_say_which_parameters_steer() {
        let reg = registry();
        let tools = reg
            .names()
            .filter_map(|n| reg.get(n).map(|t| t.signature().clone()))
            .collect();
        let text = PlanRequest::new("x", tools).tool_descriptions();
        assert!(text.contains("to: steers this call"), "{text}");
        assert!(text.contains("discloses its arguments to to"), "{text}");
        assert!(text.contains("fetch(url: steers this call)"), "{text}");
    }

    #[test]
    fn an_attempt_limit_of_zero_still_tries_once() {
        let (reg, cfg) = (registry(), config());
        let planner = ScriptedPlanner::new([corrected_plan()]);
        let report = Session::new(&reg, &cfg, &planner)
            .with_max_attempts(0)
            .run("do it");
        assert_eq!(report.attempts.len(), 1);
    }
}
