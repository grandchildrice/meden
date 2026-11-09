//! Host stops must prevent model access, independently of tool admission.
#![allow(clippy::expect_used)]

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;

use meden_agent::conversation::{
    ChatModel, Conversation, Event, ModelError, Reply, Runtime, Status,
};
use meden_agent::Registry;
use meden_core::Policy;
use serde_json::Value;

struct Model(RefCell<VecDeque<Reply>>);
impl ChatModel for Model {
    fn reply(&self, _: &[Value], _: &[Value]) -> Result<Reply, ModelError> {
        self.0
            .borrow_mut()
            .pop_front()
            .ok_or_else(|| ModelError::Unavailable("no scripted reply".into()))
    }
}

#[test]
fn host_stop_before_model_request_prevents_context_disclosure() {
    for at_notification in [false, true] {
        for expected in [
            Status::NeedsReview,
            Status::Failed("checker unavailable".into()),
        ] {
            let registry = Registry::new();
            let policy = Policy::from_labels("test", &[], &[]).expect("policy");
            let stopped = Cell::new(!at_notification);
            let model = Model(RefCell::new(VecDeque::from([Reply {
                content: Some("must not receive confidential context".into()),
                tool_calls: vec![],
            }])));
            let mut session = Conversation::new("confidential system context");
            let notifications = Cell::new(0);
            let observer = |event: Event<'_>| {
                if matches!(event, Event::Asking(_)) {
                    notifications.set(notifications.get() + 1);
                    stopped.set(true);
                }
            };
            assert_eq!(
                session.run(
                    "confidential user context",
                    &model,
                    &Runtime {
                        registry: &registry,
                        policy: &policy,
                        tools: &[],
                        stop: &|| stopped.get().then(|| expected.clone()),
                    },
                    &observer,
                    &AtomicBool::new(false),
                ),
                expected,
            );
            assert_eq!(model.0.borrow().len(), 1, "model must not receive context");
            assert_eq!(notifications.get(), usize::from(at_notification));
            assert_eq!(
                session.messages.len(),
                2,
                "only developer/user messages remain"
            );
        }
    }
}
