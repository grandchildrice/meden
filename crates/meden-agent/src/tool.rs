//! Host-owned tools, JSON schemas and per-call capability requirements.
use meden_core::{Policy, Requirement};
use serde_json::Value;
use std::collections::BTreeMap;
use thiserror::Error;

/// Named JSON arguments; the host tool validates its accepted value types.
pub type Args = BTreeMap<String, Value>;

/// Model-facing definition of a registered tool.
#[derive(Clone, Debug)]
pub struct ToolSig {
    /// Stable name.
    pub name: String,
    /// Model-facing description.
    pub description: String,
    /// Argument JSON Schema, including required fields and defaults.
    pub parameters: Value,
}
impl ToolSig {
    /// A tool definition with its complete argument schema.
    pub fn new(name: &str, description: &str, parameters: Value) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}

/// An execution error returned to the model for recovery.
#[derive(Debug, Error)]
pub enum ToolError {
    /// Host validation or execution failed.
    #[error("{0}")]
    Failed(String),
}

pub(crate) type Executor<'a> = Box<dyn FnOnce(&Policy) -> Result<Value, ToolError> + 'a>;

/// A frozen host call. Preparation exposes no public execution method;
/// the conversation must obtain its consuming executor through policy admission.
///
/// ```compile_fail
/// use meden_agent::tool::Prepared;
/// let call = Prepared::new(((), vec![]), |(), _| Ok(serde_json::Value::Null));
/// let (execute, _) = call.into_parts();
/// execute();
/// ```
pub struct Prepared<'a> {
    execute: Executor<'a>,
    requirements: Vec<Requirement>,
}

impl<'a> Prepared<'a> {
    /// Bind owned request data and its requirements to a one-use implementation.
    /// Execution receives the same immutable policy used for admission, so an
    /// adapter can check additional resource facts observed inside its worker.
    pub fn new<T: 'a>(
        (request, requirements): (T, Vec<Requirement>),
        execute: impl FnOnce(T, &Policy) -> Result<Value, ToolError> + 'a,
    ) -> Self {
        Self {
            execute: Box::new(move |rules| execute(request, rules)),
            requirements,
        }
    }

    pub(crate) fn into_parts(self) -> (Executor<'a>, Vec<Requirement>) {
        (self.execute, self.requirements)
    }
}

/// An implementation owned by the host, never supplied by model output.
pub trait Tool {
    /// Definition exposed to the model.
    fn signature(&self) -> &ToolSig;
    /// Decode and freeze a request with its declared effects. Preparation must
    /// perform no tool effect; execution is deferred until policy admission.
    fn prepare(&self, args: Args) -> Result<Prepared<'_>, ToolError>;
}

/// Implementations available to a conversation.
#[derive(Default)]
pub struct Registry {
    tools: BTreeMap<String, Box<dyn Tool>>,
}
impl Registry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }
    /// Register a host-owned implementation.
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.signature().name.clone(), tool);
    }
    /// Build a registry with a tool.
    #[must_use]
    pub fn with(mut self, tool: Box<dyn Tool>) -> Self {
        self.register(tool);
        self
    }
    /// Look up an implementation.
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(AsRef::as_ref)
    }
    /// Registered names in stable order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.tools.keys().map(String::as_str)
    }
}
