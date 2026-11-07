//! Tools and their signatures.
//!
//! A tool declares, once, which of its parameters *steer* what it does and
//! whether it discloses its arguments to someone. Those two declarations are
//! what the flow rules in [`crate::gate`] check; the capability requirements
//! it reports per call are what the policy in `tagent-core` checks.

use std::collections::BTreeMap;

use tagent_core::Requirement;
use thiserror::Error;

use crate::value::Value;

/// Arguments of a call, by parameter name.
pub type Args = BTreeMap<String, Value>;

/// One parameter of a tool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamSpec {
    /// Parameter name.
    pub name: String,
    /// Does this argument steer what the tool does?
    ///
    /// A recipient, a path, a URL, a shell command: anything that decides
    /// *what happens* rather than merely *what is said*. Untrusted data must
    /// never reach such a parameter, which is what stops a fetched web page
    /// from choosing who an email goes to.
    pub control: bool,
}

impl ParamSpec {
    /// A parameter that steers the call.
    pub fn control(name: &str) -> Self {
        ParamSpec {
            name: name.to_owned(),
            control: true,
        }
    }

    /// A parameter that is only carried, not acted on.
    pub fn data(name: &str) -> Self {
        ParamSpec {
            name: name.to_owned(),
            control: false,
        }
    }
}

/// What a tool declares about itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolSig {
    /// Tool name, also the provenance recorded on its results.
    pub name: String,
    /// Its parameters.
    pub params: Vec<ParamSpec>,
    /// The parameter whose value names the principal this call discloses its
    /// other arguments to, if it discloses them at all.
    ///
    /// `send_email` sets this to `to`: the call hands everything else to
    /// whoever that names, so every other argument must permit that reader.
    pub discloses_to: Option<String>,
}

impl ToolSig {
    /// A signature with no disclosure.
    pub fn new(name: &str, params: Vec<ParamSpec>) -> Self {
        ToolSig {
            name: name.to_owned(),
            params,
            discloses_to: None,
        }
    }

    /// This signature, declaring that it discloses to the principal named by
    /// `param`.
    #[must_use]
    pub fn disclosing_to(mut self, param: &str) -> Self {
        self.discloses_to = Some(param.to_owned());
        self
    }

    /// The named parameter.
    pub fn param(&self, name: &str) -> Option<&ParamSpec> {
        self.params.iter().find(|p| p.name == name)
    }

    /// The parameters that steer the call.
    pub fn control_params(&self) -> impl Iterator<Item = &ParamSpec> {
        self.params.iter().filter(|p| p.control)
    }
}

/// Why a tool failed once it was allowed to run.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ToolError {
    /// A required argument was missing.
    #[error("missing argument {0:?}")]
    MissingArgument(String),
    /// An argument was of the wrong kind.
    #[error("argument {name:?} should be {expected}, got {actual}")]
    WrongKind {
        /// Parameter name.
        name: String,
        /// What the tool wanted.
        expected: &'static str,
        /// What it got.
        actual: &'static str,
    },
    /// The tool failed for its own reasons.
    #[error("{0}")]
    Failed(String),
}

/// Something the interpreter can call.
pub trait Tool {
    /// What this tool declares about itself.
    fn signature(&self) -> &ToolSig;

    /// The capabilities this particular call needs, which the policy then
    /// rules on. Argument values may determine them: a fetch of
    /// `https://api.github.com` needs `net.egress[api.github.com]`.
    fn requirements(&self, args: &Args) -> Vec<Requirement>;

    /// Run the call. Only ever reached after the gate allows it.
    fn call(&self, args: &Args) -> Result<Value, ToolError>;
}

/// The tools an interpreter may call.
#[derive(Default)]
pub struct Registry {
    tools: BTreeMap<String, Box<dyn Tool>>,
}

impl Registry {
    /// An empty registry.
    pub fn new() -> Self {
        Registry::default()
    }

    /// Add a tool, replacing any tool of the same name.
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.signature().name.clone(), tool);
    }

    /// Add a tool, taking `self` by value so registries can be built in one
    /// expression.
    #[must_use]
    pub fn with(mut self, tool: Box<dyn Tool>) -> Self {
        self.register(tool);
        self
    }

    /// The tool of that name.
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(AsRef::as_ref)
    }

    /// Every registered name.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.tools.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    // Tests fail by panicking; that is their job.
    #![allow(clippy::panic)]

    use super::*;
    use crate::label::Label;
    use crate::value::Data;

    struct Echo(ToolSig);

    impl Tool for Echo {
        fn signature(&self) -> &ToolSig {
            &self.0
        }
        fn requirements(&self, _args: &Args) -> Vec<Requirement> {
            vec![]
        }
        fn call(&self, args: &Args) -> Result<Value, ToolError> {
            args.get("text")
                .cloned()
                .ok_or_else(|| ToolError::MissingArgument("text".to_owned()))
        }
    }

    #[test]
    fn registry_round_trip() {
        let sig = ToolSig::new("echo", vec![ParamSpec::data("text")]);
        let reg = Registry::new().with(Box::new(Echo(sig)));
        assert_eq!(reg.names().collect::<Vec<_>>(), vec!["echo"]);
        assert!(reg.get("echo").is_some());
        assert!(reg.get("nope").is_none());

        let Some(tool) = reg.get("echo") else {
            panic!("echo should be registered")
        };
        let args: Args = [("text".to_owned(), Value::str("hi", Label::user()))]
            .into_iter()
            .collect();
        assert_eq!(tool.call(&args).map(|v| v.data), Ok(Data::Str("hi".into())));
        assert_eq!(
            tool.call(&Args::new()),
            Err(ToolError::MissingArgument("text".to_owned()))
        );
    }

    #[test]
    fn signature_declares_control_and_disclosure() {
        let sig = ToolSig::new(
            "send_email",
            vec![ParamSpec::control("to"), ParamSpec::data("body")],
        )
        .disclosing_to("to");
        assert_eq!(sig.discloses_to.as_deref(), Some("to"));
        assert_eq!(
            sig.control_params()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            vec!["to"]
        );
        assert!(sig.param("body").is_some_and(|p| !p.control));
        assert!(sig.param("missing").is_none());
    }
}
