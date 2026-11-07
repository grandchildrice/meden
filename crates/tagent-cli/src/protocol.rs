//! `tagent run --protocol`: let another process own the tools.
//!
//! A benchmark harness like [AgentDojo](https://github.com/ethz-spylab/agentdojo)
//! holds the tools and the world they act on, both in Python. The planning,
//! the labels and the gate stay here. So the two talk over JSON lines on
//! standard input and output: the host describes the tools and the task, this
//! process asks for calls, and the host runs them.
//!
//! What crosses the line is plain JSON. Labels do not: a label is a claim
//! this runtime makes about where a value came from, and accepting one from
//! outside would let the host assert that a web page is trustworthy. The
//! host says *which tool* answered; the runtime decides what that means.
//!
//! ```text
//! host → {"instruction": "...", "tools": [...], "policy": {...}}
//! agent → {"type": "asking", "attempt": 1}
//! agent → {"type": "call", "id": 1, "tool": "read_file", "args": {"path": "a"}}
//! host → {"id": 1, "ok": "contents"}
//! agent → {"type": "decision", "attempt": 1, "tool": "read_file", "allowed": true}
//! agent → {"type": "done", "outcome": "done", "answer": "...", "refusals": []}
//! ```
//!
//! A session is not one plan. The planner may be asked again — because the
//! gate refused a call, because the plan went wrong on its own terms, or
//! because the plan set `"continues": true` to read something before it could
//! say what to do with it. So expect several `asking`, `plan` and `call`
//! messages before `done`, bounded by `max_attempts`.

use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::sync::{Arc, Mutex};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use tagent_agent::planner::{Outcome, Quarantined, Session, SessionEvent};
use tagent_agent::tool::{Args, ParamSpec, Tool, ToolError, ToolSig};
use tagent_agent::{
    data_from_json, data_to_json, GateConfig, Label, Refusal, Registry, Source, Value,
};
use tagent_core::{Capability, Policy, Requirement};

/// One tool, as the host describes it.
#[derive(Debug, Deserialize)]
pub struct ToolSpec {
    /// Its name.
    pub name: String,
    /// What it does, in a sentence.
    #[serde(default)]
    pub description: String,
    /// Its parameters.
    #[serde(default)]
    pub params: Vec<ParamSpecJson>,
    /// The parameter naming who this call discloses its arguments to.
    #[serde(default)]
    pub discloses_to: Option<String>,
    /// The capabilities a call needs, as `cap` or `cap[scope]`.
    ///
    /// A scope may name a parameter in braces: `net.egress[{to}]` takes its
    /// scope from the `to` argument of the call being made.
    #[serde(default)]
    pub requires: Vec<String>,
    /// Who may read what this tool returns.
    ///
    /// Absent means anyone. Naming principals here is how a deployment says
    /// that a tool returns something confidential: the result's readers are
    /// narrowed to this list, and any later call that would disclose it to
    /// someone else is refused.
    #[serde(default)]
    pub returns_readers: Option<Vec<String>>,
}

/// One parameter, as the host describes it.
#[derive(Debug, Deserialize)]
pub struct ParamSpecJson {
    /// Its name.
    pub name: String,
    /// Does it steer what the call does?
    #[serde(default)]
    pub control: bool,
    /// Must a call supply it?
    ///
    /// Defaults to required, because a host that says nothing about a
    /// parameter is better served by a refusal it can read than by a call
    /// that reaches a tool without an argument it needed.
    #[serde(default = "yes")]
    pub required: bool,
}

/// The default for [`ParamSpecJson::required`].
const fn yes() -> bool {
    true
}

/// What the host sends to start a session.
#[derive(Debug, Deserialize)]
pub struct Start {
    /// The user's instruction.
    pub instruction: String,
    /// The tools available.
    pub tools: Vec<ToolSpec>,
    /// The capability policy.
    pub policy: Policy,
    /// Sources trusted to steer actions. Defaults to the usual baseline.
    #[serde(default)]
    pub trusted: Vec<Source>,
    /// How many times to let the planner try. Defaults to three.
    #[serde(default)]
    pub max_attempts: Option<usize>,
}

