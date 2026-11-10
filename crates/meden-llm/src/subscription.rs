//! Read host-selected subscription logins without invoking another agent,
//! refreshing shared tokens or probing a remote service during discovery.
//! Credentials stay in the host broker, outside every offline worker.

use std::io::Read;
use std::path::{Path, PathBuf};

use base64::Engine;
use serde_json::Value;

use crate::{Protocol, Provider};

const AUTH_LIMIT: u64 = 1_048_576;

pub(crate) fn providers() -> Vec<Provider> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let codex = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| home.as_ref().map(|h| h.join(".codex")));
    let claude = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| home.as_ref().map(|h| h.join(".claude")));
    let hermes = crate::hermes::hermes_home();
    let env = hermes
        .as_ref()
        .and_then(|h| read_text(&h.join(".env")))
        .map(|s| crate::hermes::parse_env(&s))
        .unwrap_or_default();
    let claude_token = ["CLAUDE_CODE_OAUTH_TOKEN", "ANTHROPIC_TOKEN"]
        .into_iter()
        .find_map(|name| {
            std::env::var(name)
                .ok()
                .or_else(|| env.get(name).cloned())
                .filter(|s| !s.trim().is_empty())
        });
    let config = hermes
        .as_ref()
        .and_then(|h| read_text(&h.join("config.yaml")))
        .and_then(|s| crate::hermes::parse_default_model(&s));
    let mut found = discover(
        codex.as_deref(),
        claude.as_deref(),
        hermes.as_deref(),
        claude_token,
    );
    for provider in &mut found {
        let base_variable = if provider.id == "codex" {
            "MEDEN_CODEX_BASE_URL"
        } else {
            "MEDEN_CLAUDE_BASE_URL"
        };
        if let Ok(base) = std::env::var(base_variable) {
            provider.base_url = base;
        }
        let (variable, hermes_id) = if provider.id == "codex" {
            ("MEDEN_CODEX_MODEL", "openai-codex")
        } else {
            ("MEDEN_CLAUDE_MODEL", "anthropic")
        };
        if let Some(model) = std::env::var(variable)
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                config
                    .as_ref()
                    .and_then(|(id, model)| (id == hermes_id).then(|| model.clone()).flatten())
            })
        {
            provider.model = model;
        }
    }
    found
}

fn discover(
    codex: Option<&Path>,
    claude: Option<&Path>,
    hermes: Option<&Path>,
    claude_token: Option<String>,
) -> Vec<Provider> {
    let mut found = Vec::new();
    let hermes_auth = hermes.and_then(|h| read_json(&h.join("auth.json")));
    let hermes_codex = hermes_auth
        .as_ref()
        .and_then(|v| v.pointer("/providers/openai-codex/tokens"))
        .filter(|tokens| nonempty(tokens, "access_token").is_some());
    let cli_auth = codex.and_then(|h| read_json(&h.join("auth.json")));
    let cli_codex = cli_auth.as_ref().and_then(|v| v.get("tokens"));
    // Match Hermes' own singleton first; never rotate accounts or use an API
    // key as a subscription token. A malformed selected source is not replaced
    // by a different account after a failed request.
    if let Some(tokens) = hermes_codex.or(cli_codex) {
        if let Some(token) = nonempty(tokens, "access_token") {
            let account_id = nonempty(tokens, "account_id")
                .map(str::to_owned)
                .or_else(|| jwt_account(token));
            found.push(Provider {
                id: "codex".into(),
                label: "Codex subscription".into(),
                model: "gpt-6.1-sol".into(),
                base_url: "https://chatgpt.com/backend-api/codex".into(),
                protocol: Protocol::Responses { account_id },
                api_key: Some(token.to_owned()),
                note: if hermes_codex.is_some() {
                    "subscription login from Hermes; renew with hermes auth".into()
                } else {
                    "subscription login from Codex; renew with codex login".into()
                },
            });
        }
    }
    let cli_claude = claude.and_then(|h| read_json(&h.join(".credentials.json")));
    let token = claude_token
        .map(|token| (token, "host OAuth/setup-token environment"))
        .or_else(|| {
            cli_claude
                .as_ref()
                .and_then(|v| v.get("claudeAiOauth"))
                .and_then(|v| nonempty(v, "accessToken"))
                .map(|s| (s.to_owned(), "Claude Code login"))
        });
    if let Some((token, source)) = token.filter(|(token, _)| !token.starts_with("sk-ant-api")) {
        found.push(Provider {
            id: "claude".into(),
            label: "Claude subscription".into(),
            model: "claude-sonnet-4-6".into(),
            base_url: "https://api.anthropic.com/v1".into(),
            protocol: Protocol::Claude,
            api_key: Some(token),
            note: format!("{source}; renew with claude auth login or claude setup-token"),
        });
    }
    found
}

