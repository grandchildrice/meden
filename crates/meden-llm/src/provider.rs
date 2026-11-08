//! Finding a model to plan with, and talking to whichever one is chosen.
//!
//! [`detect`] looks at the machine and reports what could answer: agents
//! already installed (`claude`, `codex`, `hermes`, `ollama`), servers already
//! listening (Ollama, vLLM, llama.cpp, LM Studio, Hermes' gateway), and keys
//! already in the environment (Ollama Cloud, Anthropic, OpenAI).
//!
//! For the historical isolated planner, model selection does not change the
//! label/policy checks its interpreter performs.
//! A plan from a 4B model served on a laptop and a plan from a hosted
//! frontier model meet the same three checks. Which one is in use is a
//! question of how good the plans are. The native conversation engine does
//! not enforce those flow rules. Provider discovery is not authorization to
//! send confidential context; the target design adds that separate boundary.

use std::time::Duration;

use meden_agent::planner::{PlanRequest, Planner, PlannerError, Quarantined};
use meden_agent::prompt::{parse_plan, system_prompt, user_prompt};
use meden_agent::{Data, Plan};

use crate::anthropic::Anthropic;
use crate::command::{CommandSpec, OUTPUT_PLACEHOLDER, PROMPT_PLACEHOLDER};
use crate::openai::OpenAiCompatible;

/// How to reach a model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Transport {
    /// An OpenAI-compatible `/v1/chat/completions` endpoint.
    OpenAi {
        /// Base URL, ending in `/v1`.
        base_url: String,
        /// Key, when the server wants one.
        api_key: Option<String>,
    },
    /// Anthropic's Messages API.
    Anthropic {
        /// The API key.
        api_key: String,
    },
    /// A command-line tool that answers one prompt.
    Command(CommandSpec),
}

/// A model this machine could use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provider {
    /// Stable identifier, for `--provider`.
    pub id: String,
    /// What to show a person.
    pub label: String,
    /// The model name, where the transport needs one.
    pub model: String,
    /// How to reach it.
    pub transport: Transport,
    /// Why this was offered, for the picker.
    pub note: String,
}

impl Provider {
    /// Ask this model one question.
    pub fn chat(&self, system: &str, user: &str) -> Result<String, PlannerError> {
        match &self.transport {
            Transport::OpenAi { base_url, api_key } => {
                let mut client = OpenAiCompatible::new(base_url, &self.model);
                if let Some(key) = api_key {
                    client = client.with_api_key(key);
                }
                client.chat(system, user)
            }
            Transport::Anthropic { api_key } => {
                Anthropic::new(api_key, &self.model).chat(system, user)
            }
            Transport::Command(spec) => {
                // Command-line tools take one prompt, so the two turns are
                // joined. The separator is spelled out because a small model
                // otherwise loses track of where the instructions end.
                let prompt = format!("{system}\n\n---\n\nTask:\n{user}");
                spec.run(&prompt)
            }
        }
    }
}

impl Planner for Provider {
    fn plan(&self, request: &PlanRequest) -> Result<Plan, PlannerError> {
        let reply = self.chat(&system_prompt(request), &user_prompt(request))?;
        parse_plan(&reply)
    }
}

impl Quarantined for Provider {
    fn extract(&self, question: &str, text: &str) -> Result<Data, PlannerError> {
        let user = format!("Question: {question}\n\nDocument:\n{text}");
        let reply = self.chat(crate::QUARANTINE_PROMPT, &user)?;
        Ok(Data::Str(reply.trim().to_owned()))
    }
}

impl meden_agent::conversation::ChatModel for Provider {
    fn reply(
        &self,
        messages: &[serde_json::Value],
        tools: &[serde_json::Value],
    ) -> Result<meden_agent::conversation::Reply, PlannerError> {
        let Transport::OpenAi { base_url, api_key } = &self.transport else {
            return Err(PlannerError::Unavailable(
                "native tool calling requires an OpenAI-compatible HTTP provider".to_owned(),
            ));
        };
        let mut client = OpenAiCompatible::new(base_url, &self.model);
        if let Some(key) = api_key {
            client = client.with_api_key(key);
        }
        client.tool_reply(messages, tools)
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

/// Everything this machine could plan with, best-first.
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
            transport: Transport::OpenAi {
                base_url: base.trim_end_matches('/').to_owned(),
                api_key: std::env::var(crate::ENV_API_KEY)
                    .ok()
                    .filter(|k| !k.is_empty()),
            },
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
                transport: Transport::OpenAi {
                    base_url: c.url.to_owned(),
                    api_key: None,
                },
                note: format!("listening on {}", c.url),
            });
        }
    }

    // Anything already configured in Hermes, including Ollama Cloud. Doing
    // this before the bare environment variables means a machine that
    // already has Hermes set up needs no setup here at all.
    found.extend(crate::hermes::providers());

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
                    transport: Transport::OpenAi {
                        base_url: base,
                        api_key: Some(key),
                    },
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
                    transport: Transport::OpenAi {
                        base_url: base.to_owned(),
                        api_key: Some(key),
                    },
                    note: format!("from {env}"),
                });
            }
        }
    }

    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        if !key.is_empty() {
            found.push(Provider {
                id: "anthropic".to_owned(),
                label: "Anthropic".to_owned(),
                model: "claude-sonnet-5".to_owned(),
                transport: Transport::Anthropic { api_key: key },
                note: "from ANTHROPIC_API_KEY".to_owned(),
            });
        }
    }

    found.extend(detect_commands());
    found
}