/// What the agent sends back.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Out {
    /// About to ask the planner.
    Asking {
        /// Which attempt, from one.
        attempt: usize,
    },
    /// Please run this tool and reply with its result.
    Call {
        /// Correlates the reply.
        id: u64,
        /// Which tool.
        tool: String,
        /// Its arguments, as plain JSON.
        args: serde_json::Value,
    },
    /// What the planner wrote, so a host can log it.
    Plan {
        /// Which attempt.
        attempt: usize,
        /// The plan, as JSON.
        plan: serde_json::Value,
    },
    /// What the gate decided about a call.
    Decision {
        /// Which attempt.
        attempt: usize,
        /// Which tool.
        tool: String,
        /// Whether it ran.
        allowed: bool,
        /// The full reason, for a log.
        #[serde(skip_serializing_if = "Option::is_none")]
        refusal: Option<String>,
        /// The reason as the planner is told it, which quotes no value.
        #[serde(skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
        /// Which rule refused, as a stable name.
        #[serde(skip_serializing_if = "Option::is_none")]
        rule: Option<&'static str>,
    },
    /// The session ended.
    Done {
        /// `done`, `refused`, `failed` or `no_plan`.
        outcome: &'static str,
        /// A sentence for a person.
        summary: String,
        /// The final value, when there is one.
        #[serde(skip_serializing_if = "Option::is_none")]
        answer: Option<serde_json::Value>,
        /// Which rules refused anything, in order.
        refusals: Vec<&'static str>,
    },
}

/// The host's reply to a [`Out::Call`].
#[derive(Debug, Deserialize)]
struct CallReply {
    id: u64,
    #[serde(default)]
    ok: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<String>,
}

/// The stable name of whichever rule refused.
pub const fn rule_name(r: &Refusal) -> &'static str {
    match r {
        Refusal::UnknownTool { .. } => "unknown_tool",
        Refusal::MissingArgument { .. } => "missing_argument",
        Refusal::Policy { .. } => "policy",
        Refusal::UntrustedControl { .. } => "untrusted_control",
        Refusal::DisclosureTargetNotText { .. } => "disclosure_target_not_text",
        Refusal::Disclosure { .. } => "disclosure",
    }
}

/// The line-oriented link to the host.
struct Link {
    out: Mutex<Box<dyn Write + Send>>,
    input: Mutex<Box<dyn BufRead + Send>>,
    next_id: Mutex<u64>,
}

impl Link {
    fn send(&self, message: &Out) -> Result<()> {
        let line = serde_json::to_string(message)?;
        let mut out = self
            .out
            .lock()
            .map_err(|_| anyhow::anyhow!("the output link is poisoned"))?;
        writeln!(out, "{line}")?;
        out.flush()?;
        Ok(())
    }

    /// Ask the host to run a call and wait for its reply.
    fn call(&self, tool: &str, args: &serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let id = {
            let mut next = self
                .next_id
                .lock()
                .map_err(|_| ToolError::Failed("the link is poisoned".to_owned()))?;
            let id = *next;
            *next = next.saturating_add(1);
            id
        };
        self.send(&Out::Call {
            id,
            tool: tool.to_owned(),
            args: args.clone(),
        })
        .map_err(|e| ToolError::Failed(e.to_string()))?;

        let mut line = String::new();
        {
            let mut input = self
                .input
                .lock()
                .map_err(|_| ToolError::Failed("the link is poisoned".to_owned()))?;
            input
                .read_line(&mut line)
                .map_err(|e| ToolError::Failed(e.to_string()))?;
        }
        if line.trim().is_empty() {
            return Err(ToolError::Failed("the host closed the link".to_owned()));
        }
        let reply: CallReply = serde_json::from_str(&line)
            .map_err(|e| ToolError::Failed(format!("unreadable reply: {e}")))?;
        if reply.id != id {
            return Err(ToolError::Failed(format!(
                "the host answered call {} while {id} was outstanding",
                reply.id
            )));
        }
        match (reply.ok, reply.error) {
            (_, Some(e)) => Err(ToolError::Failed(e)),
            (Some(value), None) => Ok(value),
            (None, None) => Ok(serde_json::Value::Null),
        }
    }
}

