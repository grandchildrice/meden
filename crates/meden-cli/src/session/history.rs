//! Conversation data only. These checkpoints cannot restore kernel authority.

use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Speaker {
    User,
    Assistant,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Message {
    pub speaker: Speaker,
    pub text: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct History {
    version: u32,
    workspace: Vec<u8>,
    pub messages: Vec<Value>,
    pub transcript: Vec<Message>,
    // Presentation only, never restored as a conversation role or authority.
    #[serde(default)]
    pub stopped: Option<String>,
}

pub(crate) fn load(root: &Path) -> Result<History> {
    let path = crate::storage::directory(root)?.join("conversation.json");
    let saved = crate::storage::read::<History>(&path)?.unwrap_or_else(|| History {
        version: 1,
        workspace: root.as_os_str().as_encoded_bytes().to_vec(),
        messages: Vec::new(),
        transcript: Vec::new(),
        stopped: None,
    });
    anyhow::ensure!(
        saved.version == 1 && saved.workspace == root.as_os_str().as_encoded_bytes(),
        "saved conversation belongs to another repository or version"
    );
    for message in &saved.messages {
        anyhow::ensure!(
            matches!(
                message.get("role").and_then(Value::as_str),
                Some("user" | "assistant" | "tool")
            ),
            "saved history cannot replace host system instructions"
        );
    }
    Ok(saved)
}

pub(super) fn save_messages(root: &Path, messages: &[Value]) -> Result<()> {
    let mut saved = load(root)?;
    saved.messages = messages.get(1..).unwrap_or_default().to_vec();
    save(root, &saved)
}

pub(crate) fn save_transcript(
    root: &Path,
    transcript: Vec<Message>,
    stopped: Option<String>,
) -> Result<()> {
    let mut saved = load(root)?;
    saved.transcript = transcript;
    saved.stopped = stopped;
    save(root, &saved)
}

pub(super) fn save_stop(root: &Path, stopped: Option<String>) -> Result<()> {
    let mut saved = load(root)?;
    saved.stopped = stopped;
    save(root, &saved)
}

pub(crate) fn clear(root: &Path, clear_view: bool) -> Result<()> {
    let mut saved = load(root)?;
    saved.messages.clear();
    saved.stopped = None;
    if clear_view {
        saved.transcript.clear();
    }
    save(root, &saved)
}

fn save(root: &Path, saved: &History) -> Result<()> {
    crate::storage::write(
        &crate::storage::directory(root)?.join("conversation.json"),
        saved,
    )
}