fn nonempty<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field)?.as_str().filter(|s| !s.trim().is_empty())
}

fn read_text(path: &Path) -> Option<String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(AUTH_LIMIT.saturating_add(1))
        .read_to_end(&mut bytes)
        .ok()?;
    if u64::try_from(bytes.len()).ok()? > AUTH_LIMIT {
        return None;
    }
    String::from_utf8(bytes).ok()
}

fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_str(&read_text(path)?).ok()
}

// This is account-routing metadata, not JWT validation or authorization.
// The provider validates the actual frozen bearer token at inference time.
fn jwt_account(token: &str) -> Option<String> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let claims: Value = serde_json::from_slice(&bytes).ok()?;
    claims
        .pointer("/https:~1~1api.openai.com~1auth/chatgpt_account_id")?
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use serde_json::json;

    fn write(home: &Path, name: &str, value: &Value) {
        std::fs::write(home.join(name), serde_json::to_vec(value).expect("JSON"))
            .expect("auth fixture");
    }

    #[test]
    fn cli_subscriptions_are_separate_from_api_keys_and_debug_redacts_tokens() {
        let codex = tempfile::tempdir().expect("Codex home");
        let claude = tempfile::tempdir().expect("Claude home");
        write(
            codex.path(),
            "auth.json",
            &json!({"OPENAI_API_KEY":"paid-api-key"}),
        );
        assert!(discover(Some(codex.path()), None, None, None).is_empty());
        write(
            codex.path(),
            "auth.json",
            &json!({"tokens":{
                "access_token":"codex-secret", "account_id":"account-a"
            }}),
        );
        write(
            claude.path(),
            ".credentials.json",
            &json!({"claudeAiOauth":{
                "accessToken":"claude-secret", "refreshToken":"unused-secret"
            }}),
        );
        let providers = discover(Some(codex.path()), Some(claude.path()), None, None);
        assert_eq!(providers.len(), 2);
        let codex = providers.first().expect("Codex");
        assert_eq!(codex.id, "codex");
        assert_eq!(
            codex.protocol,
            Protocol::Responses {
                account_id: Some("account-a".into())
            }
        );
        assert_eq!(codex.api_key.as_deref(), Some("codex-secret"));
        let claude = providers.last().expect("Claude");
        assert_eq!(claude.protocol, Protocol::Claude);
        assert_eq!(claude.api_key.as_deref(), Some("claude-secret"));
        assert!(!format!("{providers:?}").contains("secret"));
    }

    #[test]
    fn hermes_codex_and_host_claude_token_take_precedence_without_writing_auth() {
        let home = tempfile::tempdir().expect("home");
        let auth = json!({"providers":{"openai-codex":{"tokens":{
            "access_token":"hermes-secret", "account_id":"hermes-account"
        }}}, "tokens":{"access_token":"cli-secret"},
        "claudeAiOauth":{"accessToken":"file-secret"}});
        write(home.path(), "auth.json", &auth);
        write(home.path(), ".credentials.json", &auth);
        let providers = discover(
            Some(home.path()),
            Some(home.path()),
            Some(home.path()),
            Some("setup-secret".into()),
        );
        assert_eq!(
            providers.first().expect("Codex").api_key.as_deref(),
            Some("hermes-secret")
        );
        assert_eq!(
            providers.last().expect("Claude").api_key.as_deref(),
            Some("setup-secret")
        );
        assert_eq!(read_json(&home.path().join("auth.json")), Some(auth));
    }

    #[test]
    fn malformed_empty_or_oversized_auth_never_becomes_a_provider() {
        let home = tempfile::tempdir().expect("home");
        for auth in [
            json!({"tokens":null}),
            json!({"tokens":{"access_token":" "}}),
        ] {
            write(home.path(), "auth.json", &auth);
            assert!(discover(Some(home.path()), None, None, None).is_empty());
        }
        std::fs::write(home.path().join("auth.json"), "{".repeat(1_048_577)).expect("oversized");
        assert!(discover(Some(home.path()), None, None, None).is_empty());
        assert!(discover(None, None, None, Some("sk-ant-api-billed".into())).is_empty());
    }

    #[test]
    fn account_claim_is_routing_metadata_only() {
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&json!({"https://api.openai.com/auth":{
                "chatgpt_account_id":"account-from-token"
            }}))
            .expect("claims"),
        );
        assert_eq!(
            jwt_account(&format!("header.{payload}.signature")),
            Some("account-from-token".into())
        );
        assert_eq!(jwt_account("not-a-jwt"), None);
    }
}
