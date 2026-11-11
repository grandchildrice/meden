//! Bounded SSE observation. Progress is provisional; only a complete decoded
//! response returns calls to the conversation controller.
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Read};

use meden_agent::conversation::{Delta, ModelError, Reply};
use serde_json::{json, Value};

use crate::openai::invalid;
use crate::provider::Protocol;
use crate::responses::string;

pub(crate) fn read_response(
    response: &mut ureq::http::Response<ureq::Body>,
    protocol: &Protocol,
    notify: &dyn Fn(Delta<'_>),
    limit: u64,
) -> Result<Reply, ModelError> {
    let sse = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(';')
                .next()
                .is_some_and(|v| v.trim() == "text/event-stream")
        });
    let mut reader = response
        .body_mut()
        .as_reader()
        .take(limit.saturating_add(1));
    if sse || matches!(protocol, Protocol::Responses { .. }) {
        return read_sse(&mut reader, protocol, notify, limit);
    }
    // Some compatible/local servers return ordinary JSON even when asked to
    // stream. Decode that single response; do not retry or change endpoints.
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|_| invalid("incomplete API reply"))?;
    if bytes.len() as u64 > limit {
        return Err(invalid("API reply exceeded the size limit"));
    }
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|_| invalid("unreadable API reply"))?;
    match protocol {
        Protocol::Claude => crate::claude::decode_response(&value),
        Protocol::ChatCompletions => {
            if let Some(reasoning) = value
                .pointer("/choices/0/message/reasoning_content")
                .and_then(Value::as_str)
            {
                notify(Delta::Reasoning(reasoning));
            }
            crate::openai::decode_response(&value)
        }
        Protocol::Responses { .. } => Err(invalid("expected response stream")),
    }
}

fn read_sse(
    reader: impl Read,
    protocol: &Protocol,
    notify: &dyn Fn(Delta<'_>),
    limit: u64,
) -> Result<Reply, ModelError> {
    let mut reader = BufReader::new(reader);
    let mut bytes = Vec::new();
    let mut data = String::new();
    let mut chat = Chat::default();
    let mut claude = Claude::default();
    loop {
        let mut line = Vec::new();
        let count = reader
            .read_until(b'\n', &mut line)
            .map_err(|_| invalid("incomplete API stream"))?;
        if count == 0 {
            break;
        }
        if (bytes.len() as u64).saturating_add(count as u64) > limit {
            return Err(invalid("API reply exceeded the size limit"));
        }
        bytes.extend_from_slice(&line);
        if !line.ends_with(b"\n") {
            return Err(invalid("stream ended inside an event frame"));
        }
        let line = std::str::from_utf8(&line).map_err(|_| invalid("invalid UTF-8 stream"))?;
        let line = line.trim_end_matches('\n').trim_end_matches('\r');
        if let Some(value) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(value.strip_prefix(' ').unwrap_or(value));
        } else if line.is_empty() && !data.is_empty() {
            if data == "[DONE]" {
                if matches!(protocol, Protocol::ChatCompletions) {
                    chat.done = true;
                }
            } else {
                let event: Value = serde_json::from_str(&data)
                    .map_err(|_| invalid("invalid stream event JSON"))?;
                if event.get("error").is_some_and(|v| !v.is_null())
                    || event.get("type").and_then(Value::as_str) == Some("error")
                {
                    return Err(invalid(
                        "provider returned a stream error; no calls admitted",
                    ));
                }
                match protocol {
                    Protocol::Responses { .. } => match event.get("type").and_then(Value::as_str) {
                        Some("response.output_text.delta") => {
                            notify(Delta::Text(string(&event, "delta")?));
                        }
                        Some("response.reasoning_summary_text.delta") => {
                            notify(Delta::Reasoning(string(&event, "delta")?));
                        }
                        _ => {}
                    },
                    Protocol::ChatCompletions => chat.event(&event, notify)?,
                    Protocol::Claude => claude.event(&event, notify)?,
                }
            }
            data.clear();
        }
    }
    if !data.is_empty() {
        return Err(invalid("stream ended inside an event frame"));
    }
    match protocol {
        Protocol::Responses { .. } => crate::responses::decode_stream(&bytes),
        Protocol::ChatCompletions => chat.finish(),
        Protocol::Claude => claude.finish(),
    }
}

