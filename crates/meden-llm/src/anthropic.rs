//! Anthropic's Messages API, which is not OpenAI-compatible.

use std::time::Duration;

use meden_agent::planner::PlannerError;
use serde::{Deserialize, Serialize};

/// The API version header Anthropic requires.
const API_VERSION: &str = "2023-06-01";

/// A client for Anthropic's Messages API.
pub struct Anthropic {
    api_key: String,
    model: String,
    base_url: String,
    max_tokens: u32,
    timeout: Duration,
}

impl Anthropic {
    /// A client for this key and model.
    pub fn new(api_key: &str, model: &str) -> Self {
        Anthropic {
            api_key: api_key.to_owned(),
            model: model.to_owned(),
            base_url: "https://api.anthropic.com/v1".to_owned(),
            max_tokens: 8192,
            timeout: Duration::from_secs(300),
        }
    }

    /// This client, pointed somewhere else.
    #[must_use]
    pub fn with_base_url(mut self, base_url: &str) -> Self {
        base_url
            .trim_end_matches('/')
            .clone_into(&mut self.base_url);
        self
    }

    /// This client with a different timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Send one exchange and return the reply's text.
    pub fn chat(&self, system: &str, user: &str) -> Result<String, PlannerError> {
        let body = Request {
            model: &self.model,
            max_tokens: self.max_tokens,
            system,
            messages: vec![Message {
                role: "user",
                content: user.to_owned(),
            }],
        };

        let config = ureq::Agent::config_builder()
            .timeout_global(Some(self.timeout))
            .build();
        let agent: ureq::Agent = config.into();
        let mut response = agent
            .post(&format!("{}/messages", self.base_url))
            .header("content-type", "application/json")
            .header("anthropic-version", API_VERSION)
            .header("x-api-key", &self.api_key)
            .send_json(&body)
            .map_err(|e| PlannerError::Unavailable(e.to_string()))?;

        let parsed: Response = response
            .body_mut()
            .read_json()
            .map_err(|e| PlannerError::NotAPlan(format!("unreadable reply: {e}")))?;

        let text: String = parsed
            .content
            .into_iter()
            .filter(|b| b.kind == "text")
            .map(|b| b.text)
            .collect();
        if text.is_empty() {
            return Err(PlannerError::NotAPlan("the reply had no text".to_owned()));
        }
        Ok(text)
    }
}

#[derive(Serialize)]
struct Request<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: &'static str,
    content: String,
}

#[derive(Deserialize)]
struct Response {
    #[serde(default)]
    content: Vec<Block>,
}

#[derive(Deserialize)]
struct Block {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    text: String,
}

#[cfg(test)]
mod tests {
    // Tests fail by panicking; that is their job.
    #![allow(clippy::panic)]

    use super::*;

    #[test]
    fn a_request_has_the_shape_the_api_expects() {
        let body = Request {
            model: "claude-sonnet-5",
            max_tokens: 1024,
            system: "sys",
            messages: vec![Message {
                role: "user",
                content: "hi".to_owned(),
            }],
        };
        let json = serde_json::to_string(&body).unwrap_or_default();
        assert!(json.contains("\"system\":\"sys\""), "{json}");
        assert!(json.contains("\"max_tokens\":1024"), "{json}");
    }

    #[test]
    fn only_text_blocks_are_read() {
        let parsed: Response = serde_json::from_str(
            r#"{"content":[{"type":"thinking","text":"ignored"},{"type":"text","text":"kept"}]}"#,
        )
        .unwrap_or(Response { content: vec![] });
        let text: String = parsed
            .content
            .into_iter()
            .filter(|b| b.kind == "text")
            .map(|b| b.text)
            .collect();
        assert_eq!(text, "kept");
    }

    #[test]
    fn an_unreachable_endpoint_is_reported() {
        let c = Anthropic::new("k", "m")
            .with_base_url("http://127.0.0.1:1/v1")
            .with_timeout(Duration::from_millis(200));
        assert!(matches!(
            c.chat("s", "u"),
            Err(PlannerError::Unavailable(_))
        ));
    }
}
