//! Host-owned tools, JSON schemas and per-call capability requirements.
use meden_core::Requirement;
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

/// An implementation owned by the host, never supplied by model output.
pub trait Tool {
    /// Definition exposed to the model.
    fn signature(&self) -> &ToolSig;
    /// Capabilities needed by this call.
    fn requirements(&self, args: &Args) -> Vec<Requirement>;
    /// Execute after the runtime admits the call.
    fn call(&self, args: &Args) -> Result<Value, ToolError>;
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
