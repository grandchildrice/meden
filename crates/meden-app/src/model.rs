//! Session-bound model port. Only implementations' actual consuming processor
//! calls grant inference; model metadata is diagnostic provenance.
use meden_agent::conversation::{ChatModel, Delta, ModelError, Reply};
use meden_runtime::plugin::{Descriptor, Role};
use serde::{Deserialize, Serialize};

#[derive(Serialize, schemars::JsonSchema)]
struct Input {
    messages: Vec<serde_json::Value>,
    tools: Vec<serde_json::Value>,
}
/// Conversation shape, independent of a provider wire format.
pub fn input_schema() -> Result<serde_json::Value, serde_json::Error> {
    serde_json::to_value(schemars::schema_for!(Input))
}
/// Only a complete native reply may carry calls to the agent controller.
pub fn output_schema() -> Result<serde_json::Value, serde_json::Error> {
    serde_json::to_value(schemars::schema_for!(Reply))
}

/// Display/review target, never a grant or a decoded implementation selection.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    /// Selected model identifier.
    pub model: String,
    /// Frozen display destination.
    pub endpoint: String,
}

/// Original host-prepared method and labels. No decoder can create this value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bound {
    definition: Descriptor,
    view: Binding,
}
impl Bound {
    /// Validate the role and both actual port schemas, then freeze the method.
    pub fn new(
        definition: Descriptor,
        model: String,
        endpoint: String,
    ) -> Result<Self, ModelError> {
        if definition.role() != Role::Model
            || !definition
                .accepts_schema(&input_schema().map_err(failed)?)
                .map_err(failed)?
            || !definition
                .accepts_output_schema(&output_schema().map_err(failed)?)
                .map_err(failed)?
        {
            return Err(failed("Model definition does not match its typed role"));
        }
        Ok(Self {
            definition,
            view: Binding { model, endpoint },
        })
    }
    /// Exact original implementation/configuration/schema identity.
    pub fn definition(&self) -> &Descriptor {
        &self.definition
    }
    /// Immutable labels for review and historical records.
    pub fn view(&self) -> &Binding {
        &self.view
    }
}

/// Application implementation supplied by trusted host composition.
/// Metadata/status cannot authorize a request or replace the real transport.
pub trait Model: Send + Sync {
    /// Original prepared implementation; no approval operation is exposed here.
    fn binding(&self) -> &Bound;
    /// Observe the grant without renewing or replacing it.
    fn is_authorized(&self, session: &[u8]) -> bool;
    /// Borrow the actual session caller. Implementations still consume their
    /// original processor call on each exchange.
    fn for_session<'a>(&'a self, session: &'a [u8]) -> Box<dyn ChatModel + 'a>;
}

/// A retained original method and actual caller for one host session.
pub struct Session<'a> {
    source: &'a dyn Model,
    original: Bound,
    caller: Box<dyn ChatModel + 'a>,
}
impl Session<'_> {
    /// Original method for provenance, not a grant.
    pub fn binding(&self) -> &Bound {
        &self.original
    }
    fn check(&self) -> Result<(), ModelError> {
        if self.source.binding() != &self.original {
            return Err(failed("Originally selected model implementation changed"));
        }
        Ok(())
    }
}
impl ChatModel for Session<'_> {
    fn reply(
        &self,
        messages: &[serde_json::Value],
        tools: &[serde_json::Value],
    ) -> Result<Reply, ModelError> {
        self.reply_with_progress(messages, tools, &|_| {})
    }
    fn reply_with_progress(
        &self,
        messages: &[serde_json::Value],
        tools: &[serde_json::Value],
        notify: &dyn Fn(Delta<'_>),
    ) -> Result<Reply, ModelError> {
        self.check()?;
        let reply = self.caller.reply_with_progress(messages, tools, notify)?;
        self.check()?;
        Ok(reply)
    }
}