/// A tool the host owns.
///
/// Holds the link by `Arc` rather than by reference because the registry
/// stores tools as `Box<dyn Tool>`, which outlives any borrow.
struct HostTool {
    sig: ToolSig,
    requires: Vec<String>,
    returns_readers: Option<Vec<String>>,
    link: Arc<Link>,
}

impl Tool for HostTool {
    fn signature(&self) -> &ToolSig {
        &self.sig
    }

    fn requirements(&self, args: &Args) -> Vec<Requirement> {
        self.requires
            .iter()
            .filter_map(|spec| parse_requirement(spec, args, &self.sig.name))
            .collect()
    }

    fn call(&self, args: &Args) -> Result<Value, ToolError> {
        let plain = serde_json::Value::Object(
            args.iter()
                .map(|(k, v)| (k.clone(), data_to_json(&v.data)))
                .collect(),
        );
        let answer = self.link.call(&self.sig.name, &plain)?;
        // The interpreter labels this with the tool; the label here only has
        // to be the neutral one it joins onto, narrowed to the readers the
        // deployment declared for this tool.
        let mut label = Label::public([Source::tool(&self.sig.name)]);
        if let Some(readers) = &self.returns_readers {
            label = label.restrict_to(&tagent_agent::Readers::only(readers.clone()));
        }
        Ok(Value::new(data_from_json(&answer, &label), label))
    }
}

/// Read `cap` or `cap[scope]`, filling `{param}` from the call's arguments.
fn parse_requirement(spec: &str, args: &Args, tool: &str) -> Option<Requirement> {
    let (name, scope) = match spec.strip_suffix(']') {
        Some(rest) => {
            let (name, scope) = rest.split_once('[')?;
            (name, Some(scope))
        }
        None => (spec, None),
    };
    let capability: Capability = name.parse().ok()?;
    let scope = scope.map(|s| match (s.strip_prefix('{'), s.strip_suffix('}')) {
        (Some(inner), _) | (_, Some(inner)) => {
            let param = inner.trim_matches(['{', '}']);
            args.get(param)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        }
        _ => s.to_owned(),
    });
    Some(Requirement::with_evidence(
        capability,
        scope.as_deref(),
        &format!("{tool}.requires"),
    ))
}

/// Run one session driven by the host.
pub fn run(
    start: &Start,
    planner: &dyn tagent_agent::Planner,
    reader: Option<&dyn Quarantined>,
    input: Box<dyn BufRead + Send>,
    output: Box<dyn Write + Send>,
) -> Result<()> {
    let link = Arc::new(Link {
        out: Mutex::new(output),
        input: Mutex::new(input),
        next_id: Mutex::new(1),
    });

    let mut registry = Registry::new();
    for spec in &start.tools {
        let params = spec
            .params
            .iter()
            .map(|p| ParamSpec {
                name: p.name.clone(),
                control: p.control,
                required: p.required,
            })
            .collect();
        let mut sig = ToolSig::new(&spec.name, params).describing(&spec.description);
        sig.discloses_to.clone_from(&spec.discloses_to);
        registry.register(Box::new(HostTool {
            sig,
            requires: spec.requires.clone(),
            returns_readers: spec.returns_readers.clone(),
            link: Arc::clone(&link),
        }));
    }

    let mut config = GateConfig::new(start.policy.clone());
    if !start.trusted.is_empty() {
        config.trusted = start.trusted.iter().cloned().collect();
    }

    let observer = |event: SessionEvent<'_>| {
        let message = match event {
            SessionEvent::Asking { attempt } => vec![Out::Asking { attempt }],
            SessionEvent::Attempted {
                attempt,
                plan,
                trace,
            } => std::iter::once(Out::Plan {
                attempt,
                plan: serde_json::to_value(plan).unwrap_or(serde_json::Value::Null),
            })
            .chain(trace.calls.iter().map(|c| Out::Decision {
                attempt,
                tool: c.tool.clone(),
                allowed: c.allowed(),
                refusal: c.outcome.as_ref().err().map(ToString::to_string),
                summary: c.outcome.as_ref().err().map(Refusal::safe_summary),
                rule: c.outcome.as_ref().err().map(rule_name),
            }))
            .collect(),
        };
        for m in &message {
            let _ = link.send(m);
        }
    };

    let planner_ref = PlannerRef(planner);
    let mut session = Session::new(&registry, &config, &planner_ref).with_observer(&observer);
    if let Some(r) = reader {
        session = session.with_quarantined(r);
    }
    if let Some(n) = start.max_attempts {
        session = session.with_max_attempts(n);
    }
    let report = session.run(&start.instruction);

    let refusals: Vec<&'static str> = report.refusals().iter().map(|r| rule_name(r)).collect();
    let (outcome, summary, answer) = match &report.outcome {
        Outcome::Done(v) => (
            "done",
            "the plan ran".to_owned(),
            Some(data_to_json(&v.data)),
        ),
        Outcome::Refused(r) => ("refused", r.to_string(), None),
        Outcome::Failed(e) => ("failed", e.to_string(), None),
        Outcome::NoPlan(e) => ("no_plan", e.to_string(), None),
    };
    link.send(&Out::Done {
        outcome,
        summary,
        answer,
        refusals,
    })?;
    Ok(())
}

