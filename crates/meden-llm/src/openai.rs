//! Host-authorized HTTP inference, with native tool-calling protocol adapters.
use meden_agent::conversation::{Delta, ModelError};
use meden_core::processor::HttpCall;
use std::{io::Read, time::Duration};

/// Only the consuming core call supplies endpoint, model and credential. The
/// retained native adapter may encode/decode, never replace those parameters.
pub(crate) fn reply(
    authorization: HttpCall,
    selected: &crate::transport::Selected,
    messages: &[serde_json::Value],
    tools: &[serde_json::Value],
    notify: &dyn Fn(Delta<'_>),
) -> Result<meden_agent::conversation::Reply, ModelError> {
    let parameters = authorization.into_parameters();
    if parameters.provider() != selected.identity.as_slice() {
        return Err(crate::transport::unavailable(
            "Registered model binding differs from the original processor call",
        ));
    }
    let endpoint = utf8(parameters.endpoint())?;
    let model = utf8(parameters.model())?;
    let credential = parameters.credential().map(utf8).transpose()?;
    let body = selected.encode(model, messages, tools)?;
    let config = ureq::Agent::config_builder()
        .proxy(None)
        .max_redirects(0)
        .timeout_global(Some(Duration::from_secs(crate::transport::TIMEOUT_SECONDS)))
        .build();
    let agent: ureq::Agent = config.into();
    let mut request = agent.post(endpoint);
    if let Some(key) = credential {
        request = request.header("authorization", &format!("Bearer {key}"));
    }
    for (name, value) in &selected.headers {
        request = request.header(name, value);
    }
    selected.validate()?;
    let mut response = request.send_json(&body).map_err(|error| {
        if let ureq::Error::StatusCode(code) = error {
            return status_error(code);
        }
        ModelError::Unavailable("model request failed or delivery is uncertain".into())
    })?;
    if !response.status().is_success() {
        return Err(status_error(response.status().as_u16()));
    }
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let mut counted = Counted {
        inner: response
            .body_mut()
            .as_reader()
            .take(crate::transport::RESPONSE_BYTES.saturating_add(1)),
        bytes: 0,
    };
    let reply = selected.decode(&mut counted, content_type.as_deref(), notify)?;
    // A decoder cannot admit a reply while ignoring a trailing/incomplete body.
    let mut probe = [0; 1];
    let remaining = counted
        .read(&mut probe)
        .map_err(|_| invalid("incomplete API reply"))?;
    if counted.bytes > crate::transport::RESPONSE_BYTES {
        return Err(invalid("API reply exceeded the size limit"));
    }
    if remaining != 0 {
        return Err(invalid("model adapter did not consume the complete reply"));
    }
    selected.validate()?;
    Ok(reply)
}

struct Counted<R> {
    inner: R,
    bytes: u64,
}
impl<R: Read> Read for Counted<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let count = self.inner.read(buffer)?;
        self.bytes = self
            .bytes
            .saturating_add(u64::try_from(count).map_err(std::io::Error::other)?);
        Ok(count)
    }
}

fn status_error(code: u16) -> ModelError {
    let reason = match code {
        400 => "request or model rejected; check the selected model and provider support",
        401 => {
            "authentication expired or invalid; update the selected API key or host login for a fresh review"
        }
        403 => "account or model access refused by the provider",
        429 => "provider usage or rate limit reached; wait for the account limit to reset",
        _ => "request did not complete successfully",
    };
    ModelError::Http {
        status: code,
        reason: format!("{reason}; no automatic retry, account rotation or API-key fallback"),
    }
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