#[derive(Default)]
struct Chat {
    text: String,
    calls: BTreeMap<u64, Value>,
    finish_reason: Option<String>,
    done: bool,
}
impl Chat {
    fn event(&mut self, event: &Value, notify: &dyn Fn(Delta<'_>)) -> Result<(), ModelError> {
        if self.done {
            return Err(invalid("event after stream completion"));
        }
        let choices = event
            .get("choices")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("missing stream choices"))?;
        for choice in choices {
            if choice.get("index").and_then(Value::as_u64) != Some(0) {
                return Err(invalid("unsupported stream choice"));
            }
            if self.finish_reason.is_some() {
                return Err(invalid("event after choice completion"));
            }
            let delta = choice
                .get("delta")
                .ok_or_else(|| invalid("missing choice delta"))?;
            for key in ["reasoning_content", "reasoning"] {
                if let Some(text) = delta.get(key).and_then(Value::as_str) {
                    notify(Delta::Reasoning(text));
                }
            }
            if let Some(text) = delta.get("content").and_then(Value::as_str) {
                self.text.push_str(text);
                notify(Delta::Text(text));
            }
            if delta.get("refusal").is_some_and(|v| !v.is_null()) {
                return Err(invalid("model refused the request"));
            }
            if let Some(calls) = delta.get("tool_calls") {
                for call in calls
                    .as_array()
                    .ok_or_else(|| invalid("invalid stream calls"))?
                {
                    let index = call
                        .get("index")
                        .and_then(Value::as_u64)
                        .filter(|i| *i < 4096)
                        .ok_or_else(|| invalid("invalid call index"))?;
                    let target = self.calls.entry(index).or_insert_with(
                        || json!({"id":"","type":"function","function":{"name":"","arguments":""}}),
                    );
                    append(&mut target["id"], call.get("id"))?;
                    if let Some(function) = call.get("function") {
                        let target_function = target
                            .get_mut("function")
                            .ok_or_else(|| invalid("missing partial function"))?;
                        append(field_mut(target_function, "name")?, function.get("name"))?;
                        append(
                            field_mut(target_function, "arguments")?,
                            function.get("arguments"),
                        )?;
                    }
                }
            }
            if let Some(reason) = choice.get("finish_reason").filter(|v| !v.is_null()) {
                let reason = reason
                    .as_str()
                    .filter(|v| matches!(*v, "stop" | "tool_calls"))
                    .ok_or_else(|| invalid("model stream was truncated or interrupted"))?;
                self.finish_reason = Some(reason.to_owned());
            }
        }
        Ok(())
    }
    fn finish(self) -> Result<Reply, ModelError> {
        if !self.done || self.finish_reason.is_none() {
            return Err(invalid("stream ended without successful completion"));
        }
        let mut ids = BTreeSet::new();
        for (expected, (index, call)) in self.calls.iter().enumerate() {
            if *index != expected as u64
                || string(call, "id")?.is_empty()
                || !ids.insert(string(call, "id")?)
                || call
                    .pointer("/function/name")
                    .and_then(Value::as_str)
                    .is_none_or(str::is_empty)
            {
                return Err(invalid("incomplete or duplicate streamed call"));
            }
        }
        if (self.finish_reason.as_deref() == Some("tool_calls")) == self.calls.is_empty() {
            return Err(invalid("stream finish reason disagrees with tool calls"));
        }
        crate::openai::decode_response(
            &json!({"choices":[{"finish_reason":self.finish_reason,"message":{
            "content":self.text,"tool_calls":self.calls.into_values().collect::<Vec<_>>()}}]}),
        )
    }
}

fn append(target: &mut Value, part: Option<&Value>) -> Result<(), ModelError> {
    if let Some(part) = part.filter(|v| !v.is_null()) {
        let mut value = target.as_str().unwrap_or_default().to_owned();
        value.push_str(
            part.as_str()
                .ok_or_else(|| invalid("invalid string delta"))?,
        );
        *target = Value::String(value);
    }
    Ok(())
}

