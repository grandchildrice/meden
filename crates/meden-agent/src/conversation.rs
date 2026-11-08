//! Ordinary conversation and tool execution, with capability checks.
//!
//! The model reads tool results. Capability checks constrain declared effects;
//! they do not establish outbound confidentiality or trustworthy model choices.

use std::sync::atomic::{AtomicBool, Ordering};

use meden_core::{check, Policy};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as Json};

use crate::{tool::Args, Registry};

/// An API function call. Arguments remain text until execution, so malformed
/// JSON can be returned to the model as an error with its original call id.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCall {
    /// Correlates a call with its result.
    pub id: String,
    /// Function details.
    pub function: FunctionCall,
}

/// The function name and its JSON-encoded arguments.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionCall {
    /// Registered tool name.
    pub name: String,
    /// Arguments as returned by the model API.
    pub arguments: String,
}

/// One assistant reply, including every call in a parallel-call group.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Reply {
    /// Optional accompanying text.
    pub content: Option<String>,
    /// Calls to execute before the next model request.
    #[serde(default, deserialize_with = "calls_or_empty")]
    pub tool_calls: Vec<ToolCall>,
}

fn calls_or_empty<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<ToolCall>, D::Error> {
    Ok(Option::<Vec<ToolCall>>::deserialize(d)?.unwrap_or_default())
}

/// Tools and capability policy for one execution environment.
pub struct Runtime<'a> {
    /// Implementations the agent may invoke.
    pub registry: &'a Registry,
    /// Capabilities granted by the operator.
    pub policy: &'a Policy,
    /// API definitions, in the host's original order.
    pub tools: &'a [Json],
}

impl Reply {
    fn message(&self) -> Json {
        let mut message =
            json!({"role":"assistant", "content": self.content.as_deref().map(content)});
        if let Some(object) = message
            .as_object_mut()
            .filter(|_| !self.tool_calls.is_empty())
        {
            object.insert(
                "tool_calls".to_owned(),
                Json::Array(
                    self.tool_calls
                        .iter()
                        .map(|call| {
                            json!({"id":call.id, "type":"function", "function":{
                                "name":call.function.name,
                                "arguments":crate::chat_json::arguments(&call.function.arguments)
                            }})
                        })
                        .collect(),
                ),
            );
        }
        message
    }
}

/// Text content blocks, also used by AgentDojo's OpenAI client.
pub fn content(text: &str) -> Json {
    json!([{"type":"text", "text":text}])
}

/// Model transport or response failure.
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    /// The model endpoint could not complete the request.
    #[error("model unavailable: {0}")]
    Unavailable(String),
    /// The response cannot be used as an assistant message.
    #[error("invalid model response: {0}")]
    InvalidResponse(String),
}

/// A model that supports native function calling.
pub trait ChatModel {
    /// Generate the next assistant message from the full conversation.
    fn reply(&self, messages: &[Json], tools: &[Json]) -> Result<Reply, ModelError>;
}

/// Session completion, distinct from tool errors the model may recover from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// A final assistant answer.
    Done(String),
    /// Model access failed.
    Failed(String),
    /// The model still wanted tools at the execution limit.
    Incomplete,
    /// Cancellation was requested; no later tools were started.
    Cancelled,
}

/// Progress events for a protocol host or terminal interface.
pub enum Event<'a> {
    /// A model request is about to start.
    Asking(usize),
    /// An assistant message has arrived, before any of its tools execute.
    Assistant(&'a Reply),
    /// A tool has been checked, before it executes.
    Decision {
        /// The model's call.
        call: &'a ToolCall,
        /// None when allowed; a rule and explanation otherwise.
        refusal: Option<(&'static str, &'a str)>,
    },
    /// A tool finished, or returned a recoverable error.
    ToolResult {
        /// The original call.
        call: &'a ToolCall,
        /// Its visible result.
        text: &'a str,
        /// Whether execution failed.
        error: bool,
    },
}

/// Persistent conversation history. Reuse it across user turns in a TUI.
pub struct Conversation {
    /// Exact messages supplied to the model, available for audit and replay.
    pub messages: Vec<Json>,
    /// Maximum number of tool-execution groups per user turn.
    pub max_iterations: usize,
}

impl Conversation {
    /// Begin a conversation with a host-owned system instruction.
    pub fn new(system: &str) -> Self {
        Self {
            messages: vec![json!({"role":"developer", "content":content(system)})],
            max_iterations: 15,
        }
    }