/// Borrows a planner so a trait object fits where a concrete type is wanted.
struct PlannerRef<'a>(&'a dyn tagent_agent::Planner);

impl tagent_agent::Planner for PlannerRef<'_> {
    fn plan(
        &self,
        request: &tagent_agent::PlanRequest,
    ) -> Result<tagent_agent::Plan, tagent_agent::PlannerError> {
        self.0.plan(request)
    }
}

/// Read the opening message from the host.
pub fn read_start(input: &mut impl BufRead) -> Result<Start> {
    let mut text = String::new();
    loop {
        let mut line = String::new();
        let read = input
            .read_line(&mut line)
            .context("reading the opening message")?;
        if read == 0 {
            break;
        }
        text.push_str(&line);
        if text.trim().is_empty() {
            continue;
        }
        // Completeness and validity are different questions: an unfinished
        // object and a finished one missing a field both fail to parse as a
        // `Start`, and only the first is worth waiting on.
        match serde_json::from_str::<serde_json::Value>(&text) {
            // Unfinished: keep reading.
            Err(e) if e.is_eof() => {}
            Err(e) => return Err(e).context("parsing the opening message"),
            Ok(_) => return serde_json::from_str(&text).context("reading the opening message"),
        }
    }
    if text.trim().is_empty() {
        bail!("the host sent nothing");
    }
    bail!("the opening message never completed")
}

