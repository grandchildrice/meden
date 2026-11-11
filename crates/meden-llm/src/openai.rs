//! Host-authorized HTTP inference, with native tool-calling protocol adapters.
use crate::provider::{Identity, Protocol};
use meden_agent::conversation::{Delta, ModelError};
use meden_core::processor::HttpCall;
use std::time::Duration;

/// The HTTP boundary consumes a host-authorized snapshot; callers cannot
/// substitute another endpoint/model/credential after a Boolean decision.
pub(crate) fn reply(
    authorization: HttpCall,
    messages: &[serde_json::Value],
    tools: &[serde_json::Value],
    notify: &dyn Fn(Delta<'_>),
) -> Result<meden_agent::conversation::Reply, ModelError> {
    const LIMIT: u64 = 16 * 1024 * 1024;
    let parameters = authorization.into_parameters();
    let identity: Identity = serde_json::from_slice(parameters.provider())
        .map_err(|_| ModelError::Unavailable("invalid frozen processor protocol".into()))?;
    let endpoint = utf8(parameters.endpoint())?;
    let model = utf8(parameters.model())?;
    let credential = parameters.credential().map(utf8).transpose()?;
    let body = match &identity.protocol {
        Protocol::ChatCompletions => request_body(model, messages, tools),
        Protocol::Responses { .. } => crate::responses::request_body(model, messages, tools)?,
        Protocol::Claude => crate::claude::request_body(model, messages, tools)?,
    };
    let config = ureq::Agent::config_builder()
        .proxy(None)
        .max_redirects(0)
        .timeout_global(Some(Duration::from_secs(300)))
        .build();
    let agent: ureq::Agent = config.into();
    let mut request = agent.post(endpoint);
    if let Some(key) = credential {
        request = request.header("authorization", &format!("Bearer {key}"));
    }
    match &identity.protocol {
        Protocol::Responses { account_id } => {
            request = request.header("accept", "text/event-stream");
            if let Some(account) = account_id {
                request = request.header("chatgpt-account-id", account);
            }
        }
        Protocol::Claude => {
            request = request
                .header("anthropic-version", "2023-06-01")
                .header("anthropic-beta", "oauth-2025-04-20");
        }
        Protocol::ChatCompletions => {}
    }
    // A failed exchange may already have delivered the confidential body.
    // Neither a redirect nor an automatic retry grants another send.
    let mut response = request.send_json(&body).map_err(|error| {
        if let ureq::Error::StatusCode(code) = error {
            return status_error(code);
        }
        ModelError::Unavailable("model request failed or delivery is uncertain".into())
    })?;
    if !response.status().is_success() {
        return Err(status_error(response.status().as_u16()));
    }
    crate::stream::read_response(&mut response, &identity.protocol, notify, LIMIT)
}

fn status_error(code: u16) -> ModelError {
    let reason = match code {
        400 => "request or model rejected; check the selected model and provider support",
        401 => {
            "login expired or invalid; renew the host login and restart Meden for a fresh review"
        }
        403 => "account or model access refused by the provider",
        429 => "provider usage or rate limit reached; wait for the account limit to reset",
        _ => "request did not complete successfully",
    };
    ModelError::Unavailable(format!(
        "model endpoint returned HTTP {code}: {reason}; no retry, account rotation or API-key fallback"
    ))
}

pub(crate) fn invalid(message: &str) -> ModelError {
    ModelError::InvalidResponse(message.to_owned())
}

/// Preserve ordinary text and text blocks; reject unsupported content rather
/// than silently dropping part of the original conversation.
pub(crate) fn text_content(value: Option<&serde_json::Value>) -> Result<String, ModelError> {
    match value {
        None | Some(serde_json::Value::Null) => Ok(String::new()),
        Some(serde_json::Value::String(text)) => Ok(text.clone()),
        Some(serde_json::Value::Array(blocks)) => blocks
            .iter()
            .map(|block| {
                if block.get("type").and_then(serde_json::Value::as_str) != Some("text") {
                    return Err(invalid("unsupported conversation content block"));
                }
                block
                    .get("text")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
                    .ok_or_else(|| invalid("invalid conversation text"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|texts| texts.join("\n")),
        _ => Err(invalid("unsupported conversation content")),
    }
}

fn utf8(bytes: &[u8]) -> Result<&str, ModelError> {
    std::str::from_utf8(bytes)
        .map_err(|_| ModelError::Unavailable("invalid frozen processor parameters".into()))
}

pub(crate) fn request_body(
    model: &str,
    messages: &[serde_json::Value],
    tools: &[serde_json::Value],
) -> serde_json::Value {
    let mut body = serde_json::json!({"model":model, "messages":messages, "stream":true});
    if let Some(object) = body.as_object_mut().filter(|_| !tools.is_empty()) {
        object.insert("tools".to_owned(), serde_json::json!(tools));
        object.insert("tool_choice".to_owned(), serde_json::json!("auto"));
    }
    body
}

pub(crate) fn decode_response(
    response: &serde_json::Value,
) -> Result<meden_agent::conversation::Reply, ModelError> {
    let choice = response
        .get("choices")
        .and_then(serde_json::Value::as_array)
        .and_then(|choices| choices.first())
        .ok_or_else(|| ModelError::InvalidResponse("API reply had no choices".to_owned()))?;
    if choice
        .get("finish_reason")
        .and_then(serde_json::Value::as_str)
        == Some("length")
    {
        return Err(ModelError::InvalidResponse(
            "the model reply was truncated".to_owned(),
        ));
    }
    serde_json::from_value(choice.get("message").cloned().unwrap_or_default())
        .map_err(|e| ModelError::InvalidResponse(format!("invalid assistant message: {e}")))
}
