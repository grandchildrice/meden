//! Ordinary conversation and tool execution, with capability checks.
//!
//! The model reads tool results. Capability checks constrain declared effects;
//! they do not establish outbound confidentiality or trustworthy model choices.

use std::sync::atomic::{AtomicBool, Ordering};

use meden_core::{check, Policy};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as Json};

use crate::flow::{interrupt, Flow, FlowEffect, FlowInput, FlowOutcome};
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
    /// Host-derived stop before another model/tool effect. This is never
    /// decoded from model arguments or diagnostic text.
    pub stop: &'a dyn Fn() -> Option<Status>,
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
    /// Model access or a required host service failed.
    Failed(String),
    /// The model still wanted tools at the execution limit.
    Incomplete,
    /// Cancellation was requested; no later tools were started.
    Cancelled,
    /// The host requires a user decision before continuing this conversation.
    NeedsReview,
}

impl Status {
    /// Permit host inspection after normal completion or tool-budget exhaustion.
    /// `Some(true)` requires a fresh check; this signal is not proof of a project.
    pub fn inspection(&self) -> Option<bool> {
        match self {
            Self::Done(_) => Some(false),
            Self::Incomplete => Some(true),
            Self::Failed(_) | Self::Cancelled | Self::NeedsReview => None,
        }
    }
}

/// A tool error remains recoverable; a host stop must survive later callbacks.
#[derive(Debug)]
pub enum ExecutionError {
    /// Invalid arguments, an unknown tool or a capability refusal.
    Refused(String),
    /// The registered implementation failed after dispatch.
    Tool(String),
    /// The host stopped execution before the implementation was called.
    Stopped(Status),
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
        let mut flow = Flow::begin(self.max_iterations);
        let mut input = FlowInput::Start;
        loop {
            match flow.advance(
                &mut input,
                cancelled.load(Ordering::Relaxed),
                (runtime.stop)(),
            ) {
                FlowEffect::Asking(attempt) => {
                    observer(Event::Asking(attempt));
                    input = FlowInput::Observed;
                }
                FlowEffect::Model => {
                    input = match model.reply(&self.messages, runtime.tools) {
                        Ok(reply) => FlowInput::Reply(reply),
                        Err(error) => FlowInput::ModelFailed(Status::Failed(error.to_string())),
                    };
                }
                FlowEffect::Assistant => {
                    let reply = flow.reply();
                    observer(Event::Assistant(reply));
                    self.messages.push(reply.message());
                    input = FlowInput::Observed;
                }
                FlowEffect::Tool { execute: permitted } => {
                    let Some(call) = flow.pending_call() else {
                        return Status::Failed("conversation call unavailable".into());
                    };
                    let result = if permitted {
                        execute(
                            call,
                            runtime.registry,
                            runtime.policy,
                            observer,
                            cancelled,
                            runtime.stop,
                        )
                    } else {
                        Err(ExecutionError::Stopped(Status::Incomplete))
                    };
                    input = FlowInput::ToolResult(result);
                }
                FlowEffect::Record => {
                    let (Some(call), Some(result)) = (flow.pending_call(), flow.result()) else {
                        return Status::Failed("conversation result unavailable".into());
                    };
                    let (text, error) = match result {
                        Ok(text) => (text.as_str(), false),
                        Err(ExecutionError::Tool(text) | ExecutionError::Refused(text)) => {
                            (text.as_str(), true)
                        }
                        Err(ExecutionError::Stopped(_)) => {
                            ("Tool not executed: turn stopped", true)
                        }
                    };
                    observer(Event::ToolResult { call, text, error });
                    self.messages.push(json!({
                        "role":"tool", "tool_call_id":call.id,
                        "name":call.function.name,
                        "content": if error { Json::String(text.to_owned()) } else { content(text) }
                    }));
                    input = FlowInput::Observed;
                }
                FlowEffect::Stop => {
                    return match flow.into_outcome() {
                        FlowOutcome::Finish(text) => Status::Done(text.unwrap_or_default()),
                        FlowOutcome::Stopped(status) => status,
                        FlowOutcome::Fault => {
                            Status::Failed("conversation protocol violated".into())
                        }
                    }
                }
            }
        }
    }
}