/// The command-line agents that are installed.
pub fn detect_commands() -> Vec<Provider> {
    let mut found = Vec::new();
    for spec in command_providers() {
        if let Transport::Command(c) = &spec.transport {
            if which(&c.program).is_some() {
                found.push(spec);
            }
        }
    }
    found
}

/// Every command-line agent this knows how to drive, installed or not.
pub fn command_providers() -> Vec<Provider> {
    let timeout = Duration::from_secs(300);
    vec![
        Provider {
            id: "claude".to_owned(),
            label: "Claude Code".to_owned(),
            model: "default".to_owned(),
            transport: Transport::Command(
                CommandSpec::stdout("claude", &["-p", PROMPT_PLACEHOLDER]).with_timeout(timeout),
            ),
            note: "the claude command".to_owned(),
        },
        Provider {
            id: "codex".to_owned(),
            label: "Codex".to_owned(),
            model: "default".to_owned(),
            transport: Transport::Command(
                CommandSpec::output_file(
                    "codex",
                    &[
                        "exec",
                        "--skip-git-repo-check",
                        "-o",
                        OUTPUT_PLACEHOLDER,
                        PROMPT_PLACEHOLDER,
                    ],
                )
                .with_timeout(timeout),
            ),
            note: "the codex command".to_owned(),
        },
        Provider {
            id: "hermes".to_owned(),
            label: "Hermes".to_owned(),
            model: "default".to_owned(),
            transport: Transport::Command(
                CommandSpec::stdout("hermes", &["chat", "-q", PROMPT_PLACEHOLDER])
                    .with_timeout(timeout),
            ),
            note: "the hermes command".to_owned(),
        },
        Provider {
            id: "ollama-cli".to_owned(),
            label: "Ollama (command)".to_owned(),
            model: "qwen3:8b".to_owned(),
            transport: Transport::Command(
                CommandSpec::stdout("ollama", &["run", "qwen3:8b", PROMPT_PLACEHOLDER])
                    .with_timeout(timeout),
            ),
            note: "the ollama command".to_owned(),
        },
    ]
}

/// Is this program on the path?
pub fn which(program: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(program))
        .find(|p| p.is_file())
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
        .timeout_global(Some(Duration::from_millis(700)))
        .build();
    let agent: ureq::Agent = config.into();
    let mut request = agent.get(&format!("{}/models", base_url.trim_end_matches('/')));
    if let Some(key) = api_key {
        request = request.header("authorization", &format!("Bearer {key}"));
    }
    let mut response = request.call().ok()?;
    let parsed: Models = response.body_mut().read_json().ok()?;
    Some(parsed.data.into_iter().map(|m| m.id).collect())
}

#[cfg(test)]
mod tests {
    // Tests fail by panicking; that is their job.
    #![allow(clippy::panic)]

    use super::*;

    #[test]
    fn every_known_command_provider_is_well_formed() {
        for p in command_providers() {
            assert!(!p.id.is_empty());
            assert!(!p.label.is_empty());
            match &p.transport {
                Transport::Command(c) => {
                    assert!(!c.program.is_empty());
                    assert!(
                        c.args.iter().any(|a| a.contains(PROMPT_PLACEHOLDER)),
                        "{} never passes the prompt",
                        p.id
                    );
                    if c.answer_from_file {
                        assert!(
                            c.args.iter().any(|a| a.contains(OUTPUT_PLACEHOLDER)),
                            "{} reads a file it never names",
                            p.id
                        );
                    }
                }
                other => panic!("{} should be a command, got {other:?}", p.id),
            }
        }
    }

    #[test]
    fn detection_never_fails_and_never_repeats_an_id() {
        let found = detect();
        let mut ids: Vec<&str> = found.iter().map(|p| p.id.as_str()).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before, "duplicate provider id");
    }

    #[test]
    fn which_finds_a_program_that_exists() {
        assert!(which("sh").is_some());
        assert!(which("definitely-not-installed-xyz").is_none());
    }

    #[test]
    fn a_closed_port_simply_reports_nothing() {
        assert_eq!(list_models("http://127.0.0.1:1/v1", None), None);
    }

    #[test]
    fn a_command_provider_joins_the_two_turns() {
        let p = Provider {
            id: "echo".to_owned(),
            label: "echo".to_owned(),
            model: "none".to_owned(),
            transport: Transport::Command(CommandSpec::stdout("cat", &[])),
            note: String::new(),
        };
        // `cat` with no arguments and null stdin says nothing, which is an
        // error rather than an empty plan.
        assert!(p.chat("sys", "usr").is_err());

        let p = Provider {
            transport: Transport::Command(CommandSpec::stdout("echo", &[PROMPT_PLACEHOLDER])),
            ..p
        };
        let said = p.chat("SYSTEM", "USER").unwrap_or_default();
        assert!(said.contains("SYSTEM"), "{said}");
        assert!(said.contains("USER"), "{said}");
    }
}
