//! Borrowing the model configuration someone already has.
//!
//! [Hermes](https://hermes-agent.nousresearch.com) keeps its settings in
//! `~/.hermes/config.yaml` and its keys in `~/.hermes/.env`, and the servers
//! it talks to are OpenAI-compatible. Reading that configuration means
//! Ollama Cloud and anything else already set up there works here with no
//! second setup and no key copied into another file.
//!
//! Only what is needed is read: the name, base URL, model and the *name* of
//! the environment variable holding each key. A key's value is looked up and
//! passed to the request; it is never logged, and `Debug` on a
//! [`Provider`](crate::Provider) carries it only as far as the caller does.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::provider::{Provider, Transport};

/// Where Hermes keeps its configuration.
///
/// `$HERMES_HOME` when set, otherwise `~/.hermes`. `None` when neither
/// exists.
pub fn hermes_home() -> Option<PathBuf> {
    let candidate = match std::env::var_os("HERMES_HOME") {
        Some(h) => PathBuf::from(h),
        None => home_dir()?.join(".hermes"),
    };
    candidate.is_dir().then_some(candidate)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// One `custom_providers` entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomProvider {
    /// Its name, as Hermes calls it.
    pub name: String,
    /// The OpenAI-compatible base URL.
    pub base_url: String,
    /// The environment variable the key lives in.
    pub key_env: Option<String>,
    /// The model to use.
    pub model: Option<String>,
}

/// Read `custom_providers` out of a Hermes `config.yaml`.
pub fn parse_custom_providers(yaml: &str) -> Vec<CustomProvider> {
    let Ok(docs) = yaml_rust2::YamlLoader::load_from_str(yaml) else {
        return Vec::new();
    };
    let Some(doc) = docs.first() else {
        return Vec::new();
    };
    let entries = &doc["custom_providers"];
    let Some(list) = entries.as_vec() else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|e| {
            let name = e["name"].as_str()?.to_owned();
            let base_url = e["base_url"].as_str().unwrap_or_default().trim().to_owned();
            if base_url.is_empty() {
                return None;
            }
            Some(CustomProvider {
                name,
                base_url: base_url.trim_end_matches('/').to_owned(),
                key_env: e["key_env"].as_str().map(str::to_owned),
                model: e["model"].as_str().map(str::to_owned),
            })
        })
        .collect()
}

/// The provider Hermes is set to use by default, as `(provider, model)`.
pub fn parse_default_model(yaml: &str) -> Option<(String, Option<String>)> {
    let docs = yaml_rust2::YamlLoader::load_from_str(yaml).ok()?;
    let doc = docs.first()?;
    let provider = doc["model"]["provider"].as_str()?.to_owned();
    let model = doc["model"]["model"].as_str().map(str::to_owned);
    Some((provider, model))
}

/// Read a `.env` file into a map, ignoring comments and blank lines.
pub fn parse_env(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
            .unwrap_or(value);
        out.insert(key.trim().to_owned(), value.to_owned());
    }
    out
}

/// Every provider Hermes is configured with that this runtime can use.
///
/// The one Hermes itself defaults to comes first.
pub fn providers() -> Vec<Provider> {
    let Some(home) = hermes_home() else {
        return Vec::new();
    };
    providers_in(&home)
}

