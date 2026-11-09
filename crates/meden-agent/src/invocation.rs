//! Preparation failure data; execution authority is owned by meden_core::invocation.

#[derive(Clone, Copy)]
pub(crate) enum Rule {
    UnknownTool,
    InvalidArguments,
    MissingArgument,
    Policy,
}

impl Rule {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::UnknownTool => "unknown_tool",
            Self::InvalidArguments => "invalid_arguments",
            Self::MissingArgument => "missing_argument",
            Self::Policy => "policy",
        }
    }
}

pub(crate) struct Refusal {
    pub(crate) rule: Rule,
    pub(crate) reason: String,
}
