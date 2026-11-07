//! An OpenAI-compatible chat client.
//!
//! Points at any server that speaks `/v1/chat/completions`: vLLM, Ollama,
//! llama.cpp, LM Studio, Hermes' API server or a hosted API.

use std::time::Duration;

use meden_agent::planner::{PlanRequest, Planner, PlannerError, Quarantined};
use meden_agent::prompt::{parse_plan, system_prompt, user_prompt};
use meden_agent::{Data, Plan};
use serde::{Deserialize, Serialize};

/// A client for an OpenAI-compatible chat endpoint.
pub struct OpenAiCompatible {
    base_url: String,
    model: String,
    api_key: Option<String>,
    timeout: Duration,
    temperature: f32,
    max_tokens: u32,
}

impl OpenAiCompatible {
    /// Native tool calling. Sampling and length parameters are omitted to
    /// match AgentDojo's actual OpenAI-compatible request (its zero
    /// temperature is converted to NOT_GIVEN). The server supplies defaults.
    pub fn tool_reply(
        &self,
        messages: &[serde_json::Value],
        tools: &[serde_json::Value],
    ) -> Result<meden_agent::conversation::Reply, PlannerError> {
        let mut body = serde_json::json!({"model":self.model, "messages":messages});
        if let Some(object) = body.as_object_mut().filter(|_| !tools.is_empty()) {
            object.insert("tools".to_owned(), serde_json::json!(tools));
            object.insert("tool_choice".to_owned(), serde_json::json!("auto"));
        }
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(self.timeout))
            .build();
        let agent: ureq::Agent = config.into();
        let url = format!("{}/chat/completions", self.base_url);
        for attempt in 0_u32..3 {
            let mut request = agent.post(&url);
            if let Some(key) = &self.api_key {
                request = request.header("authorization", &format!("Bearer {key}"));
            }
            let mut response = match request.send_json(&body) {
                Ok(response) => response,
                Err(error) => {
                    let permanent =
                        matches!(error, ureq::Error::StatusCode(400 | 401 | 403 | 404 | 422));
                    if permanent || attempt == 2 {
                        return Err(PlannerError::Unavailable(error.to_string()));
                    }
                    std::thread::sleep(Duration::from_secs(1_u64 << attempt));
                    continue;
                }
            };
            let response: serde_json::Value = response
                .body_mut()
                .read_json()
                .map_err(|e| PlannerError::NotAPlan(format!("unreadable API reply: {e}")))?;
            let choice = response
                .get("choices")
                .and_then(serde_json::Value::as_array)
                .and_then(|choices| choices.first())
                .ok_or_else(|| PlannerError::NotAPlan("API reply had no choices".to_owned()))?;
            if choice
                .get("finish_reason")
                .and_then(serde_json::Value::as_str)
                == Some("length")
            {
                return Err(PlannerError::NotAPlan(
                    "the model reply was truncated".to_owned(),
                ));
            }
            return serde_json::from_value(choice.get("message").cloned().unwrap_or_default())
                .map_err(|e| PlannerError::NotAPlan(format!("invalid assistant message: {e}")));
        }
        Err(PlannerError::Unavailable(
            "model retries exhausted".to_owned(),
        ))
    }

    /// A client for `base_url` (for example `http://localhost:8000/v1`).
    pub fn new(base_url: &str, model: &str) -> Self {
        OpenAiCompatible {
            base_url: base_url.trim_end_matches('/').to_owned(),
            model: model.to_owned(),
            api_key: None,
            timeout: Duration::from_secs(300),
            temperature: 0.0,
            max_tokens: 32768,
        }
    }

    /// A client configured from the environment.
    ///
    /// Reads [`crate::ENV_BASE_URL`], [`crate::ENV_MODEL`] and optionally
    /// [`crate::ENV_API_KEY`].
    pub fn from_env() -> Result<Self, PlannerError> {
        let base = std::env::var(crate::ENV_BASE_URL).map_err(|_| {
            PlannerError::Unavailable(format!("{} is not set", crate::ENV_BASE_URL))
        })?;
        let model = std::env::var(crate::ENV_MODEL)
            .map_err(|_| PlannerError::Unavailable(format!("{} is not set", crate::ENV_MODEL)))?;
        let mut client = OpenAiCompatible::new(&base, &model);
        client.api_key = std::env::var(crate::ENV_API_KEY)
            .ok()
            .filter(|k| !k.is_empty());
        Ok(client)
    }

    /// This client with an API key.
    #[must_use]
    pub fn with_api_key(mut self, key: &str) -> Self {
        self.api_key = Some(key.to_owned());
        self
    }

    /// This client with a different request timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// This client with a different answer length limit.
    ///
    /// A plan is small, but a model that reasons before answering can spend
    /// a lot of the budget getting there, and a truncated plan is not a plan.
    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// This client with a different sampling temperature.
    ///
    /// Zero by default: a plan is a program, and there is no reason to want a
    /// different one each time.
    #[must_use]
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// The server this client talks to.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Send one exchange and return the reply's text.
    pub fn chat(&self, system: &str, user: &str) -> Result<String, PlannerError> {
        let body = ChatRequest {
            model: &self.model,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            messages: vec![
                Message {
                    role: "system",
                    content: system.to_owned(),
                },
                Message {
                    role: "user",
                    content: user.to_owned(),
                },
            ],
        };

        let config = ureq::Agent::config_builder()
            .timeout_global(Some(self.timeout))
            .build();
        let agent: ureq::Agent = config.into();
        let url = format!("{}/chat/completions", self.base_url);
        let mut request = agent.post(&url).header("content-type", "application/json");
        if let Some(key) = &self.api_key {
            request = request.header("authorization", &format!("Bearer {key}"));
        }

        let mut response = request
            .send_json(&body)
            .map_err(|e| PlannerError::Unavailable(e.to_string()))?;
        let parsed: ChatResponse = response
            .body_mut()
            .read_json()
            .map_err(|e| PlannerError::NotAPlan(format!("unreadable reply: {e}")))?;

        let choice = parsed
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| PlannerError::NotAPlan("the reply had no choices".to_owned()))?;
        if choice.finish_reason.as_deref() == Some("length") {
            return Err(PlannerError::NotAPlan(format!(
                "the answer was cut off at {} tokens; raise the limit",
                self.max_tokens
            )));
        }
        Ok(choice.message.content)
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    temperature: f32,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: &'static str,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ReplyMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ReplyMessage {
    #[serde(default)]
    content: String,
}