    /// Carry out another user instruction. Tool errors and denials are fed
    /// back with their call ids so the model can correct its arguments.
    pub fn run(
        &mut self,
        instruction: &str,
        model: &dyn ChatModel,
        runtime: &Runtime<'_>,
        observer: &dyn Fn(Event<'_>),
        cancelled: &AtomicBool,
    ) -> Status {
        self.messages
            .push(json!({"role":"user", "content":content(instruction)}));
        for iteration in 0..=self.max_iterations {
            if cancelled.load(Ordering::Relaxed) {
                return Status::Cancelled;
            }
            observer(Event::Asking(iteration.saturating_add(1)));
            if cancelled.load(Ordering::Relaxed) {
                return Status::Cancelled;
            }
            let reply = match model.reply(&self.messages, runtime.tools) {
                Ok(reply) => reply,
                Err(_) if cancelled.load(Ordering::Relaxed) => return Status::Cancelled,
                Err(error) => return Status::Failed(error.to_string()),
            };
            observer(Event::Assistant(&reply));
            self.messages.push(reply.message());
            if reply.tool_calls.is_empty() {
                if cancelled.load(Ordering::Relaxed) {
                    return Status::Cancelled;
                }
                return Status::Done(reply.content.unwrap_or_default());
            }
            // Even cancelled/incomplete assistant groups need paired tool
            // messages, otherwise the next user turn is invalid API history.
            let stopped = cancelled.load(Ordering::Relaxed) || iteration == self.max_iterations;
            for call in &reply.tool_calls {
                let result = if stopped || cancelled.load(Ordering::Relaxed) {
                    Err(("cancelled", "Tool not executed: turn stopped".to_owned()))
                } else {
                    execute(call, runtime.registry, runtime.policy, observer, cancelled)
                };
                let (text, error) = match result {
                    Ok(text) => (text, false),
                    Err((_, text)) => (text, true),
                };
                observer(Event::ToolResult {
                    call,
                    text: &text,
                    error,
                });
                self.messages.push(json!({
                    "role":"tool", "tool_call_id":call.id,
                    "name":call.function.name,
                    "content": if error { Json::String(text) } else { content(&text) }
                }));
            }
            if cancelled.load(Ordering::Relaxed) {
                return Status::Cancelled;
            }
            if stopped {
                return Status::Incomplete;
            }
        }
        Status::Incomplete
    }
}

fn execute(
    call: &ToolCall,
    registry: &Registry,
    policy: &Policy,
    observer: &dyn Fn(Event<'_>),
    cancelled: &AtomicBool,
) -> Result<String, (&'static str, String)> {
    let checked = (|| {
        let tool = registry.get(&call.function.name).ok_or_else(|| {
            (
                "unknown_tool",
                format!("Unknown tool: {}", call.function.name),
            )
        })?;
        let json: Json = serde_json::from_str(&call.function.arguments)
            .map_err(|e| ("invalid_arguments", format!("Invalid JSON arguments: {e}")))?;
        let object = json.as_object().ok_or_else(|| {
            (
                "invalid_arguments",
                "Arguments must be an object".to_owned(),
            )
        })?;
        let args: Args = object
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        if let Some(required) = tool
            .signature()
            .parameters
            .get("required")
            .and_then(Json::as_array)
        {
            for name in required.iter().filter_map(Json::as_str) {
                if !args.contains_key(name) {
                    return Err(("missing_argument", format!("Missing argument: {name}")));
                }
            }
        }
        let decision = check(&tool.requirements(&args), policy);
        if !decision.allowed {
            return Err((
                "policy",
                "The capability policy refused this call".to_owned(),
            ));
        }
        Ok((tool, args))
    })();
    match checked {
        Err((rule, explanation)) => {
            observer(Event::Decision {
                call,
                refusal: Some((rule, &explanation)),
            });
            Err((rule, explanation))
        }
        Ok((tool, args)) => {
            observer(Event::Decision {
                call,
                refusal: None,
            });
            if cancelled.load(Ordering::Relaxed) {
                return Err(("cancelled", "Tool not executed: turn stopped".into()));
            }
            let result = tool
                .call(&args)
                .map_err(|e| ("tool_error", e.to_string()))?;
            Ok(result
                .as_str()
                .map_or_else(|| result.to_string(), str::to_owned))
        }
    }
}

/// Native API definitions in a host-selected order.
pub fn tool_definitions(registry: &Registry, order: &[String]) -> Vec<Json> {
    order.iter().filter_map(|name| registry.get(name)).map(|tool| {
        let sig = tool.signature();
        let schema = &sig.parameters;
        json!({"type":"function", "function":{"name":sig.name, "description":sig.description, "parameters":schema}})
    }).collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::tool::{Tool, ToolError, ToolSig};
    use meden_core::{Capability, Requirement};
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;
    use std::rc::Rc;

    struct Model(RefCell<VecDeque<Reply>>);
    impl ChatModel for Model {
        fn reply(&self, _: &[Json], _: &[Json]) -> Result<Reply, ModelError> {
            self.0
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| ModelError::Unavailable("no scripted reply".into()))
        }
    }
    struct WriteTool {
        sig: ToolSig,
        calls: Rc<Cell<usize>>,
    }
    impl Tool for WriteTool {
        fn signature(&self) -> &ToolSig {
            &self.sig
        }
        fn requirements(&self, _: &Args) -> Vec<Requirement> {
            vec![Requirement::with_evidence(
                Capability::FsWrite,
                Some("/workspace/out"),
                "write",
            )]
        }
        fn call(&self, _: &Args) -> Result<Json, ToolError> {
            self.calls.set(self.calls.get().saturating_add(1));
            Ok(json!("written"))
        }
    }
    fn call(id: &str, arguments: &str) -> ToolCall {
        ToolCall {
            id: id.into(),
            function: FunctionCall {
                name: "write".into(),
                arguments: arguments.into(),
            },
        }
    }
    fn registry(calls: &Rc<Cell<usize>>) -> Registry {
        Registry::new().with(Box::new(WriteTool {
            sig: ToolSig::new("write", "Write text", json!({"type":"object", "properties":{"text":{"type":"string"}}, "required":["text"]})),
            calls: Rc::clone(calls),
        }))
    }
    #[test]
    fn denials_and_malformed_arguments_never_execute_and_keep_call_ids() {
        let count = Rc::new(Cell::new(0));
        let registry = registry(&count);
        let policy = Policy::default();
        let model = Model(RefCell::new(VecDeque::from([
            Reply {
                content: None,
                tool_calls: vec![call("bad-json", "{"), call("denied", r#"{"text":"hello"}"#)],
            },
            Reply {
                content: Some("I could not write".into()),
                tool_calls: vec![],
            },
        ])));
        let mut session = Conversation::new("test");
        let status = session.run(
            "write",
            &model,
            &Runtime {
                registry: &registry,
                policy: &policy,
                tools: &[],
            },
            &|_| {},
            &AtomicBool::new(false),
        );
        assert_eq!(status, Status::Done("I could not write".into()));
        assert_eq!(count.get(), 0);
        let results: Vec<_> = session
            .messages
            .iter()
            .filter(|m| m.get("role") == Some(&json!("tool")))
            .collect();
        assert_eq!(results.len(), 2);
        assert_eq!(
            results.first().and_then(|m| m.get("tool_call_id")),
            Some(&json!("bad-json"))
        );
        assert!(results
            .last()
            .expect("denial result")
            .to_string()
            .contains("policy refused"));
    }
    #[test]
    fn cancellation_between_calls_preserves_history_and_stops_side_effects() {
        let count = Rc::new(Cell::new(0));
        let registry = registry(&count);
        let policy = Policy::from_labels("write", &["fs.write"], &[]).expect("policy");
        let cancelled = AtomicBool::new(false);
        let model = Model(RefCell::new(VecDeque::from([Reply {
            content: None,
            tool_calls: vec![
                call("first", r#"{"text":"hello"}"#),
                call("second", r#"{"text":"hello"}"#),
            ],
        }])));
        let mut session = Conversation::new("test");
        let observer = |event: Event<'_>| {
            if matches!(event, Event::ToolResult { .. }) {
                cancelled.store(true, Ordering::Relaxed);
            }
        };
        let status = session.run(
            "write",
            &model,
            &Runtime {
                registry: &registry,
                policy: &policy,
                tools: &[],
            },
            &observer,
            &cancelled,
        );
        assert_eq!(status, Status::Cancelled);
        assert_eq!(count.get(), 1);
        assert_eq!(
            session
                .messages
                .iter()
                .filter(|m| m.get("role") == Some(&json!("tool")))
                .count(),
            2
        );
    }
    #[test]
    fn observer_cancellation_stops_the_immediately_following_model_or_tool_effect() {
        for at_request in [true, false] {
            let count = Rc::new(Cell::new(0));
            let registry = registry(&count);
            let policy = Policy::from_labels("write", &["fs.write"], &[]).expect("policy");
            let cancelled = AtomicBool::new(false);
            let model = Model(RefCell::new(VecDeque::from([Reply {
                content: None,
                tool_calls: vec![call("pending", r#"{"text":"hello"}"#)],
            }])));
            let mut session = Conversation::new("test");
            let observer = |event: Event<'_>| {
                if matches!(event, Event::Asking(_)) && at_request
                    || matches!(event, Event::Decision { .. }) && !at_request
                {
                    cancelled.store(true, Ordering::Relaxed);
                }
            };
            assert_eq!(
                session.run(
                    "write",
                    &model,
                    &Runtime {
                        registry: &registry,
                        policy: &policy,
                        tools: &[]
                    },
                    &observer,
                    &cancelled
                ),
                Status::Cancelled,
            );
            assert_eq!(count.get(), 0);
            assert_eq!(model.0.borrow().len(), usize::from(at_request));
        }
    }
    #[test]
    fn execution_limit_is_not_success_and_runs_no_extra_tools() {
        let count = Rc::new(Cell::new(0));
        let registry = registry(&count);
        let policy = Policy::from_labels("write", &["fs.write"], &[]).expect("policy");
        let model = Model(RefCell::new(VecDeque::from([Reply {
            content: None,
            tool_calls: vec![call("extra", r#"{"text":"hello"}"#)],
        }])));
        let mut session = Conversation::new("test");
        session.max_iterations = 0;
        assert_eq!(
            session.run(
                "write",
                &model,
                &Runtime {
                    registry: &registry,
                    policy: &policy,
                    tools: &[]
                },
                &|_| {},
                &AtomicBool::new(false)
            ),
            Status::Incomplete
        );
        assert_eq!(count.get(), 0);
        assert_eq!(
            session.messages.last().and_then(|m| m.get("tool_call_id")),
            Some(&json!("extra"))
        );
    }
}
