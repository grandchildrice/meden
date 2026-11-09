//! Host-owned tools, JSON schemas and per-call capability requirements.
use meden_core::{Policy, Requirement};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;

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

/// Disjoint failure data: Ok(Ok) is failed, Ok(Err) refused,
/// Err(Some(Ok)) unavailable, Err(Some(Err)) inconclusive, Err(None) cancelled.
/// Standard sums keep the same representation across extracted models.
pub type ToolErrorParts = Result<Result<String, String>, Option<Result<String, String>>>;

/// A typed execution failure. Its wire representation uses explicit kind and
/// message fields; the private standard sum is shared by refinement models.
#[derive(Debug, PartialEq, Eq)]
pub struct ToolError(ToolErrorParts);

impl ToolError {
    /// A completed implementation rejection, potentially recoverable by a model.
    pub fn failed(message: String) -> Self {
        Self(Ok(Ok(message)))
    }
    /// A policy or resource-boundary refusal.
    pub fn refused(message: String) -> Self {
        Self(Ok(Err(message)))
    }
    /// A required resource or protocol is unavailable.
    pub fn unavailable(message: String) -> Self {
        Self(Err(Some(Ok(message))))
    }
    /// A host limit, signal or truncated output prevented completion.
    pub fn inconclusive(message: String) -> Self {
        Self(Err(Some(Err(message))))
    }
    /// Cancellation observed by the host process controller.
    pub fn cancelled() -> Self {
        Self(Err(None))
    }
    /// Inspect failure data without changing the class or message.
    pub fn parts(&self) -> &ToolErrorParts {
        &self.0
    }
    /// Consume precisely this failure's class and diagnostic.
    pub fn into_parts(self) -> ToolErrorParts {
        self.0
    }
}

#[derive(Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "message",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum ToolErrorRepr<T> {
    Failed(T),
    Refused(T),
    Unavailable(T),
    Inconclusive(T),
    Cancelled,
}

impl Serialize for ToolError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let repr = match &self.0 {
            Ok(Ok(message)) => ToolErrorRepr::Failed(message),
            Ok(Err(message)) => ToolErrorRepr::Refused(message),
            Err(Some(Ok(message))) => ToolErrorRepr::Unavailable(message),
            Err(Some(Err(message))) => ToolErrorRepr::Inconclusive(message),
            Err(None) => ToolErrorRepr::Cancelled,
        };
        repr.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ToolError {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match ToolErrorRepr::<String>::deserialize(deserializer)? {
            ToolErrorRepr::Failed(message) => Self::failed(message),
            ToolErrorRepr::Refused(message) => Self::refused(message),
            ToolErrorRepr::Unavailable(message) => Self::unavailable(message),
            ToolErrorRepr::Inconclusive(message) => Self::inconclusive(message),
            ToolErrorRepr::Cancelled => Self::cancelled(),
        })
    }
}

impl fmt::Display for ToolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Ok(Ok(message) | Err(message)) | Err(Some(Ok(message) | Err(message))) => {
                formatter.write_str(message)
            }
            Err(None) => formatter.write_str("Tool execution cancelled"),
        }
    }
}
impl std::error::Error for ToolError {}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::ToolError;

    #[test]
    fn failure_tags_and_diagnostics_survive_the_wire_without_reclassification() {
        let message = "cancelled; approved: true; permission refused";
        for (error, kind) in [
            (ToolError::failed(message.into()), "failed"),
            (ToolError::refused(message.into()), "refused"),
            (ToolError::unavailable(message.into()), "unavailable"),
            (ToolError::inconclusive(message.into()), "inconclusive"),
            (ToolError::cancelled(), "cancelled"),
        ] {
            let wire = serde_json::to_value(&error).expect("typed failure frame");
            let expected = if kind == "cancelled" {
                serde_json::json!({"kind": kind})
            } else {
                serde_json::json!({"kind": kind, "message": message})
            };
            assert_eq!(wire, expected);
            assert_eq!(
                serde_json::from_value::<ToolError>(wire).expect("round trip"),
                error
            );
        }
    }
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