/// [`providers`], against a given Hermes home. Separated so it can be tested.
pub fn providers_in(home: &Path) -> Vec<Provider> {
    let Ok(config) = std::fs::read_to_string(home.join("config.yaml")) else {
        return Vec::new();
    };
    let env = std::fs::read_to_string(home.join(".env"))
        .map(|t| parse_env(&t))
        .unwrap_or_default();
    let default = parse_default_model(&config);

    let mut found: Vec<Provider> = parse_custom_providers(&config)
        .into_iter()
        .filter_map(|c| {
            // A key named in the config, found either in Hermes' .env or in
            // this process's own environment.
            let api_key = c.key_env.as_ref().and_then(|name| {
                env.get(name)
                    .cloned()
                    .or_else(|| std::env::var(name).ok())
                    .filter(|k| !k.is_empty())
            });
            // A provider whose config names a key variable but has no value
            // for it cannot be used.
            if c.key_env.is_some() && api_key.is_none() {
                return None;
            }
            let model = c
                .model
                .clone()
                .or_else(|| {
                    default
                        .as_ref()
                        .filter(|(p, _)| p == &c.name || p == &format!("custom:{}", c.name))
                        .and_then(|(_, m)| m.clone())
                })
                .unwrap_or_else(|| "default".to_owned());
            Some(Provider {
                id: format!("hermes:{}", c.name),
                label: format!("{} (via Hermes)", c.name),
                model,
                transport: Transport::OpenAi {
                    base_url: c.base_url.clone(),
                    api_key,
                },
                note: format!("from {}", home.join("config.yaml").display()),
            })
        })
        .collect();

    // Put the one Hermes defaults to first.
    if let Some((preferred, _)) = &default {
        let wanted = preferred.trim_start_matches("custom:");
        found.sort_by_key(|p| p.id != format!("hermes:{wanted}"));
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = r"
model:
  provider: custom:ollama-cloud
  model: minimax-m3
  base_url: ''
custom_providers:
  - name: ollama-cloud
    base_url: https://ollama.com/v1
    key_env: OLLAMA_API_KEY
    model: minimax-m3
  - name: somewhere-else
    base_url: https://example.invalid/v1/
    key_env: MISSING_KEY_VAR
  - name: no-url
    base_url: ''
";

    #[test]
    fn custom_providers_are_read() {
        let found = parse_custom_providers(CONFIG);
        assert_eq!(found.len(), 2, "{found:?}");
        let first = found.first().map(|c| c.base_url.clone());
        assert_eq!(first.as_deref(), Some("https://ollama.com/v1"));
        // A trailing slash is trimmed so URLs join predictably.
        let second = found.get(1).map(|c| c.base_url.clone());
        assert_eq!(second.as_deref(), Some("https://example.invalid/v1"));
    }

    #[test]
    fn the_default_provider_is_read() {
        assert_eq!(
            parse_default_model(CONFIG),
            Some((
                "custom:ollama-cloud".to_owned(),
                Some("minimax-m3".to_owned())
            ))
        );
        assert_eq!(parse_default_model("not: yaml: at: all"), None);
    }

    #[test]
    fn env_files_are_read_with_quotes_and_comments() {
        let env = parse_env(
            "# a comment\nOLLAMA_API_KEY=abc123\nexport QUOTED=\"with spaces\"\nSINGLE='x'\nbroken\n\n",
        );
        assert_eq!(
            env.get("OLLAMA_API_KEY").map(String::as_str),
            Some("abc123")
        );
        assert_eq!(env.get("QUOTED").map(String::as_str), Some("with spaces"));
        assert_eq!(env.get("SINGLE").map(String::as_str), Some("x"));
        assert!(!env.contains_key("broken"));
    }

    #[test]
    fn a_provider_whose_key_is_missing_is_not_offered() {
        let dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(_) => return,
        };
        let _ = std::fs::write(dir.path().join("config.yaml"), CONFIG);
        let _ = std::fs::write(dir.path().join(".env"), "OLLAMA_API_KEY=secret\n");

        let found = providers_in(dir.path());
        let ids: Vec<&str> = found.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["hermes:ollama-cloud"], "{found:?}");

        let p = found.first().map(|p| p.transport.clone());
        match p {
            Some(Transport::OpenAi { base_url, api_key }) => {
                assert_eq!(base_url, "https://ollama.com/v1");
                assert_eq!(api_key.as_deref(), Some("secret"));
            }
            other => panic!("expected an OpenAI transport, got {other:?}"),
        }
        assert_eq!(
            found.first().map(|p| p.model.clone()).as_deref(),
            Some("minimax-m3")
        );
    }

    #[test]
    fn a_home_without_a_config_offers_nothing() {
        let dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(_) => return,
        };
        assert!(providers_in(dir.path()).is_empty());
    }
}