/// How each declared tool reads back, so a host can check its own
/// declarations before a run.
pub fn describe(start: &Start) -> BTreeMap<String, Vec<String>> {
    start
        .tools
        .iter()
        .map(|t| {
            let mut notes = Vec::new();
            for p in &t.params {
                notes.push(if p.control {
                    format!("{} (steers)", p.name)
                } else {
                    p.name.clone()
                });
            }
            if let Some(d) = &t.discloses_to {
                notes.push(format!("discloses to {d}"));
            }
            (t.name.clone(), notes)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    // Tests fail by panicking; that is their job.
    #![allow(clippy::panic)]

    use super::*;
    use std::io::Cursor;
    use tagent_agent::{Expr, Plan, ScriptedPlanner};

    /// A writer the test can read back afterwards. The link takes an owned
    /// `Box<dyn Write + Send>`, so a borrowed buffer will not do.
    #[derive(Clone, Default)]
    struct SharedBuf(Arc<Mutex<Vec<u8>>>);

    impl SharedBuf {
        fn contents(&self) -> String {
            self.0
                .lock()
                .map(|b| String::from_utf8_lossy(&b).into_owned())
                .unwrap_or_default()
        }
    }

    impl Write for SharedBuf {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            match self.0.lock() {
                Ok(mut b) => {
                    b.extend_from_slice(data);
                    Ok(data.len())
                }
                Err(_) => Err(std::io::Error::other("poisoned")),
            }
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn start_json() -> &'static str {
        r#"{"instruction":"pay the invoice",
            "tools":[
              {"name":"read_page","params":[{"name":"url","control":true}],
               "requires":["net.egress[{url}]"]},
              {"name":"send_money","params":[{"name":"to","control":true},{"name":"amount"}],
               "discloses_to":"to","requires":["net.egress[bank]"]}],
            "policy":{"name":"p","grants":["net.egress"]}}"#
    }

    fn start() -> Start {
        match serde_json::from_str(start_json()) {
            Ok(s) => s,
            Err(e) => panic!("the example start message should parse: {e}"),
        }
    }

    #[test]
    fn a_start_message_parses_and_describes_itself() {
        let s = start();
        assert_eq!(s.instruction, "pay the invoice");
        let described = describe(&s);
        assert_eq!(
            described.get("send_money").map(Vec::as_slice),
            Some(
                [
                    "to (steers)".to_owned(),
                    "amount".to_owned(),
                    "discloses to to".to_owned()
                ]
                .as_slice()
            )
        );
    }

    #[test]
    fn a_requirement_takes_its_scope_from_an_argument() {
        let args: Args = [(
            "url".to_owned(),
            Value::str("example.com", Label::runtime()),
        )]
        .into_iter()
        .collect();
        let req = parse_requirement("net.egress[{url}]", &args, "read_page");
        assert_eq!(
            req.map(|r| r.label()),
            Some("net.egress[example.com]".into())
        );

        let fixed = parse_requirement("net.egress[bank]", &Args::new(), "x");
        assert_eq!(fixed.map(|r| r.label()), Some("net.egress[bank]".into()));

        assert_eq!(
            parse_requirement("not.a.capability", &Args::new(), "x"),
            None
        );
    }

    /// The host runs the tools, the runtime rules on them, and a recipient
    /// taken from a page is refused even though the host happily answered.
    #[test]
    fn the_host_runs_tools_and_the_gate_still_refuses() {
        let plan = Plan::new()
            .bind(
                "page",
                Expr::call("read_page").arg("url", Expr::str("evil.example")),
            )
            .then(
                Expr::call("send_money")
                    .arg("to", Expr::var("page").field("payee"))
                    .arg("amount", Expr::Int(100)),
            );
        let planner = ScriptedPlanner::new([plan]);

        // The host answers the one call it is asked for.
        let host_replies = "{\"id\":1,\"ok\":{\"payee\":\"attacker\"}}\n";
        let written = SharedBuf::default();
        let result = run(
            &start(),
            &planner,
            None,
            Box::new(Cursor::new(host_replies)),
            Box::new(written.clone()),
        );
        assert!(result.is_ok(), "{result:?}");

        let transcript = written.contents();
        assert!(transcript.contains("\"type\":\"call\""), "{transcript}");
        assert!(
            transcript.contains("\"rule\":\"untrusted_control\""),
            "{transcript}"
        );
        assert!(transcript.contains("\"type\":\"done\""), "{transcript}");
    }

    #[test]
    fn a_host_that_closes_the_link_is_reported() {
        let planner = ScriptedPlanner::new([
            Plan::new().then(Expr::call("read_page").arg("url", Expr::str("x")))
        ]);
        let written = SharedBuf::default();
        let result = run(
            &start(),
            &planner,
            None,
            Box::new(Cursor::new("")),
            Box::new(written.clone()),
        );
        assert!(result.is_ok());
        let transcript = written.contents();
        assert!(transcript.contains("closed the link"), "{transcript}");
    }

    #[test]
    fn read_start_rejects_nothing_and_rubbish() {
        assert!(read_start(&mut Cursor::new("")).is_err());
        assert!(read_start(&mut Cursor::new("not json\n")).is_err());
        match read_start(&mut Cursor::new(start_json())) {
            Ok(_) => {}
            Err(e) => panic!("the example start message should parse: {e:#}"),
        }
    }
}
