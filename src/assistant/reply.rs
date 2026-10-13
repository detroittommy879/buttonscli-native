//! SSE framing and strict, review-only assistant action parsing.

use serde::Deserialize;
use serde_json::Value;

const MAX_EVENT_BYTES: usize = 128 * 1024;
pub(crate) const MAX_ANSWER_CHARS: usize = 64 * 1024;
const MAX_ACTIONS: usize = 2;
const MAX_COMMAND_CHARS: usize = 4096;

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ReplyError {
    EventTooLarge,
    InvalidUtf8,
    InvalidEvent,
    AnswerTooLarge,
    IncompleteResponse,
    ProviderFailed,
    Refused,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum StreamItem {
    Delta(String),
    Model(String),
    Done,
}

#[derive(Default)]
pub(crate) struct SseDecoder {
    buffer: Vec<u8>,
    done: bool,
    responses: bool,
    anthropic: bool,
}

impl SseDecoder {
    pub(crate) fn anthropic() -> Self {
        Self {
            anthropic: true,
            ..Self::default()
        }
    }
    pub(crate) fn responses() -> Self {
        Self {
            responses: true,
            ..Self::default()
        }
    }

    pub(crate) fn completed(&self) -> bool {
        self.done
    }
    pub(crate) fn push(&mut self, bytes: &[u8]) -> Result<Vec<StreamItem>, ReplyError> {
        if self.done {
            return Ok(Vec::new());
        }
        self.buffer.extend_from_slice(bytes);
        if self.buffer.len() > MAX_EVENT_BYTES && !has_separator(&self.buffer) {
            return Err(ReplyError::EventTooLarge);
        }
        let mut output = Vec::new();
        while let Some((start, width)) = find_separator(&self.buffer) {
            let event = self.buffer.drain(..start).collect::<Vec<_>>();
            self.buffer.drain(..width);
            output.extend(self.decode_event(&event)?);
            if self.done {
                self.buffer.clear();
                break;
            }
        }
        if self.buffer.len() > MAX_EVENT_BYTES {
            return Err(ReplyError::EventTooLarge);
        }
        Ok(output)
    }

    pub(crate) fn finish(&mut self) -> Result<Vec<StreamItem>, ReplyError> {
        if self.buffer.is_empty() || self.done {
            return Ok(Vec::new());
        }
        let event = std::mem::take(&mut self.buffer);
        self.decode_event(&event)
    }

    fn decode_event(&mut self, bytes: &[u8]) -> Result<Vec<StreamItem>, ReplyError> {
        if bytes.len() > MAX_EVENT_BYTES {
            return Err(ReplyError::EventTooLarge);
        }
        let text = std::str::from_utf8(bytes).map_err(|_| ReplyError::InvalidUtf8)?;
        let mut data = Vec::new();
        for line in text.lines() {
            if let Some(value) = line.strip_prefix("data:") {
                data.push(value.strip_prefix(' ').unwrap_or(value));
            }
        }
        if data.is_empty() {
            return Ok(Vec::new());
        }
        let payload = data.join("\n");
        if payload.trim() == "[DONE]" {
            if self.responses || self.anthropic {
                return Err(ReplyError::IncompleteResponse);
            }
            self.done = true;
            return Ok(vec![StreamItem::Done]);
        }
        let value: Value = serde_json::from_str(&payload).map_err(|_| ReplyError::InvalidEvent)?;
        if value.get("error").is_some() {
            return Err(if self.anthropic || self.responses {
                ReplyError::ProviderFailed
            } else {
                ReplyError::InvalidEvent
            });
        }
        if self.anthropic {
            return match value["type"].as_str() {
                Some("content_block_delta") if value["delta"]["type"] == "text_delta" => {
                    Ok(vec![StreamItem::Delta(
                        value["delta"]["text"]
                            .as_str()
                            .ok_or(ReplyError::InvalidEvent)?
                            .into(),
                    )])
                }
                Some("content_block_start") if value["content_block"]["type"] == "text" => {
                    let text = value["content_block"]["text"]
                        .as_str()
                        .ok_or(ReplyError::InvalidEvent)?;
                    Ok(if text.is_empty() {
                        vec![]
                    } else {
                        vec![StreamItem::Delta(text.into())]
                    })
                }
                Some("message_delta") if value["delta"]["stop_reason"] == "max_tokens" => {
                    Err(ReplyError::IncompleteResponse)
                }
                Some("message_delta") if value["delta"]["stop_reason"] == "refusal" => {
                    Err(ReplyError::Refused)
                }
                Some("message_stop") => {
                    self.done = true;
                    Ok(vec![StreamItem::Done])
                }
                Some("error") => Err(ReplyError::ProviderFailed),
                Some(_) => Ok(vec![]),
                None => Err(ReplyError::InvalidEvent),
            };
        }
        if self.responses {
            return match value["type"].as_str() {
                Some("response.output_text.delta") => {
                    let delta = value["delta"].as_str().ok_or(ReplyError::InvalidEvent)?;
                    Ok(vec![StreamItem::Delta(delta.into())])
                }
                Some("response.completed") => {
                    if value["response"]["status"].as_str() != Some("completed") {
                        return Err(ReplyError::IncompleteResponse);
                    }
                    self.done = true;
                    Ok(vec![StreamItem::Done])
                }
                Some("response.failed" | "error") => Err(ReplyError::ProviderFailed),
                Some("response.incomplete") => Err(ReplyError::IncompleteResponse),
                Some("response.refusal.delta" | "response.refusal.done") => {
                    Err(ReplyError::Refused)
                }
                Some(_) => Ok(Vec::new()),
                None => Err(ReplyError::InvalidEvent),
            };
        }
        let mut items = Vec::new();
        if let Some(model) = value
            .get("model")
            .and_then(Value::as_str)
            .filter(|v| v.len() <= 256)
        {
            items.push(StreamItem::Model(model.to_owned()));
        }
        if let Some(choices) = value.get("choices").and_then(Value::as_array) {
            for choice in choices {
                let content = choice
                    .get("delta")
                    .and_then(|value| value.get("content"))
                    .or_else(|| choice.get("message").and_then(|value| value.get("content")))
                    .or_else(|| choice.get("text"));
                if let Some(text) = flatten_content(content) {
                    if !text.is_empty() {
                        items.push(StreamItem::Delta(text));
                    }
                }
            }
        }
        Ok(items)
    }
}

#[cfg(test)]
mod responses_tests {
    use super::*;

    #[test]
    fn anthropic_fragmented_unicode_ignores_thinking_and_requires_message_stop() {
        let wire = concat!(
            "data: {\"type\":\"content_block_start\",\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"private\"}}\n\n",
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"hello 🦇\"}}\n\n",
            "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"}}\n\n"
        );
        let mut decoder = SseDecoder::anthropic();
        let mut items = vec![];
        for byte in wire.as_bytes() {
            items.extend(decoder.push(&[*byte]).unwrap());
        }
        assert_eq!(items, [StreamItem::Delta("hello 🦇".into())]);
        assert!(!decoder.completed());
        assert_eq!(
            decoder
                .push(b"data: {\"type\":\"message_stop\"}\n\n")
                .unwrap(),
            [StreamItem::Done]
        );
        assert!(decoder.completed());
        for (wire, expected) in [
            ("data: [DONE]\n\n", ReplyError::IncompleteResponse),
            (
                "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"max_tokens\"}}\n\n",
                ReplyError::IncompleteResponse,
            ),
            (
                "data: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\"}}\n\n",
                ReplyError::ProviderFailed,
            ),
        ] {
            assert_eq!(SseDecoder::anthropic().push(wire.as_bytes()), Err(expected));
        }
    }

    #[test]
    fn responses_events_handle_fragmented_unicode_without_duplicate_final_text() {
        let wire = concat!(
            "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{\"model\":\"test-model\"}}\n\n",
            "event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"hello 🦇\"}\n\n",
            "data: {\"type\":\"response.output_text.done\",\"text\":\"hello 🦇\"}\n\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\"}}\n\n"
        );
        let mut decoder = SseDecoder::responses();
        let mut items = Vec::new();
        for byte in wire.as_bytes() {
            items.extend(decoder.push(&[*byte]).unwrap());
        }
        assert_eq!(
            items,
            [StreamItem::Delta("hello 🦇".into()), StreamItem::Done]
        );
        assert!(decoder.completed());
    }

    #[test]
    fn responses_failure_incomplete_and_refusal_are_not_success() {
        for (kind, expected) in [
            ("response.failed", ReplyError::ProviderFailed),
            ("response.incomplete", ReplyError::IncompleteResponse),
            ("response.refusal.delta", ReplyError::Refused),
        ] {
            let wire = format!("data: {{\"type\":\"{kind}\"}}\n\n");
            assert_eq!(SseDecoder::responses().push(wire.as_bytes()), Err(expected));
        }
        assert_eq!(
            SseDecoder::responses().push(b"data: [DONE]\n\n"),
            Err(ReplyError::IncompleteResponse)
        );
    }
}