/// Execute a registered call through the same capability and cancellation
/// checks used by model turns. Controllers use this for independent checks;
/// it does not register implementations or grant additional authority.
pub fn execute(
    call: &ToolCall,
    registry: &Registry,
    policy: &Policy,
    observer: &dyn Fn(Event<'_>),
    cancelled: &AtomicBool,
    stop: &dyn Fn() -> Option<Status>,
) -> Result<String, ExecutionError> {
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
            Err(ExecutionError::Refused(explanation))
        }
        Ok((tool, args)) => {
            observer(Event::Decision {
                call,
                refusal: None,
            });
            if let Some(status) = interrupt(cancelled.load(Ordering::Relaxed), stop()) {
                return Err(ExecutionError::Stopped(status));
            }
            let result = tool
                .call(&args)
                .map_err(|e| ExecutionError::Tool(e.to_string()))?;
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
                stop: &|| None,
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
                stop: &|| None,
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
                        tools: &[],
                        stop: &|| None,
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
                    tools: &[],
                    stop: &|| None,
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

    #[test]
    fn host_stop_observed_during_decision_prevents_dispatch_and_stays_latched() {
        for clear_after_result in [false, true] {
            let count = Rc::new(Cell::new(0));
            let registry = registry(&count);
            let policy = Policy::from_labels("write", &["fs.write"], &[]).expect("policy");
            let stop = Cell::new(false);
            let model = Model(RefCell::new(VecDeque::from([Reply {
                content: None,
                tool_calls: vec![
                    call("one", r#"{"text":"one"}"#),
                    call("two", r#"{"text":"two"}"#),
                ],
            }])));
            let mut session = Conversation::new("test");
            let observer = |event: Event<'_>| match event {
                Event::Decision { .. } => stop.set(true),
                Event::ToolResult { .. } if clear_after_result => stop.set(false),
                _ => {}
            };
            let status = session.run(
                "work",
                &model,
                &Runtime {
                    registry: &registry,
                    policy: &policy,
                    tools: &[],
                    stop: &|| stop.get().then_some(Status::NeedsReview),
                },
                &observer,
                &AtomicBool::new(false),
            );
            assert_eq!(count.get(), 0, "host stop must be rechecked after Decision");
            assert_eq!(status, Status::NeedsReview);
            let ids: Vec<_> = session
                .messages
                .iter()
                .filter(|m| m["role"] == "tool")
                .map(|m| m["tool_call_id"].clone())
                .collect();
            assert_eq!(ids, vec![json!("one"), json!("two")]);
            assert!(model.0.borrow().is_empty());
        }
    }
    #[test]
    fn host_stop_skips_remaining_tools_and_preserves_paired_history() {
        for stop in [
            Status::NeedsReview,
            Status::Failed("checker unavailable".into()),
        ] {
            let count = Rc::new(Cell::new(0));
            let registry = registry(&count);
            let policy = Policy::from_labels("write", &["fs.write"], &[]).expect("policy");
            let model = Model(RefCell::new(VecDeque::from([
                Reply {
                    content: None,
                    tool_calls: vec![
                        call("one", r#"{"text":"one"}"#),
                        call("two", r#"{"text":"two"}"#),
                    ],
                },
                Reply {
                    content: Some("continued after host review".into()),
                    tool_calls: vec![],
                },
            ])));
            let mut session = Conversation::new("test");
            assert_eq!(
                session.run(
                    "propose",
                    &model,
                    &Runtime {
                        registry: &registry,
                        policy: &policy,
                        tools: &[],
                        stop: &|| (count.get() > 0).then(|| stop.clone())
                    },
                    &|_| {},
                    &AtomicBool::new(false)
                ),
                stop
            );
            assert_eq!(count.get(), 1);
            assert_eq!(model.0.borrow().len(), 1);
            let results: Vec<_> = session
                .messages
                .iter()
                .filter(|m| m["role"] == "tool")
                .map(|m| m["tool_call_id"].clone())
                .collect();
            assert_eq!(results, vec![json!("one"), json!("two")]);
            assert_eq!(
                session.run(
                    "host accepted",
                    &model,
                    &Runtime {
                        registry: &registry,
                        policy: &policy,
                        tools: &[],
                        stop: &|| None
                    },
                    &|_| {},
                    &AtomicBool::new(false)
                ),
                Status::Done("continued after host review".into())
            );
        }
    }
}