/// Capture the original method before constructing its actual borrowed caller.
/// Construction grants nothing; dispatch errors remain conversation failures so
/// the original user instruction stays in history even when a binding changed.
pub fn bind<'a>(model: &'a dyn Model, session: &'a [u8]) -> Session<'a> {
    Session {
        source: model,
        original: model.binding().clone(),
        caller: model.for_session(session),
    }
}
/// Capture only the prepared implementation's original review labels.
pub fn binding(model: &dyn Model) -> Binding {
    model.binding().view.clone()
}
/// Status preflight; actual dispatch remains authoritative.
pub fn authorized(judge: Option<&dyn Model>, session: &[u8]) -> bool {
    judge.is_none_or(|model| model.is_authorized(session))
}
fn failed(error: impl std::fmt::Display) -> ModelError {
    ModelError::Unavailable(error.to_string())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use meden_runtime::plugin::Definition;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn definition(role: Role, input: serde_json::Value, output: serde_json::Value) -> Descriptor {
        Descriptor::new(Definition {
            id: "test.model",
            role,
            version: "1",
            formats: vec!["test.model".into()],
            implementation: b"test caller",
            configuration: json!({}),
            input,
            output,
        })
        .expect("definition")
    }
    fn bound(model: &str) -> Bound {
        Bound::new(
            definition(
                Role::Model,
                input_schema().expect("input"),
                output_schema().expect("output"),
            ),
            model.into(),
            "test://host".into(),
        )
        .expect("bound")
    }
    #[test]
    fn binding_requires_the_actual_role_and_both_schemas() {
        for definition in [
            definition(
                Role::Compiler,
                input_schema().expect("input"),
                output_schema().expect("output"),
            ),
            definition(Role::Model, json!({}), output_schema().expect("output")),
            definition(Role::Model, input_schema().expect("input"), json!({})),
        ] {
            assert!(Bound::new(definition, "model".into(), "test://host".into()).is_err());
        }
    }
    struct Fake {
        original: Bound,
        changed: Bound,
        phase: AtomicUsize,
        calls: AtomicUsize,
        drift_on_prepare: bool,
        drift_on_reply: bool,
    }
    impl Fake {
        fn new() -> Self {
            Self {
                original: bound("original"),
                changed: bound("changed"),
                phase: AtomicUsize::new(0),
                calls: AtomicUsize::new(0),
                drift_on_prepare: false,
                drift_on_reply: false,
            }
        }
    }
    impl Model for Fake {
        fn binding(&self) -> &Bound {
            if self.phase.load(Ordering::SeqCst) == 0 {
                &self.original
            } else {
                &self.changed
            }
        }
        fn is_authorized(&self, _: &[u8]) -> bool {
            true
        }
        fn for_session<'a>(&'a self, _: &'a [u8]) -> Box<dyn ChatModel + 'a> {
            if self.drift_on_prepare {
                self.phase.store(1, Ordering::SeqCst);
            }
            Box::new(Caller(self))
        }
    }
    struct Caller<'a>(&'a Fake);
    impl ChatModel for Caller<'_> {
        fn reply(
            &self,
            _: &[serde_json::Value],
            _: &[serde_json::Value],
        ) -> Result<Reply, ModelError> {
            self.0.calls.fetch_add(1, Ordering::SeqCst);
            if self.0.drift_on_reply {
                self.0.phase.store(1, Ordering::SeqCst);
            }
            serde_json::from_value(json!({"content":"complete", "tool_calls":[{"id":"call", "function":{"name":"write_file", "arguments":"{}"}}]})).map_err(failed)
        }
    }
    #[test]
    fn session_retains_original_method_and_rejects_changes_before_dispatch() {
        let source = Fake::new();
        let caller = bind(&source, b"session");
        assert_eq!(caller.binding(), &source.original);
        assert_eq!(caller.reply(&[], &[]).expect("reply").tool_calls.len(), 1);
        source.phase.store(1, Ordering::SeqCst);
        assert_eq!(caller.binding(), &source.original);
        assert!(caller.reply(&[], &[]).is_err());
        assert_eq!(source.calls.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn method_is_frozen_before_caller_preparation() {
        let source = Fake {
            drift_on_prepare: true,
            ..Fake::new()
        };
        let caller = bind(&source, b"session");
        assert_eq!(caller.binding(), &source.original);
        assert!(caller.reply(&[], &[]).is_err());
        assert_eq!(source.calls.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn a_complete_reply_cannot_admit_calls_after_method_drift() {
        let source = Fake {
            drift_on_reply: true,
            ..Fake::new()
        };
        assert!(bind(&source, b"session").reply(&[], &[]).is_err());
        assert_eq!(source.calls.load(Ordering::SeqCst), 1);
    }
}