#[derive(Default)]
struct Claude {
    message: Option<Value>,
    open: BTreeSet<usize>,
    arguments: BTreeMap<usize, String>,
    done: bool,
}
impl Claude {
    fn event(&mut self, event: &Value, notify: &dyn Fn(Delta<'_>)) -> Result<(), ModelError> {
        let kind = string(event, "type")?;
        if kind == "ping" {
            return Ok(());
        }
        if self.done {
            return Err(invalid("event after Claude completion"));
        }
        if kind == "message_start" {
            if self.message.is_some() {
                return Err(invalid("repeated Claude message"));
            }
            let message = event
                .get("message")
                .ok_or_else(|| invalid("missing Claude message"))?;
            if message
                .get("content")
                .and_then(Value::as_array)
                .is_none_or(|v| !v.is_empty())
            {
                return Err(invalid("invalid initial Claude content"));
            }
            self.message = Some(message.clone());
            return Ok(());
        }
        let message = self
            .message
            .as_mut()
            .ok_or_else(|| invalid("Claude event before message start"))?;
        match kind {
            "content_block_start" => {
                let index = index(event)?;
                let blocks = message["content"]
                    .as_array_mut()
                    .ok_or_else(|| invalid("invalid Claude blocks"))?;
                if index != blocks.len() || !self.open.is_empty() {
                    return Err(invalid("out of order Claude block"));
                }
                let block = event
                    .get("content_block")
                    .ok_or_else(|| invalid("missing Claude block"))?;
                blocks.push(block.clone());
                self.open.insert(index);
                if let Some(text) = block.get("text").and_then(Value::as_str) {
                    notify(Delta::Text(text));
                }
            }
            "content_block_delta" => {
                let index = index(event)?;
                if !self.open.contains(&index) {
                    return Err(invalid("delta outside an open Claude block"));
                }
                let block = message["content"]
                    .get_mut(index)
                    .ok_or_else(|| invalid("unknown Claude block"))?;
                let delta = event
                    .get("delta")
                    .ok_or_else(|| invalid("missing Claude delta"))?;
                match string(delta, "type")? {
                    "text_delta" if block.get("type").and_then(Value::as_str) == Some("text") => {
                        let text = string(delta, "text")?;
                        append(&mut block["text"], Some(&Value::String(text.into())))?;
                        notify(Delta::Text(text));
                    }
                    "input_json_delta"
                        if block.get("type").and_then(Value::as_str) == Some("tool_use") =>
                    {
                        self.arguments
                            .entry(index)
                            .or_default()
                            .push_str(string(delta, "partial_json")?);
                    }
                    // Signed thinking requires history replay support before it
                    // can be admitted. The current Claude adapter leaves it off.
                    _ => return Err(invalid("unsupported Claude delta")),
                }
            }
            "content_block_stop" => {
                let index = index(event)?;
                if !self.open.remove(&index) {
                    return Err(invalid("stopping unopened Claude block"));
                }
                if let Some(text) = self.arguments.remove(&index) {
                    let block = message
                        .get_mut("content")
                        .and_then(|v| v.get_mut(index))
                        .ok_or_else(|| invalid("unknown Claude block"))?;
                    *field_mut(block, "input")? = serde_json::from_str(&text)
                        .map_err(|_| invalid("incomplete Claude tool JSON"))?;
                }
            }
            "message_delta" if self.open.is_empty() => {
                message["stop_reason"] = event
                    .pointer("/delta/stop_reason")
                    .cloned()
                    .ok_or_else(|| invalid("missing Claude stop reason"))?;
            }
            "message_stop" if self.open.is_empty() => self.done = true,
            _ => return Err(invalid("unsupported or out of order Claude event")),
        }
        Ok(())
    }
    fn finish(self) -> Result<Reply, ModelError> {
        if !self.done || !self.open.is_empty() {
            return Err(invalid("Claude stream ended without completion"));
        }
        crate::claude::decode_response(
            &self
                .message
                .ok_or_else(|| invalid("missing Claude message"))?,
        )
    }
}
fn index(event: &Value) -> Result<usize, ModelError> {
    event
        .get("index")
        .and_then(Value::as_u64)
        .filter(|v| *v < 4096)
        .and_then(|v| usize::try_from(v).ok())
        .ok_or_else(|| invalid("invalid Claude block index"))
}

fn field_mut<'a>(value: &'a mut Value, field: &str) -> Result<&'a mut Value, ModelError> {
    value
        .get_mut(field)
        .ok_or_else(|| invalid("missing stream field"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::indexing_slicing)]
    use super::*;
    use std::cell::RefCell;

    fn frames(events: &[Value]) -> String {
        use std::fmt::Write as _;
        let mut output = String::new();
        for event in events {
            write!(output, "data: {event}\n\n").expect("string write");
        }
        output
    }

    #[test]
    fn chat_fragments_keep_original_calls_and_deltas_are_only_observations() {
        let mut stream = frames(&[
            json!({"choices":[{"index":0,"delta":{"reasoning_content":"Plan."}}]}),
            json!({"choices":[{"index":0,"delta":{"content":"Checking.","tool_calls":[{"index":0,"id":"call-1","function":{"name":"bash","arguments":"{\"command\":"}}]}}]}),
            json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"pwd\"}"}}]},"finish_reason":"tool_calls"}]}),
        ]);
        let seen = RefCell::new(Vec::new());
        let observe = |delta: Delta<'_>| match delta {
            Delta::Text(t) | Delta::Reasoning(t) => seen.borrow_mut().push(t.to_owned()),
        };
        assert!(
            read_sse(
                stream.as_bytes(),
                &Protocol::ChatCompletions,
                &observe,
                4096
            )
            .is_err(),
            "partial calls cannot be admitted"
        );
        assert_eq!(*seen.borrow(), ["Plan.", "Checking."]);
        stream.push_str("data: [DONE]\n\n");
        let reply = read_sse(stream.as_bytes(), &Protocol::ChatCompletions, &|_| {}, 4096)
            .expect("complete stream");
        assert_eq!(reply.content.as_deref(), Some("Checking."));
        assert_eq!(reply.tool_calls[0].id, "call-1");
        assert_eq!(
            reply.tool_calls[0].function.arguments,
            r#"{"command":"pwd"}"#
        );
    }

    #[test]
    fn interrupted_invalid_and_oversized_streams_never_return_a_reply() {
        let choice = json!({"choices":[{"index":0,"delta":{"content":"Unverified text"}}]});
        for tail in [
            "",
            "data: [DONE]\n\n",
            "data: invalid\n\n",
            "data: {\"error\":{}}\n\n",
        ] {
            let stream = format!("{}{tail}", frames(std::slice::from_ref(&choice)));
            assert!(
                read_sse(stream.as_bytes(), &Protocol::ChatCompletions, &|_| {}, 4096).is_err()
            );
        }
        let stream = frames(&[choice]);
        assert!(read_sse(stream.as_bytes(), &Protocol::ChatCompletions, &|_| {}, 12).is_err());
    }

    #[test]
    fn response_progress_does_not_replace_the_completed_response() {
        let stream = frames(&[
            json!({"type":"response.reasoning_summary_text.delta","delta":"Plan."}),
            json!({"type":"response.output_text.delta","delta":"Provisional"}),
            json!({"type":"response.completed","response":{"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"Final"}]}]}}),
        ]);
        let seen = RefCell::new(Vec::new());
        let reply = read_sse(
            stream.as_bytes(),
            &Protocol::Responses { account_id: None },
            &|delta| match delta {
                Delta::Text(t) | Delta::Reasoning(t) => seen.borrow_mut().push(t.to_owned()),
            },
            4096,
        )
        .expect("complete response");
        assert_eq!(*seen.borrow(), ["Plan.", "Provisional"]);
        assert_eq!(reply.content.as_deref(), Some("Final"));
        let failed = stream.replace("response.completed", "response.failed");
        assert!(read_sse(
            failed.as_bytes(),
            &Protocol::Responses { account_id: None },
            &|_| {},
            4096
        )
        .is_err());
    }

    #[test]
    fn claude_requires_closed_blocks_and_a_successful_terminal_message() {
        let events = [
            json!({"type":"message_start","message":{"type":"message","role":"assistant","content":[],"stop_reason":null}}),
            json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
            json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}),
            json!({"type":"content_block_stop","index":0}),
            json!({"type":"message_delta","delta":{"stop_reason":"end_turn"}}),
            json!({"type":"message_stop"}),
        ];
        let stream = frames(&events);
        assert_eq!(
            read_sse(stream.as_bytes(), &Protocol::Claude, &|_| {}, 4096)
                .expect("Claude completion")
                .content
                .as_deref(),
            Some("Hello")
        );
        for n in 0..events.len() {
            assert!(read_sse(
                frames(&events[..n]).as_bytes(),
                &Protocol::Claude,
                &|_| {},
                4096
            )
            .is_err());
        }
        assert!(read_sse(
            stream.replace("end_turn", "max_tokens").as_bytes(),
            &Protocol::Claude,
            &|_| {},
            4096
        )
        .is_err());
    }
}
