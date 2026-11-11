//! Anthropic Messages conversion for host-selected OAuth/setup tokens.
//! Tool calls return to Meden's controller; this adapter executes no tools.

use meden_agent::conversation::{FunctionCall, ModelError, Reply, ToolCall};
use serde_json::{json, Value};

use crate::openai::{invalid, text_content};
use crate::responses::string;

pub(crate) fn request_body(
    model: &str,
    messages: &[Value],
    tools: &[Value],
) -> Result<Value, ModelError> {
    let mut system = Vec::new();
    let mut converted: Vec<Value> = Vec::new();
    for message in messages {
        let role = string(message, "role")?;
        let text = text_content(message.get("content"))?;
        if matches!(role, "system" | "developer") {
            system.push(text);
            continue;
        }
        let mut blocks = Vec::new();
        if role == "tool" {
            blocks.push(json!({"type":"tool_result", "tool_use_id":string(message,"tool_call_id")?, "content":text}));
        } else {
            if !matches!(role, "user" | "assistant") {
                return Err(invalid("unsupported conversation role"));
            }
            if !text.is_empty() {
                blocks.push(json!({"type":"text","text":text}));
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
                    let input: Value = serde_json::from_str(string(function, "arguments")?)
                        .map_err(|_| invalid("invalid JSON in tool call history"))?;
                    blocks.push(json!({"type":"tool_use", "id":string(call,"id")?,
                        "name":string(function,"name")?, "input":input}));
                }
            }
        }
        let role = if role == "tool" { "user" } else { role };
        // Parallel tool results must share one user message. The same merge
        // also handles adjacent user messages without inventing a reply.
        if let Some(previous) = converted
            .last_mut()
            .filter(|v| v.get("role").and_then(Value::as_str) == Some(role))
        {
            previous
                .get_mut("content")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| invalid("invalid converted content"))?
                .extend(blocks);
        } else if !blocks.is_empty() {
            converted.push(json!({"role":role,"content":blocks}));
        }
    }
    let mut body = json!({"model":model,"max_tokens":16384,"system":system.join("\n\n"),"messages":converted,"stream":true});
    if !tools.is_empty() {
        let functions = tools.iter().map(|tool| {
            let function = tool.get("function").ok_or_else(|| invalid("invalid function schema"))?;
            if tool.get("type").and_then(Value::as_str) != Some("function") {
                return Err(invalid("only local function tools are supported"));
            }
            Ok(json!({"name":string(function,"name")?,
                "description":function.get("description").cloned().unwrap_or(json!("")),
                "input_schema":function.get("parameters").ok_or_else(|| invalid("missing tool parameters"))?}))
        }).collect::<Result<Vec<_>, ModelError>>()?;
        let object = body
            .as_object_mut()
            .ok_or_else(|| invalid("invalid request body"))?;
        object.insert("tools".into(), json!(functions));
        object.insert("tool_choice".into(), json!({"type":"auto"}));
    }
    Ok(body)
}

pub(crate) fn decode_response(response: &Value) -> Result<Reply, ModelError> {
    if response.get("type").and_then(Value::as_str) != Some("message")
        || response.get("role").and_then(Value::as_str) != Some("assistant")
        || !matches!(
            response.get("stop_reason").and_then(Value::as_str),
            Some("end_turn" | "tool_use" | "stop_sequence")
        )
    {
        return Err(invalid(
            "Claude response failed, was truncated or did not finish",
        ));
    }
    let mut text = Vec::new();
    let mut calls = Vec::new();
    for block in response
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("missing Claude content"))?
    {
        match block.get("type").and_then(Value::as_str) {
            Some("text") => text.push(string(block, "text")?.to_owned()),
            Some("tool_use") => {
                let id = string(block, "id")?.to_owned();
                if calls.iter().any(|call: &ToolCall| call.id == id) {
                    return Err(invalid("duplicate Claude call ID"));
                }
                let input = block
                    .get("input")
                    .filter(|v| v.is_object())
                    .ok_or_else(|| invalid("invalid Claude tool input"))?;
                calls.push(ToolCall {
                    id,
                    function: FunctionCall {
                        name: string(block, "name")?.to_owned(),
                        arguments: serde_json::to_string(input)
                            .map_err(|_| invalid("invalid Claude tool JSON"))?,
                    },
                });
            }
            // Thinking is disabled in requests, so signed blocks need not be
            // replayed. Unexpected protocol content is never silently lost.
            _ => return Err(invalid("unsupported Claude content block")),
        }
    }
    if text.is_empty() && calls.is_empty() {
        return Err(invalid("Claude response contained no assistant reply"));
    }
    if (response.get("stop_reason").and_then(Value::as_str) == Some("tool_use")) == calls.is_empty()
    {
        return Err(invalid("Claude stop reason disagrees with its tool calls"));
    }
    Ok(Reply {
        content: (!text.is_empty()).then(|| text.join("\n")),
        tool_calls: calls,
    })
}