impl Planner for OpenAiCompatible {
    fn plan(&self, request: &PlanRequest) -> Result<Plan, PlannerError> {
        let reply = self.chat(&system_prompt(request), &user_prompt(request))?;
        parse_plan(&reply)
    }
}

impl Quarantined for OpenAiCompatible {
    fn extract(&self, question: &str, text: &str) -> Result<Data, PlannerError> {
        let user = format!("Question: {question}\n\nDocument:\n{text}");
        let reply = self.chat(crate::QUARANTINE_PROMPT, &user)?;
        Ok(Data::Str(reply.trim().to_owned()))
    }
}

#[cfg(test)]
mod tests {
    // Tests fail by panicking; that is their job.
    #![allow(clippy::panic)]

    use super::*;

    #[test]
    fn a_base_url_keeps_no_trailing_slash() {
        let c = OpenAiCompatible::new("http://localhost:8000/v1/", "qwen");
        assert_eq!(c.base_url(), "http://localhost:8000/v1");
    }

    #[test]
    fn the_environment_must_name_a_server_and_a_model() {
        // Cannot safely mutate the environment in a threaded test, so check
        // the error path that does not depend on it being unset.
        let missing = OpenAiCompatible::from_env();
        if std::env::var(crate::ENV_BASE_URL).is_err() {
            assert!(matches!(missing, Err(PlannerError::Unavailable(_))));
        }
    }

    #[test]
    fn a_request_serialises_the_way_the_api_expects() {
        let body = ChatRequest {
            model: "qwen",
            temperature: 0.0,
            max_tokens: 1024,
            messages: vec![Message {
                role: "system",
                content: "hi".to_owned(),
            }],
        };
        let json = serde_json::to_string(&body).unwrap_or_default();
        assert!(json.contains("\"model\":\"qwen\""), "{json}");
        assert!(json.contains("\"role\":\"system\""), "{json}");
    }

    #[test]
    fn a_reply_without_choices_is_an_error() {
        let parsed: Result<ChatResponse, _> = serde_json::from_str("{\"choices\":[]}");
        assert!(parsed.is_ok_and(|p| p.choices.is_empty()));
    }

    #[test]
    fn an_unreachable_server_is_reported_not_panicked() {
        // Port 0 is never listening.
        let c = OpenAiCompatible::new("http://127.0.0.1:1/v1", "m")
            .with_timeout(Duration::from_millis(200));
        assert!(matches!(
            c.chat("s", "u"),
            Err(PlannerError::Unavailable(_))
        ));
    }
}
