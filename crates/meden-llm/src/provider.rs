//! Discover connection metadata. Discovery is not inference authorization.
use std::time::Duration;

/// Wire format and authentication metadata frozen with the processor grant.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "api", rename_all = "snake_case", deny_unknown_fields)]
pub enum Protocol {
    /// OpenAI-compatible Chat Completions (including Ollama).
    ChatCompletions,
    /// Subscription Responses streaming, with an optional ChatGPT account.
    Responses {
        /// Account selected by the host's Codex login, never by the model.
        account_id: Option<String>,
    },
    /// Anthropic Messages with a host-owned subscription OAuth token.
    Claude,
}

impl Protocol {
    pub(crate) fn path(&self) -> &'static str {
        match self {
            Self::ChatCompletions => "/chat/completions",
            Self::Responses { .. } => "/responses",
            Self::Claude => "/messages",
        }
    }
}

/// Encoded into the core's opaque provider bytes so transport interpretation
/// cannot use a later presentation copy to replace the protocol or account.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Identity {
    pub id: String,
    pub protocol: Protocol,
}

/// A discovered HTTP model connection. Discovery does not authorize inference.
#[derive(Clone, PartialEq, Eq)]
pub struct Provider {
    /// Stable connection identifier.
    pub id: String,
    /// Human-readable connection name.
    pub label: String,
    /// Model identifier.
    pub model: String,
    /// HTTP API base URL.
    pub base_url: String,
    /// Exact API format and authentication metadata.
    pub protocol: Protocol,
    /// Host-owned credential, excluded from debug formatting.
    pub api_key: Option<String>,
    /// Discovery source shown in the picker.
    pub note: String,
}
impl std::fmt::Debug for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Provider")
            .field("id", &self.id)
            .field("model", &self.model)
            .finish_non_exhaustive()
    }
}
/// An endpoint worth probing, and what it would be called.
struct Candidate {
    id: &'static str,
    label: &'static str,
    url: &'static str,
    default_model: &'static str,
}

const HTTP_CANDIDATES: [Candidate; 4] = [
    Candidate {
        id: "ollama",
        label: "Ollama",
        url: "http://localhost:11434/v1",
        default_model: "qwen3:8b",
    },
    Candidate {
        id: "vllm",
        label: "vLLM",
        url: "http://localhost:8000/v1",
        default_model: "local",
    },
    Candidate {
        id: "lmstudio",
        label: "LM Studio",
        url: "http://localhost:1234/v1",
        default_model: "local",
    },
    Candidate {
        id: "hermes-api",
        label: "Hermes gateway",
        url: "http://localhost:8642/v1",
        default_model: "hermes",
    },
];

/// Available host model connections, in discovery order.
///
/// Probing is quick and never fatal: anything that does not answer is simply
/// not offered.
pub fn detect() -> Vec<Provider> {
    let mut found = Vec::new();

    // Anything the operator configured explicitly wins.
    if let (Ok(base), Ok(model)) = (
        std::env::var(crate::ENV_BASE_URL),
        std::env::var(crate::ENV_MODEL),
    ) {
        found.push(Provider {
            id: "configured".to_owned(),
            label: "Configured server".to_owned(),
            model,
            base_url: base.trim_end_matches('/').to_owned(),
            protocol: Protocol::ChatCompletions,
            api_key: std::env::var(crate::ENV_API_KEY)
                .ok()
                .filter(|k| !k.is_empty()),
            note: format!("from {}", crate::ENV_BASE_URL),
        });
    }

    for c in &HTTP_CANDIDATES {
        if let Some(models) = list_models(c.url, None) {
            let model = models
                .first()
                .cloned()
                .unwrap_or_else(|| c.default_model.to_owned());
            found.push(Provider {
                id: c.id.to_owned(),
                label: c.label.to_owned(),
                model,
                base_url: c.url.to_owned(),
                protocol: Protocol::ChatCompletions,
                api_key: None,
                note: format!("listening on {}", c.url),
            });
        }
    }

    // Anything already configured in Hermes, including Ollama Cloud. Doing
    // this before the bare environment variables means a machine that
    // already has Hermes set up needs no setup here at all.
    found.extend(crate::hermes::providers());
    found.extend(crate::subscription::providers());

    // Ollama Cloud named directly in this process's environment.
    if let Ok(key) = std::env::var("OLLAMA_API_KEY") {
        if !key.is_empty() {
            let host = std::env::var("OLLAMA_HOST")
                .ok()
                .filter(|h| !h.is_empty())
                .unwrap_or_else(|| "https://ollama.com".to_owned());
            let base = format!("{}/v1", host.trim_end_matches('/'));
            let model = list_models(&base, Some(&key))
                .and_then(|m| m.first().cloned())
                .unwrap_or_else(|| "gpt-oss:20b".to_owned());
            if found.iter().any(|p| p.id == "hermes:ollama-cloud") {
                // Already offered through Hermes' configuration.
            } else {
                found.push(Provider {
                    id: "ollama-cloud".to_owned(),
                    label: "Ollama Cloud".to_owned(),
                    model,
                    base_url: base,
                    protocol: Protocol::ChatCompletions,
                    api_key: Some(key),
                    note: "from OLLAMA_API_KEY".to_owned(),
                });
            }
        }
    }

    for (id, label, env, base, model) in [(
        "openai",
        "OpenAI",
        "OPENAI_API_KEY",
        "https://api.openai.com/v1",
        "gpt-5",
    )] {
        if let Ok(key) = std::env::var(env) {
            if !key.is_empty() {
                found.push(Provider {
                    id: id.to_owned(),
                    label: label.to_owned(),
                    model: model.to_owned(),
                    base_url: base.to_owned(),
                    protocol: Protocol::ChatCompletions,
                    api_key: Some(key),
                    note: format!("from {env}"),
                });
            }
        }
    }

    found
}

/// Ask an OpenAI-compatible server what models it has.
///
/// `None` when it does not answer, which is how a closed port is reported.
fn list_models(base_url: &str, api_key: Option<&str>) -> Option<Vec<String>> {
    #[derive(serde::Deserialize)]
    struct Models {
        data: Vec<Model>,
    }
    #[derive(serde::Deserialize)]
    struct Model {
        id: String,
    }

    let config = ureq::Agent::config_builder()
        .proxy(None)
        .max_redirects(0)
        .timeout_global(Some(Duration::from_millis(700)))
        .build();
    let agent: ureq::Agent = config.into();
    let mut request = agent.get(&format!("{}/models", base_url.trim_end_matches('/')));
    if let Some(key) = api_key {
        request = request.header("authorization", &format!("Bearer {key}"));
    }
    let mut response = request.call().ok()?;
    if !response.status().is_success() {
        return None;
    }
    let parsed: Models = response.body_mut().read_json().ok()?;
    Some(parsed.data.into_iter().map(|m| m.id).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_closed_port_simply_reports_nothing() {
        assert!(list_models("http://127.0.0.1:1/v1", None).is_none());
    }
    #[test]
    fn detection_never_repeats_an_id() {
        let providers = detect();
        let unique: std::collections::BTreeSet<_> = providers.iter().map(|p| &p.id).collect();
        assert_eq!(providers.len(), unique.len());
    }
}
