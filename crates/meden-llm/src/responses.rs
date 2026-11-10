//! Stateless Responses conversion. Stream deltas never become executable
//! calls: only a complete, successful response admits the original call list.

use meden_agent::conversation::{FunctionCall, ModelError, Reply, ToolCall};
use serde_json::{json, Value};

use crate::openai::{invalid, text_content};

pub(crate) fn request_body(
    model: &str,
    messages: &[Value],
    tools: &[Value],
) -> Result<Value, ModelError> {
    let mut instructions = Vec::new();
    let mut input = Vec::new();
    for message in messages {
        let role = string(message, "role")?;
        let text = text_content(message.get("content"))?;
        match role {
            "system" | "developer" => instructions.push(text),
            "tool" => input.push(json!({"type":"function_call_output",
                "call_id":string(message, "tool_call_id")?, "output":text})),
            "user" | "assistant" => {
                if !text.is_empty() || role == "user" {
                    let kind = if role == "assistant" {
                        "output_text"
                    } else {
                        "input_text"
                    };
                    input.push(json!({"role":role,"content":[{"type":kind,"text":text}]}));
                }
                if let Some(calls) = message.get("tool_calls").filter(|v| !v.is_null()) {
                    for call in calls
                        .as_array()
                        .ok_or_else(|| invalid("invalid tool call history"))?
                    {
                        if role != "assistant" {
                            return Err(invalid("tool calls outside assistant history"));
                        }
                        let function = call
                            .get("function")
                            .ok_or_else(|| invalid("missing function"))?;
                        input.push(json!({"type":"function_call", "call_id":string(call,"id")?,
                            "name":string(function,"name")?, "arguments":string(function,"arguments")?}));
                    }
                }
            }
            _ => return Err(invalid("unsupported conversation role")),
        }
    }
    let mut body = json!({"model":model,"instructions":instructions.join("\n\n"),
        "input":input,"store":false,"stream":true});
    if !tools.is_empty() {
        let functions = tools.iter().map(|tool| {
            let function = tool.get("function").ok_or_else(|| invalid("invalid function schema"))?;
            if tool.get("type").and_then(Value::as_str) != Some("function") {
                return Err(invalid("only local function tools are supported"));
            }
            Ok(json!({"type":"function", "name":string(function,"name")?,
                "description":function.get("description").cloned().unwrap_or(json!("")),
                "parameters":function.get("parameters").ok_or_else(|| invalid("missing tool parameters"))?,
                "strict":false}))
        }).collect::<Result<Vec<_>, ModelError>>()?;
        let object = body
            .as_object_mut()
            .ok_or_else(|| invalid("invalid request body"))?;
        object.insert("tools".into(), json!(functions));
        object.insert("tool_choice".into(), json!("auto"));
    }
    Ok(body)
}

pub(crate) fn decode_stream(bytes: &[u8]) -> Result<Reply, ModelError> {
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("invalid UTF-8 response stream"))?;
    let normalized = text.replace("\r\n", "\n");
    if !normalized.ends_with("\n\n") {
        return Err(invalid("response stream ended inside an event frame"));
    }
    let mut completed = None;
    for event in normalized.split("\n\n") {
        let data = event
            .lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .map(|line| line.strip_prefix(' ').unwrap_or(line))
            .collect::<Vec<_>>()
            .join("\n");
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        let event: Value =
            serde_json::from_str(&data).map_err(|_| invalid("invalid response stream event"))?;
        match event.get("type").and_then(Value::as_str) {
            Some("response.completed") => {
                if completed.is_some() {
                    return Err(invalid("repeated response completion"));
                }
                completed = Some(decode_response(
                    event
                        .get("response")
                        .ok_or_else(|| invalid("missing completed response"))?,
                )?);
            }
            Some("error" | "response.failed" | "response.incomplete" | "response.cancelled") => {
                return Err(ModelError::Unavailable("subscription response failed, was interrupted or reached its usage limit; no calls admitted".into()));
            }
            Some(_) => {}
            None => return Err(invalid("missing response event type")),
        }
    }
    completed.ok_or_else(|| invalid("response stream ended without successful completion"))
}

fn decode_response(response: &Value) -> Result<Reply, ModelError> {
    if response.get("status").and_then(Value::as_str) != Some("completed")
        || response.get("error").is_some_and(|v| !v.is_null())
    {
        return Err(invalid("response did not complete successfully"));
    }
    let mut text = Vec::new();
    let mut calls = Vec::new();
    for item in response
        .get("output")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("missing response output"))?
    {
        if item
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|s| s != "completed")
        {
            return Err(invalid("incomplete response item"));
        }
        match item.get("type").and_then(Value::as_str) {
            Some("message") => {
                for block in item
                    .get("content")
                    .and_then(Value::as_array)
                    .ok_or_else(|| invalid("missing response content"))?
                {
                    if block.get("type").and_then(Value::as_str) != Some("output_text") {
                        return Err(invalid("unsupported or refused response content"));
                    }
                    text.push(string(block, "text")?.to_owned());
                }
            }
            Some("function_call") => {
                let id = string(item, "call_id")?.to_owned();
                if calls.iter().any(|call: &ToolCall| call.id == id) {
                    return Err(invalid("duplicate response call ID"));
                }
                calls.push(ToolCall {
                    id,
                    function: FunctionCall {
                        name: string(item, "name")?.to_owned(),
                        arguments: string(item, "arguments")?.to_owned(),
                    },
                });
            }
            Some("reasoning") => {}
            _ => return Err(invalid("unsupported response output item")),
        }
    }
    if text.is_empty() && calls.is_empty() {
        return Err(invalid("response contained no assistant reply"));
    }
    Ok(Reply {
        content: (!text.is_empty()).then(|| text.join("\n")),
        tool_calls: calls,
    })
}

pub(crate) fn string<'a>(value: &'a Value, field: &str) -> Result<&'a str, ModelError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("missing or invalid protocol string"))
}
