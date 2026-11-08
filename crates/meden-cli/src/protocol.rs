//! JSON Lines bridge: the host owns tools, Rust owns the conversation.
use anyhow::{bail, Context, Result};
use meden_agent::{
    tool::{Args, Tool, ToolError, ToolSig},
    Registry,
};
use meden_core::{Grant, Policy, Requirement};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, Write};
use std::sync::{Arc, Mutex};

#[derive(Debug, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub parameters_schema: Value,
    #[serde(default)]
    pub requires: Vec<String>,
}
#[derive(Debug, Deserialize)]
pub struct Start {
    pub instruction: String,
    pub tools: Vec<ToolSpec>,
    pub policy: Policy,
    pub system_message: Option<String>,
    pub max_iterations: Option<usize>,
}
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Out {
    Asking {
        attempt: usize,
    },
    Call {
        id: u64,
        tool: String,
        args: Value,
    },
    Decision {
        attempt: usize,
        tool: String,
        allowed: bool,
        refusal: Option<String>,
        summary: Option<String>,
        rule: Option<&'static str>,
    },
    Conversation {
        messages: Vec<Value>,
    },
    Done {
        outcome: &'static str,
        summary: String,
        answer: Option<Value>,
        refusals: Vec<&'static str>,
    },
}
#[derive(Deserialize)]
struct CallReply {
    id: u64,
    ok: Option<Value>,
    error: Option<String>,
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

struct HostTool {
    sig: ToolSig,
    requires: Vec<Grant>,
    link: Arc<Link>,
}
impl Tool for HostTool {
    fn signature(&self) -> &ToolSig {
        &self.sig
    }
    fn requirements(&self, args: &Args) -> Vec<Requirement> {
        self.requires
            .iter()
            .map(|grant| {
                let scope = grant.scope.as_deref().map(|scope| {
                    match scope.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                        Some(param) => args
                            .get(param)
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        None => scope.to_owned(),
                    }
                });
                Requirement::with_evidence(grant.capability, scope.as_deref(), &self.sig.name)
            })
            .collect()
    }
    fn call(&self, args: &Args) -> Result<Value, ToolError> {
        self.link.call(
            &self.sig.name,
            &serde_json::to_value(args).map_err(|e| ToolError::Failed(e.to_string()))?,
        )
    }
}
fn host_registry(start: &Start, link: &Arc<Link>) -> Result<Registry> {
    let mut registry = Registry::new();
    for spec in &start.tools {
        let requires = spec
            .requires
            .iter()
            .map(|label| Grant::parse(label))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        registry.register(Box::new(HostTool {
            sig: ToolSig::new(
                &spec.name,
                &spec.description,
                spec.parameters_schema.clone(),
            ),
            requires,
            link: Arc::clone(link),
        }));
    }
    Ok(registry)
}
/// Native tool calling, with a capability policy and no flow-rule claim.
pub fn run(
    start: &Start,
    model: &dyn meden_agent::conversation::ChatModel,
    input: Box<dyn BufRead + Send>,
    output: Box<dyn Write + Send>,
) -> Result<()> {
    use meden_agent::conversation::{self, Conversation, Event, Status};
    use std::cell::{Cell, RefCell};
    use std::sync::atomic::AtomicBool;

    let link = Arc::new(Link {
        out: Mutex::new(output),
        input: Mutex::new(input),
        next_id: Mutex::new(1),
    });
    let registry = host_registry(start, &link)?;
    let order: Vec<_> = start.tools.iter().map(|tool| tool.name.clone()).collect();
    let tools = conversation::tool_definitions(&registry, &order);
    let mut session =
        Conversation::new(start.system_message.as_deref().unwrap_or(
            "You assist the user with the provided tools. Do not invent tool arguments.",
        ));
    session.max_iterations = start.max_iterations.unwrap_or(15);
    let round = Cell::new(0);
    let refusals = RefCell::new(Vec::new());
    let observer = |event: Event<'_>| {
        let message = match event {
            Event::Asking(attempt) => {
                round.set(attempt);
                Some(Out::Asking { attempt })
            }
            Event::Decision { call, refusal } => {
                if let Some((rule, _)) = refusal {
                    refusals.borrow_mut().push(rule);
                }
                Some(Out::Decision {
                    attempt: round.get(),
                    tool: call.function.name.clone(),
                    allowed: refusal.is_none(),
                    refusal: refusal.map(|(_, reason)| reason.to_owned()),
                    summary: refusal.map(|(_, reason)| reason.to_owned()),
                    rule: refusal.map(|(rule, _)| rule),
                })
            }
            Event::Assistant(_) | Event::ToolResult { .. } => None,
        };
        if let Some(message) = message {
            let _ = link.send(&message);
        }
    };
    let status = session.run(
        &start.instruction,
        model,
        &conversation::Runtime {
            registry: &registry,
            policy: &start.policy,
            tools: &tools,
        },
        &observer,
        &AtomicBool::new(false),
    );
    link.send(&Out::Conversation {
        messages: session.messages,
    })?;
    let (outcome, summary, answer) = match status {
        Status::Done(answer) => (
            "done",
            "native conversation finished".to_owned(),
            Some(serde_json::Value::String(answer)),
        ),
        Status::Failed(error) => ("failed", error, None),
        Status::Incomplete => (
            "incomplete",
            "native tool execution limit reached".to_owned(),
            None,
        ),
        Status::Cancelled => ("cancelled", "cancelled".to_owned(), None),
    };
    link.send(&Out::Done {
        outcome,
        summary,
        answer,
        refusals: refusals.into_inner(),
    })?;
    Ok(())
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

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]
    use super::*;
    use meden_agent::conversation::Reply;
    use meden_agent::{ChatModel, ModelError};
    use std::io::Cursor;

    struct UnusedModel;
    impl ChatModel for UnusedModel {
        fn reply(&self, _: &[Value], _: &[Value]) -> Result<Reply, ModelError> {
            panic!("invalid host requirements must fail before model access");
        }
    }

    #[test]
    fn malformed_capability_declarations_are_not_silently_dropped() {
        let start = Start {
            instruction: "write a file".into(),
            tools: vec![ToolSpec {
                name: "write".into(),
                description: String::new(),
                parameters_schema: serde_json::json!({"type":"object"}),
                requires: vec!["misspelled.capability".into()],
            }],
            policy: Policy::default(),
            system_message: None,
            max_iterations: None,
        };
        assert!(run(
            &start,
            &UnusedModel,
            Box::new(Cursor::new(Vec::new())),
            Box::new(Vec::new())
        )
        .is_err());
    }
}