fn has_separator(bytes: &[u8]) -> bool {
    find_separator(bytes).is_some()
}

fn find_separator(bytes: &[u8]) -> Option<(usize, usize)> {
    let lf = bytes
        .windows(2)
        .position(|part| part == b"\n\n")
        .map(|index| (index, 2));
    let crlf = bytes
        .windows(4)
        .position(|part| part == b"\r\n\r\n")
        .map(|index| (index, 4));
    match (lf, crlf) {
        (Some(a), Some(b)) => Some(if a.0 < b.0 { a } else { b }),
        (Some(a), None) => Some(a),
        (None, b) => b,
    }
}

pub(crate) fn flatten_content(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => Some(text.clone()),
        Value::Array(parts) => Some(
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect(),
        ),
        _ => None,
    }
}

#[derive(Default)]
pub(crate) struct AnswerBuffer {
    text: String,
    chars: usize,
}

impl AnswerBuffer {
    pub(crate) fn append(&mut self, delta: &str) -> Result<(), ReplyError> {
        let delta_chars = delta.chars().count();
        let new_len = self.chars.saturating_add(delta_chars);
        if new_len > MAX_ANSWER_CHARS {
            return Err(ReplyError::AnswerTooLarge);
        }
        self.text.push_str(delta);
        self.chars = new_len;
        Ok(())
    }
    pub(crate) fn text(&self) -> &str {
        &self.text
    }
    pub(crate) fn into_string(self) -> String {
        self.text
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum SuggestedAction {
    Command {
        label: String,
        description: String,
        command: String,
        send_enter: bool,
    },
    Control {
        label: String,
        description: String,
        key: ControlKey,
    },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum ControlKey {
    CtrlC,
    CtrlD,
    CtrlZ,
    Enter,
    Tab,
    Escape,
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct AssistantReply {
    pub answer: String,
    pub actions: Vec<SuggestedAction>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ActionInput {
    List(Vec<ActionObject>),
    One(ActionObject),
}

#[derive(Deserialize)]
struct ActionObject {
    label: Option<String>,
    kind: Option<String>,
    control: Option<String>,
    command: Option<String>,
    description: Option<String>,
    reason: Option<String>,
    #[serde(rename = "sendEnter")]
    send_enter: Option<bool>,
}

#[derive(Deserialize)]
struct StructuredReply {
    answer: Option<String>,
    commands: Option<ActionInput>,
    action: Option<ActionObject>,
}

/// Display only answer text while the response envelope is incomplete.
/// Actions remain unavailable until the complete reply has been validated.
pub(crate) fn streamed_answer(raw: &str) -> String {
    let raw = raw.trim_start();
    if raw.starts_with('<') {
        let lower = raw.to_ascii_lowercase();
        let Some(start) = lower.find("<answer>").map(|start| start + 8) else {
            return String::new();
        };
        let tail = &raw[start..];
        // Hold an incomplete closing tag rather than flashing it in the UI.
        let end = tail.find('<').unwrap_or(tail.len());
        return bounded(&tail[..end]);
    }
    if raw.starts_with('{') {
        return serde_json::from_str::<StructuredReply>(raw)
            .ok()
            .and_then(|reply| reply.answer)
            .map(|answer| bounded(&answer))
            .unwrap_or_default();
    }
    bounded(raw)
}

/// Supports the original XML response contract and the equivalent JSON form.
/// Plain prose remains an answer; fenced or inferred commands are never executable actions.
pub(crate) fn parse_assistant_reply(raw: &str) -> AssistantReply {
    let answer_tag = tagged(raw, "answer");
    let commands_tag = tagged(raw, "commands");
    if let Some(answer) = answer_tag {
        let actions = commands_tag.map(parse_actions).unwrap_or_default();
        return AssistantReply {
            answer: bounded(answer),
            actions,
        };
    }
    if let Ok(reply) = serde_json::from_str::<StructuredReply>(raw.trim()) {
        let mut actions = reply.commands.map(action_list).unwrap_or_default();
        if actions.is_empty() {
            if let Some(action) = reply.action {
                actions = action_list(ActionInput::One(action));
            }
        }
        return AssistantReply {
            answer: bounded(reply.answer.as_deref().unwrap_or(raw.trim())),
            actions,
        };
    }
    AssistantReply {
        answer: bounded(raw.trim()),
        actions: Vec::new(),
    }
}

fn tagged<'a>(raw: &'a str, tag: &str) -> Option<&'a str> {
    let lower = raw.to_ascii_lowercase();
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = lower.find(&open)? + open.len();
    let end = lower[start..].find(&close)? + start;
    Some(raw[start..end].trim())
}

fn parse_actions(raw: &str) -> Vec<SuggestedAction> {
    let candidate = raw
        .trim()
        .trim_start_matches("```json")
        .trim_end_matches("```")
        .trim();
    match serde_json::from_str::<ActionInput>(candidate) {
        Ok(value) => action_list(value),
        Err(_) => Vec::new(),
    }
}

fn action_list(input: ActionInput) -> Vec<SuggestedAction> {
    let values = match input {
        ActionInput::List(values) => values,
        ActionInput::One(value) => vec![value],
    };
    values
        .into_iter()
        .take(MAX_ACTIONS)
        .filter_map(validate_action)
        .collect()
}

fn validate_action(value: ActionObject) -> Option<SuggestedAction> {
    let label = clean_text(value.label.as_deref()?, 120)?;
    let description = clean_text(
        value
            .description
            .as_deref()
            .or(value.reason.as_deref())
            .unwrap_or(""),
        500,
    )
    .unwrap_or_default();
    if value
        .kind
        .as_deref()
        .is_some_and(|kind| kind.eq_ignore_ascii_case("control"))
    {
        let key = match value
            .control
            .as_deref()?
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "ctrl+c" => ControlKey::CtrlC,
            "ctrl+d" => ControlKey::CtrlD,
            "ctrl+z" => ControlKey::CtrlZ,
            "enter" => ControlKey::Enter,
            "tab" => ControlKey::Tab,
            "escape" => ControlKey::Escape,
            "up" => ControlKey::Up,
            "down" => ControlKey::Down,
            "left" => ControlKey::Left,
            "right" => ControlKey::Right,
            _ => return None,
        };
        return Some(SuggestedAction::Control {
            label,
            description,
            key,
        });
    }
    let command = clean_text(value.command.as_deref()?, MAX_COMMAND_CHARS)?;
    if command.is_empty() || command.chars().any(|ch| matches!(ch, '\n' | '\r' | '\0')) {
        return None;
    }
    Some(SuggestedAction::Command {
        label,
        description,
        command,
        send_enter: value.send_enter.unwrap_or(false),
    })
}

fn clean_text(raw: &str, limit: usize) -> Option<String> {
    let text = raw.trim();
    if text.is_empty() || text.chars().count() > limit || text.chars().any(char::is_control) {
        return None;
    }
    Some(text.to_owned())
}

fn bounded(text: &str) -> String {
    text.chars().take(MAX_ANSWER_CHARS).collect()
}
