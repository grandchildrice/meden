//! OpenAI-compatible native tool-calling client.
use meden_agent::conversation::{ChatModel, ModelError};
use std::time::Duration;
/// HTTP client for native assistant messages.
pub struct OpenAiCompatible {
    base_url: String,
    model: String,
    api_key: Option<String>,
    timeout: Duration,
}
impl OpenAiCompatible {
    /// Connect to an OpenAI-compatible base URL.
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').into(),
            model: model.into(),
            api_key: None,
            timeout: Duration::from_secs(300),
        }
    }
    /// Attach the host's credential.
    #[must_use]
    pub fn with_api_key(mut self, key: &str) -> Self {
        self.api_key = Some(key.into());
        self
    }
}
impl ChatModel for OpenAiCompatible {
    /// Native tool calling. Sampling and length parameters are omitted to
    /// match AgentDojo's actual OpenAI-compatible request (its zero
    /// temperature is converted to NOT_GIVEN). The server supplies defaults.
    fn reply(
        &self,
        messages: &[serde_json::Value],
        tools: &[serde_json::Value],
    ) -> Result<meden_agent::conversation::Reply, ModelError> {
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
                        return Err(ModelError::Unavailable(error.to_string()));
                    }
                    std::thread::sleep(Duration::from_secs(1_u64 << attempt));
                    continue;
                }
            };
            let response: serde_json::Value = response
                .body_mut()
                .read_json()
                .map_err(|e| ModelError::InvalidResponse(format!("unreadable API reply: {e}")))?;
            let choice = response
                .get("choices")
                .and_then(serde_json::Value::as_array)
                .and_then(|choices| choices.first())
                .ok_or_else(|| {
                    ModelError::InvalidResponse("API reply had no choices".to_owned())
                })?;
            if choice
                .get("finish_reason")
                .and_then(serde_json::Value::as_str)
                == Some("length")
            {
                return Err(ModelError::InvalidResponse(
                    "the model reply was truncated".to_owned(),
                ));
            }
            return serde_json::from_value(choice.get("message").cloned().unwrap_or_default())
                .map_err(|e| {
                    ModelError::InvalidResponse(format!("invalid assistant message: {e}"))
                });
        }
        Err(ModelError::Unavailable(
            "model retries exhausted".to_owned(),
        ))
    }
}
