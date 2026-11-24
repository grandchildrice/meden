//! Untrusted portable work and conversation data, shared by application adapters.
//! These records cannot restore grants, acceptance or admitted proof evidence.
/// Host-bound conversation checkpoints, separate from portable repository data.
pub mod history;
/// Original offline Lake executor; results are historical observations.
pub mod lake;
/// Policy-bound portable record storage and original check workflow.
pub mod store;
mod tool;
pub use tool::register;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A presentation speaker, never a model instruction role.
#[derive(Clone, Copy, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Speaker {
    User,
    Assistant,
}
/// Original transcript text. Interpretation and display remain adapters.
#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub speaker: Speaker,
    pub text: String,
}

#[derive(Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Work {
    pub id: String,
    pub title: String,
    pub requirement: String,
    pub module: String,
    pub theorems: Vec<String>,
    pub next: String,
}
#[derive(Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub id: String,
    pub task: String,
    pub time: u64,
    pub note: String,
    pub check: Option<Check>,
}
#[derive(Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Check {
    pub engine_sha256: String,
    pub requirement: String,
    pub theorems: Vec<String>,
    pub inspection: String,
    pub source_revision: String,
    pub after_revision: Option<String>,
    pub module: String,
    pub success: bool,
    pub termination: String,
    pub stdout: String,
    pub stderr: String,
}
#[derive(Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Conversation {
    pub id: String,
    pub messages: Vec<Message>,
}
#[derive(Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Document {
    #[schemars(range(min = 1, max = 1))]
    pub version: u8,
    pub name: String,
    pub tasks: Vec<Work>,
    pub events: Vec<Event>,
    pub conversations: Vec<Conversation>,
}
#[derive(Clone, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub document: Option<Document>,
    pub error: Option<String>,
    pub lean_available: bool,
    pub lean: String,
}

pub fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1000
        && value.split('.').all(|part| {
            !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
}
pub fn validate(document: &Document) -> Result<()> {
    anyhow::ensure!(
        document.version == 1
            && document.name.len() <= 500
            && document.tasks.len() <= 64
            && document.events.len() <= 1000
            && document.conversations.len() <= 100,
        "Unsupported or oversized portable workspace"
    );
    let mut ids = BTreeSet::new();
    for task in &document.tasks {
        anyhow::ensure!(
            valid_name(&task.id)
                && ids.insert(task.id.clone())
                && valid_name(&task.module)
                && !task.title.trim().is_empty()
                && task.title.len() <= 1000
                && !task.requirement.trim().is_empty()
                && task.requirement.len() <= 16384
                && task.next.len() <= 16384
                && task.theorems.len() <= 100
                && task.theorems.iter().all(|name| valid_name(name)),
            "Invalid portable work item"
        );
    }
    let mut events = BTreeSet::new();
    for event in &document.events {
        anyhow::ensure!(
            ids.contains(&event.task)
                && valid_name(&event.id)
                && events.insert(&event.id)
                && event.note.len() <= 65536,
            "Invalid portable history linkage"
        );
        if let Some(check) = &event.check {
            anyhow::ensure!(
                valid_name(&check.module)
                    && check.requirement.len() <= 16384
                    && check.theorems.len() <= 100
                    && check.theorems.iter().all(|name| valid_name(name))
                    && check.inspection.len() <= 256_000
                    && digest_valid(&check.engine_sha256)
                    && check.stdout.len() <= 256_000
                    && check.stderr.len() <= 256_000
                    && digest_valid(&check.source_revision)
                    && check.after_revision.as_deref().is_none_or(digest_valid),
                "Invalid portable check record"
            );
        }
    }
    for conversation in &document.conversations {
        anyhow::ensure!(
            valid_name(&conversation.id)
                && conversation.messages.len() <= 4000
                && conversation
                    .messages
                    .iter()
                    .all(|m| m.text.len() <= 262_144),
            "Invalid portable conversation"
        );
    }
    Ok(())
}
pub fn digest_valid(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
}
