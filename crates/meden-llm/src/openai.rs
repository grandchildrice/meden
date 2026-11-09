//! OpenAI-compatible native tool-calling client.
use meden_agent::conversation::ModelError;
use meden_core::processor::HttpCall;
use std::time::Duration;

/// The HTTP boundary consumes a host-authorized snapshot; callers cannot
/// substitute another endpoint/model/credential after a Boolean decision.
pub(crate) fn reply(
    authorization: HttpCall,
    messages: &[serde_json::Value],
    tools: &[serde_json::Value],
) -> Result<meden_agent::conversation::Reply, ModelError> {
    let parameters = authorization.into_parameters();
    let endpoint = utf8(parameters.endpoint())?;
    let model = utf8(parameters.model())?;
    let credential = parameters.credential().map(utf8).transpose()?;
    let body = request_body(model, messages, tools);
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
    // A failed exchange may already have delivered the confidential body.
    // Neither a redirect nor an automatic retry grants another send.
    let mut response = request.send_json(&body).map_err(|_| {
        ModelError::Unavailable("model request failed or delivery is uncertain".into())
    })?;
    if !response.status().is_success() {
        return Err(ModelError::Unavailable(format!(
            "model endpoint returned HTTP {}",
            response.status().as_u16()
        )));
    }
    let response: serde_json::Value = response
        .body_mut()
        .read_json()
        .map_err(|e| ModelError::InvalidResponse(format!("unreadable API reply: {e}")))?;
    decode_response(&response)
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
    let mut body = serde_json::json!({"model":model, "messages":messages});
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
