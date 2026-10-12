//! Provider connection checks and model discovery. Responses are bounded by transport.

use std::cell::Cell;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use zeroize::Zeroizing;

use super::provider::{validate_endpoint, ProviderApi, ProviderProfile};
use super::reply::{flatten_content, AnswerBuffer, ReplyError, SseDecoder, StreamItem};
use super::transport::{HttpTransport, Method, Request, ResponseHead, TransportError};

pub(crate) type KeyLoader<'a> =
    dyn FnMut(&ProviderProfile) -> Result<Option<Zeroizing<String>>, ClientError> + 'a;

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ClientError {
    Endpoint,
    Model,
    Http(u16),
    HttpDetail(u16, String),
    Transport(TransportError),
    InvalidReply,
    Reply(ReplyError),
    Cancelled,
    RequestTooLarge,
    Quota,
    Credential,
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Endpoint => f.write_str("configure a valid provider endpoint"),
            Self::Model => f.write_str("enter a model ID"),
            Self::Http(429) => f.write_str("provider returned HTTP 429 (rate limit or quota); wait or check the provider account"),
            Self::Http(status) => write!(f, "provider returned HTTP {status}"),
            Self::HttpDetail(status, message) => write!(f, "HTTP {status}: {message}"),
            Self::Transport(error) => write!(f, "{error}"),
            Self::InvalidReply => f.write_str("provider returned an invalid response"),
            Self::Reply(error) => write!(f, "assistant response could not be read: {error:?}"),
            Self::Cancelled => f.write_str("provider request was cancelled"),
            Self::RequestTooLarge => f.write_str("provider request exceeded 1 MiB"),
            Self::Quota => f.write_str("provider quota exhausted; check billing or configure a fallback"),
            Self::Credential => f.write_str("provider credential could not be loaded"),
        }
    }
}

#[cfg(test)]
fn completion_url(endpoint: &str) -> Result<String, ClientError> {
    provider_url(&ProviderProfile {
        endpoint: endpoint.into(),
        api: ProviderApi::ChatCompletions,
        ..Default::default()
    })
}

pub(crate) fn provider_url(provider: &ProviderProfile) -> Result<String, ClientError> {
    validate_endpoint(&provider.endpoint).map_err(|_| ClientError::Endpoint)?;
    let base = provider.base_url().ok_or(ClientError::Endpoint)?;
    let leaf = match provider.resolved_api() {
        ProviderApi::Responses => "responses",
        ProviderApi::Anthropic => "messages",
        _ => "chat/completions",
    };
    Ok(format!("{base}/{leaf}"))
}

fn completion_body(
    provider: &ProviderProfile,
    prompts: (&str, &str),
    history: &[(bool, String)],
    stream: bool,
) -> Value {
    let mut messages = Vec::with_capacity(history.len() + 2);
    if provider.resolved_api() == ProviderApi::ChatCompletions {
        messages.push(json!({"role":"system", "content":prompts.0}));
    }
    messages.extend(history.iter().map(|(assistant, content)| {
        json!({
            "role": if *assistant { "assistant" } else { "user" }, "content": content,
        })
    }));
    messages.push(json!({"role":"user", "content":prompts.1}));
    if provider.resolved_api() == ProviderApi::Anthropic {
        json!({"model":provider.model, "system":prompts.0, "messages":messages, "stream":stream, "max_tokens":4096})
    } else if provider.uses_responses() {
        json!({"model":provider.model, "instructions":prompts.0, "input":messages, "stream":stream, "store":false})
    } else {
        json!({"model":provider.model, "messages":messages, "stream":stream})
    }
}

pub(crate) fn models_url(endpoint: &str) -> Result<String, ClientError> {
    validate_endpoint(endpoint).map_err(|_| ClientError::Endpoint)?;
    let provider = ProviderProfile {
        endpoint: endpoint.into(),
        ..Default::default()
    };
    Ok(format!(
        "{}/models",
        provider.base_url().ok_or(ClientError::Endpoint)?
    ))
}

pub(crate) fn collect_models(body: &[u8]) -> Result<Vec<String>, ClientError> {
    let value: Value = serde_json::from_slice(body).map_err(|_| ClientError::InvalidReply)?;
    let mut models = BTreeSet::new();
    for list in ["data", "models"] {
        if let Some(entries) = value.get(list).and_then(Value::as_array) {
            for entry in entries {
                let id = entry
                    .as_str()
                    .or_else(|| entry.get("id").and_then(Value::as_str))
                    .or_else(|| entry.get("name").and_then(Value::as_str));
                if let Some(id) = id.map(str::trim).filter(|id| {
                    !id.is_empty() && id.len() <= 256 && !id.chars().any(char::is_control)
                }) {
                    models.insert(id.to_owned());
                }
            }
        }
    }
    Ok(models.into_iter().take(500).collect())
}

pub(crate) fn discover_models(
    transport: &dyn HttpTransport,
    provider: &ProviderProfile,
    key: Option<Zeroizing<String>>,
) -> Result<Vec<String>, ClientError> {
    let primary = match &provider.models_endpoint {
        Some(endpoint) => {
            validate_endpoint(endpoint).map_err(|_| ClientError::Endpoint)?;
            endpoint.clone()
        }
        None => models_url(&provider.endpoint)?,
    };
    let mut candidates = vec![primary];
    if provider.models_endpoint.is_none() {
        let base = provider.base_url().ok_or(ClientError::Endpoint)?;
        if !base.rsplit('/').next().is_some_and(|segment| {
            segment
                .strip_prefix('v')
                .is_some_and(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
        }) {
            candidates.push(format!("{base}/v1/models"));
        }
    }
    for (index, mut url) in candidates.iter().cloned().enumerate() {
        if provider.resolved_api() == ProviderApi::Anthropic {
            url.push_str("?limit=500");
        }
        let response = transport
            .execute(Request {
                anthropic: provider.resolved_api() == ProviderApi::Anthropic,
                method: Method::Get,
                url,
                body: None,
                key: key.clone(),
            })
            .map_err(ClientError::Transport)?;
        if matches!(response.status, 404 | 405) && index + 1 < candidates.len() {
            continue;
        }
        if !(200..300).contains(&response.status) {
            return Err(http_error(
                response.status,
                &response.body,
                key.as_deref().map(String::as_str),
            ));
        }
        return collect_models(&response.body);
    }
    Err(ClientError::InvalidReply)
}

pub(crate) fn test_connection(
    transport: &dyn HttpTransport,
    provider: &ProviderProfile,
    key: Option<Zeroizing<String>>,
) -> Result<(), ClientError> {
    validate_endpoint(&provider.endpoint).map_err(|_| ClientError::Endpoint)?;
    if provider.model.trim().is_empty() {
        return Err(ClientError::Model);
    }
    let mut body = completion_body(provider, ("Reply briefly.", "Reply OK."), &[], false);
    match provider.resolved_api() {
        ProviderApi::Responses => body["max_output_tokens"] = json!(1024),
        ProviderApi::Anthropic => body["max_tokens"] = json!(1024),
        _ => {
            let official = url::Url::parse(&provider.endpoint)
                .is_ok_and(|url| url.host_str() == Some("api.openai.com"));
            body[if official {
                "max_completion_tokens"
            } else {
                "max_tokens"
            }] = json!(1024);
        }
    }
    let redaction_key = key.clone();
    let response = transport
        .execute(Request {
            anthropic: provider.resolved_api() == ProviderApi::Anthropic,
            method: Method::Post,
            url: provider_url(provider)?,
            body: Some(body),
            key,
        })
        .map_err(ClientError::Transport)?;
    if !(200..300).contains(&response.status) {
        return Err(http_error(
            response.status,
            &response.body,
            redaction_key.as_deref().map(String::as_str),
        ));
    }
    provider_content(&response.body, provider).map(|_| ())
}

fn http_error(status: u16, body: &[u8], key: Option<&str>) -> ClientError {
    let value: Value = match serde_json::from_slice(body) {
        Ok(value) => value,
        Err(_) => return ClientError::Http(status),
    };
    let message = value["error"]["message"]
        .as_str()
        .or_else(|| value["message"].as_str())
        .or_else(|| value["detail"].as_str());
    let Some(message) = message else {
        return ClientError::Http(status);
    };
    let message = match key.filter(|key| !key.is_empty()) {
        Some(key) => message.replace(key, "[redacted]"),
        None => message.to_owned(),
    };
    let message = crate::session::context::redact_obvious_secrets(&message, key);
    ClientError::HttpDetail(
        status,
        message
            .chars()
            .filter(|c| !c.is_control())
            .take(512)
            .collect(),
    )
}

/// Explicit setup probe only; conversation requests never replay across formats.
pub(crate) fn detect_format(
    transport: &dyn HttpTransport,
    provider: &ProviderProfile,
    key: Option<Zeroizing<String>>,
) -> Result<ProviderApi, ClientError> {
    let mut formats = vec![provider.resolved_api()];
    for api in [
        ProviderApi::ChatCompletions,
        ProviderApi::Responses,
        ProviderApi::Anthropic,
    ] {
        if !formats.contains(&api) {
            formats.push(api);
        }
    }
    let mut error = ClientError::InvalidReply;
    for api in formats {
        let mut probe = provider.clone();
        probe.api = api;
        match test_connection(transport, &probe, key.clone()) {
            Ok(()) => return Ok(api),
            Err(failure) => {
                let may_probe = matches!(
                    failure,
                    ClientError::Http(400 | 401 | 404 | 405 | 415 | 422)
                        | ClientError::HttpDetail(400 | 401 | 404 | 405 | 415 | 422, _)
                        | ClientError::InvalidReply
                );
                if !may_probe {
                    return Err(failure);
                }
                error = failure;
            }
        }
    }
    Err(error)
}

pub(crate) fn stream_completion(
    transport: &dyn HttpTransport,
    provider: &ProviderProfile,
    key: Option<Zeroizing<String>>,
    prompts: (&str, &str),
    history: &[(bool, String)],
    cancelled: &AtomicBool,
    on_delta: &mut dyn FnMut(&str),
) -> Result<String, ClientError> {
    let started = Instant::now();
    let mut stream = true;
    let mut attempts = 0;
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err(ClientError::Cancelled);
        }
        attempts += 1;
        let mut emitted = false;
        let hints = AttemptHints::new(stream);
        let result = completion_attempt(
            transport,
            provider,
            key.as_ref()
                .map(|key| Zeroizing::new(key.as_str().to_owned())),
            prompts,
            history,
            (cancelled, &hints),
            &mut |delta| {
                emitted |= !delta.is_empty();
                on_delta(delta);
            },
        );
        let Err(error) = result else {
            return result;
        };
        if emitted || cancelled.load(Ordering::Relaxed) || attempts >= 3 {
            return Err(error);
        }
        if stream && hints.unsupported_stream.get() {
            stream = false;
            continue;
        }
        if !error.can_retry() {
            return Err(error);
        }
        let delay = hints.retry_after.get().unwrap_or_else(|| {
            Duration::from_millis(250 * (1 << (attempts - 1)) + fastrand::u64(0..150))
        });
        // Never retry earlier than Retry-After, and never wait without a bound.
        if delay > Duration::from_secs(30) || started.elapsed() + delay > Duration::from_secs(180) {
            return Err(error);
        }
        let until = Instant::now() + delay;
        while Instant::now() < until {
            if cancelled.load(Ordering::Relaxed) {
                return Err(ClientError::Cancelled);
            }
            std::thread::sleep(
                Duration::from_millis(20).min(until.saturating_duration_since(Instant::now())),
            );
        }
    }
}

impl ClientError {
    fn can_retry(&self) -> bool {
        matches!(
            self,
            Self::Http(408 | 429 | 500..=599)
                | Self::HttpDetail(408 | 429 | 500..=599, _)
                | Self::Transport(TransportError::Network)
        )
    }

    pub(crate) fn can_fallback(&self) -> bool {
        self.can_retry() || matches!(self, Self::Quota)
    }
}

/// Never falls back after any answer text was exposed to the consumer.
pub(crate) fn complete_with_fallback(
    transport: &dyn HttpTransport,
    providers: &[ProviderProfile],
    load_key: &mut KeyLoader<'_>,
    prompts: (&str, &str),
    history: &[(bool, String)],
    cancelled: &AtomicBool,
    on_delta: &mut dyn FnMut(&str),
) -> Result<String, ClientError> {
    let mut last_error = ClientError::Model;
    for provider in providers.iter().take(4) {
        if cancelled.load(Ordering::Relaxed) {
            return Err(ClientError::Cancelled);
        }
        let key = load_key(provider)?;
        let mut emitted = false;
        let result = stream_completion(
            transport,
            provider,
            key,
            prompts,
            history,
            cancelled,
            &mut |delta| {
                emitted |= !delta.is_empty();
                on_delta(delta);
            },
        );
        match result {
            Ok(answer) => return Ok(answer),
            Err(error) if !emitted && error.can_fallback() => last_error = error,
            Err(error) => return Err(error),
        }
    }
    Err(last_error)
}

struct AttemptHints {
    stream: bool,
    retry_after: Cell<Option<Duration>>,
    quota: Cell<bool>,
    unsupported_stream: Cell<bool>,
}

impl AttemptHints {
    fn new(stream: bool) -> Self {
        Self {
            stream,
            retry_after: Cell::new(None),
            quota: Cell::new(false),
            unsupported_stream: Cell::new(false),
        }
    }
}

fn completion_attempt(
    transport: &dyn HttpTransport,
    provider: &ProviderProfile,
    key: Option<Zeroizing<String>>,
    prompts: (&str, &str),
    history: &[(bool, String)],
    control: (&AtomicBool, &AttemptHints),
    on_delta: &mut dyn FnMut(&str),
) -> Result<String, ClientError> {
    let (cancelled, hints) = control;
    validate_endpoint(&provider.endpoint).map_err(|_| ClientError::Endpoint)?;
    if provider.model.trim().is_empty() {
        return Err(ClientError::Model);
    }
    let status = Cell::new(0_u16);
    let is_sse = Cell::new(false);
    let mut raw = Vec::new();
    let mut decoder = if provider.uses_responses() {
        SseDecoder::responses()
    } else if provider.resolved_api() == ProviderApi::Anthropic {
        SseDecoder::anthropic()
    } else {
        SseDecoder::default()
    };
    let mut answer = AnswerBuffer::default();
    let mut parser_error = None;
    let mut on_head = |head: ResponseHead| {
        status.set(head.status);
        hints.retry_after.set(head.retry_after);
        is_sse.set(
            head.content_type
                .as_deref()
                .is_some_and(|value| value.to_ascii_lowercase().contains("text/event-stream")),
        );
    };
    let mut on_chunk = |chunk: &[u8]| {
        if cancelled.load(Ordering::Relaxed) {
            return Err(TransportError::Cancelled);
        }
        if !(200..300).contains(&status.get()) {
            raw.extend_from_slice(chunk);
            return if raw.len() > 1024 * 1024 {
                Err(TransportError::ResponseTooLarge)
            } else {
                Ok(())
            };
        }
        if !is_sse.get() && raw.is_empty() {
            let prefix = String::from_utf8_lossy(chunk)
                .trim_start()
                .to_ascii_lowercase();
            is_sse.set(
                prefix.starts_with("data:")
                    || prefix.starts_with("event:")
                    || prefix.starts_with(':'),
            );
        }
        if is_sse.get() {
            match decoder.push(chunk) {
                Ok(items) => {
                    for item in items {
                        if let StreamItem::Delta(delta) = item {
                            if let Err(error) = answer.append(&delta) {
                                parser_error = Some(error);
                                return Err(TransportError::Cancelled);
                            }
                            on_delta(&delta);
                        }
                    }
                }
                Err(error) => {
                    parser_error = Some(error);
                    return Err(TransportError::Cancelled);
                }
            }
        } else {
            raw.extend_from_slice(chunk);
            if raw.len() > 1024 * 1024 {
                return Err(TransportError::ResponseTooLarge);
            }
        }
        Ok(())
    };
    let request_body = completion_body(provider, prompts, history, hints.stream);
    if serde_json::to_vec(&request_body)
        .map_err(|_| ClientError::InvalidReply)?
        .len()
        > 1024 * 1024
    {
        return Err(ClientError::RequestTooLarge);
    }
    let redaction_key = key.clone();
    let request = Request {
        anthropic: provider.resolved_api() == ProviderApi::Anthropic,
        method: Method::Post,
        url: provider_url(provider)?,
        body: Some(request_body),
        key,
    };
    let response = transport.execute_stream(request, cancelled, &mut on_head, &mut on_chunk);
    if let Some(error) = parser_error {
        return Err(ClientError::Reply(error));
    }
    response.map_err(|error| {
        if error == TransportError::Cancelled {
            ClientError::Cancelled
        } else {
            ClientError::Transport(error)
        }
    })?;
    if cancelled.load(Ordering::Relaxed) {
        return Err(ClientError::Cancelled);
    }
    if !(200..300).contains(&status.get()) {
        if let Ok(value) = serde_json::from_slice::<Value>(&raw) {
            let error = &value["error"];
            let code = error["code"]
                .as_str()
                .or_else(|| error["type"].as_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            hints.quota.set(matches!(
                code.as_str(),
                "insufficient_quota" | "quota_exceeded" | "billing_hard_limit_reached"
            ));
            hints.unsupported_stream.set(
                matches!(status.get(), 400 | 422)
                    && (matches!(
                        code.as_str(),
                        "unsupported_stream" | "streaming_not_supported"
                    ) || (error["param"].as_str() == Some("stream")
                        && code == "unsupported_value")),
            );
        }
        return Err(if hints.quota.get() {
            ClientError::Quota
        } else {
            http_error(
                status.get(),
                &raw,
                redaction_key.as_deref().map(String::as_str),
            )
        });
    }
    if is_sse.get() {
        for item in decoder.finish().map_err(ClientError::Reply)? {
            if let StreamItem::Delta(delta) = item {
                answer.append(&delta).map_err(ClientError::Reply)?;
                on_delta(&delta);
            }
        }
        if matches!(
            provider.resolved_api(),
            ProviderApi::Responses | ProviderApi::Anthropic
        ) && !decoder.completed()
        {
            return Err(ClientError::Reply(ReplyError::IncompleteResponse));
        }
        if answer.text().is_empty() {
            return Err(ClientError::InvalidReply);
        }
        Ok(answer.into_string())
    } else {
        let body = provider_content(&raw, provider)?;
        let mut bounded = AnswerBuffer::default();
        bounded.append(&body).map_err(ClientError::Reply)?;
        on_delta(bounded.text());
        Ok(bounded.into_string())
    }
}

fn provider_content(body: &[u8], provider: &ProviderProfile) -> Result<String, ClientError> {
    if provider.resolved_api() != ProviderApi::Anthropic {
        return completion_content(body, provider.uses_responses());
    }
    let value: Value = serde_json::from_slice(body).map_err(|_| ClientError::InvalidReply)?;
    if value["type"] != "message" || value["role"] != "assistant" {
        return Err(ClientError::InvalidReply);
    }
    if value["stop_reason"] == "max_tokens" {
        return Err(ClientError::Reply(ReplyError::IncompleteResponse));
    }
    let text: String = value["content"]
        .as_array()
        .ok_or(ClientError::InvalidReply)?
        .iter()
        .filter(|part| part["type"] == "text")
        .filter_map(|part| part["text"].as_str())
        .collect();
    if text.is_empty() {
        Err(ClientError::InvalidReply)
    } else {
        Ok(text)
    }
}

fn completion_content(body: &[u8], responses: bool) -> Result<String, ClientError> {
    let value: Value = serde_json::from_slice(body).map_err(|_| ClientError::InvalidReply)?;
    if responses {
        if value["status"].as_str() != Some("completed") {
            return Err(ClientError::Reply(ReplyError::IncompleteResponse));
        }
        let mut text = String::new();
        for item in value["output"]
            .as_array()
            .ok_or(ClientError::InvalidReply)?
        {
            if item["type"] != "message" || item["role"] != "assistant" {
                continue;
            }
            for part in item["content"]
                .as_array()
                .ok_or(ClientError::InvalidReply)?
            {
                if part["type"] == "refusal" {
                    return Err(ClientError::Reply(ReplyError::Refused));
                }
                if part["type"] == "output_text" {
                    text.push_str(part["text"].as_str().ok_or(ClientError::InvalidReply)?);
                }
            }
        }
        return if text.is_empty() {
            Err(ClientError::InvalidReply)
        } else {
            Ok(text)
        };
    }
    let choice = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .ok_or(ClientError::InvalidReply)?;
    flatten_content(
        choice
            .get("message")
            .and_then(|message| message.get("content"))
            .or_else(|| choice.get("text")),
    )
    .filter(|content| !content.is_empty())
    .ok_or(ClientError::InvalidReply)
}

#[cfg(test)]
mod tests {
    #[test]
    fn responses_routes_and_bodies_preserve_history_without_chat_only_fields() {
        use super::super::provider::ProviderApi;
        let provider = ProviderProfile {
            endpoint: "https://api.openai.com/v1".into(),
            api: ProviderApi::Responses,
            ..ProviderProfile::default()
        };
        assert_eq!(
            provider_url(&provider).unwrap(),
            "https://api.openai.com/v1/responses"
        );
        let body = completion_body(
            &provider,
            ("system instructions", "next"),
            &[
                (false, "previous question".into()),
                (true, "previous answer".into()),
            ],
            true,
        );
        assert_eq!(body["instructions"], "system instructions");
        assert_eq!(body["input"][1]["role"], "assistant");
        assert_eq!(body["input"][2]["content"], "next");
        assert_eq!(body["store"], false);
        assert!(body.get("messages").is_none());
        let automatic = ProviderProfile {
            endpoint: "https://api.openai.com/v1/responses".into(),
            ..ProviderProfile::default()
        };
        assert!(automatic.uses_responses());
        assert_eq!(
            models_url(&automatic.endpoint).unwrap(),
            "https://api.openai.com/v1/models"
        );
        let chat = ProviderProfile {
            api: ProviderApi::ChatCompletions,
            ..automatic
        };
        assert_eq!(
            provider_url(&chat).unwrap(),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn responses_stream_and_json_read_only_output_text_and_require_completion() {
        let provider = ProviderProfile {
            endpoint: "https://example.test/v1/responses".into(),
            ..ProviderProfile::default()
        };
        let success = r#"{"status":"completed","output":[{"type":"reasoning","summary":[]},{"type":"message","role":"assistant","content":[{"type":"output_text","text":"OK"}]}]}"#;
        assert_eq!(completion_content(success.as_bytes(), true).unwrap(), "OK");
        assert!(completion_content(br#"{"status":"incomplete","output":[]}"#, true).is_err());
        let transport = FakeTransport {
            status: 200,
            body: success.as_bytes().to_vec(),
        };
        test_connection(&transport, &provider, None).unwrap();
        assert_eq!(
            stream_completion(
                &transport,
                &provider,
                None,
                ("s", "u"),
                &[],
                &AtomicBool::new(false),
                &mut |_| {}
            )
            .unwrap(),
            "OK"
        );
        let (endpoint, server) = local_provider_server(200, "text/event-stream", b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"OK\"}\n\ndata: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\"}}\n\n".to_vec());
        let provider = ProviderProfile {
            endpoint,
            api: super::super::provider::ProviderApi::Responses,
            ..ProviderProfile::default()
        };
        let mut text = String::new();
        assert_eq!(
            stream_completion(
                &super::super::transport::ReqwestTransport,
                &provider,
                None,
                ("s", "u"),
                &[],
                &AtomicBool::new(false),
                &mut |delta| text.push_str(delta)
            )
            .unwrap(),
            "OK"
        );
        assert_eq!(text, "OK");
        let request = server.join().unwrap();
        assert!(request.contains("POST /v1/responses"));
        assert!(request.contains("\"store\":false"));
        let truncated = FakeTransport {
            status: 200,
            body: b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"partial\"}\n\n"
                .to_vec(),
        };
        let provider = ProviderProfile {
            endpoint: "https://example.test/v1/responses".into(),
            ..provider
        };
        assert_eq!(
            stream_completion(
                &truncated,
                &provider,
                None,
                ("s", "u"),
                &[],
                &AtomicBool::new(false),
                &mut |_| {}
            ),
            Err(ClientError::Reply(ReplyError::IncompleteResponse))
        );
    }
    struct Scripted {
        replies: std::sync::Mutex<std::collections::VecDeque<(u16, &'static str)>>,
        calls: std::sync::atomic::AtomicUsize,
    }

    impl HttpTransport for Scripted {
        fn execute(&self, request: Request) -> Result<Response, TransportError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            let (status, body) = self
                .replies
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected retry");
            if body == "nonstream" {
                assert_eq!(request.body.unwrap()["stream"], false);
                return Ok(Response {
                    status,
                    body: br#"{"choices":[{"message":{"content":"OK"}}]}"#.to_vec(),
                    content_type: None,
                });
            }
            Ok(Response {
                status,
                body: body.as_bytes().to_vec(),
                content_type: None,
            })
        }
    }

    fn scripted(replies: Vec<(u16, &'static str)>) -> Scripted {
        Scripted {
            replies: std::sync::Mutex::new(replies.into()),
            calls: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    #[test]
    fn transient_retry_auth_quota_and_stream_fallback_are_classified() {
        let provider = ProviderProfile::default();
        let cancel = AtomicBool::new(false);
        let transport = scripted(vec![(503, "{}"), (200, "nonstream")]);
        // A transient retry remains streaming; supply ordinary JSON on success.
        transport.replies.lock().unwrap()[1].1 = r#"{"choices":[{"message":{"content":"OK"}}]}"#;
        assert_eq!(
            stream_completion(
                &transport,
                &provider,
                None,
                ("system", "user"),
                &[],
                &cancel,
                &mut |_| {}
            )
            .unwrap(),
            "OK"
        );
        assert_eq!(transport.calls.load(Ordering::Relaxed), 2);
        for (status, body, expected) in [
            (401, "{}", ClientError::Http(401)),
            (
                429,
                r#"{"error":{"code":"insufficient_quota"}}"#,
                ClientError::Quota,
            ),
        ] {
            let transport = scripted(vec![(status, body)]);
            assert_eq!(
                stream_completion(
                    &transport,
                    &provider,
                    None,
                    ("system", "user"),
                    &[],
                    &cancel,
                    &mut |_| {}
                ),
                Err(expected)
            );
            assert_eq!(transport.calls.load(Ordering::Relaxed), 1);
        }
        let transport = scripted(vec![
            (
                400,
                r#"{"error":{"code":"unsupported_value","param":"stream"}}"#,
            ),
            (200, "nonstream"),
        ]);
        assert_eq!(
            stream_completion(
                &transport,
                &provider,
                None,
                ("system", "user"),
                &[],
                &cancel,
                &mut |_| {}
            )
            .unwrap(),
            "OK"
        );
    }

    #[test]
    fn configured_fallback_uses_its_own_provider_key_and_stops_after_partial_output() {
        let second = ProviderProfile {
            id: "second".into(),
            model: "fallback".into(),
            ..ProviderProfile::default()
        };
        let providers = [ProviderProfile::default(), second];
        let transport = scripted(vec![
            (429, r#"{"error":{"code":"insufficient_quota"}}"#),
            (
                200,
                r#"{"choices":[{"message":{"content":"fallback OK"}}]}"#,
            ),
        ]);
        let mut loaded = Vec::new();
        let answer = complete_with_fallback(
            &transport,
            &providers,
            &mut |p| {
                loaded.push(p.id.clone());
                Ok(None)
            },
            ("s", "u"),
            &[],
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(answer, "fallback OK");
        assert_eq!(loaded, ["default-openrouter", "second"]);

        struct Partial(std::sync::atomic::AtomicUsize);
        impl HttpTransport for Partial {
            fn execute(&self, _: Request) -> Result<Response, TransportError> {
                unreachable!()
            }
            fn execute_stream(
                &self,
                _: Request,
                _: &AtomicBool,
                head: &mut dyn FnMut(ResponseHead),
                chunk: &mut dyn FnMut(&[u8]) -> Result<(), TransportError>,
            ) -> Result<(), TransportError> {
                self.0.fetch_add(1, Ordering::Relaxed);
                head(ResponseHead {
                    status: 200,
                    content_type: Some("text/event-stream".into()),
                    retry_after: None,
                });
                chunk(b"data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n")?;
                Err(TransportError::Network)
            }
        }
        let partial = Partial(std::sync::atomic::AtomicUsize::new(0));
        let mut text = String::new();
        assert!(complete_with_fallback(
            &partial,
            &providers,
            &mut |_| Ok(None),
            ("s", "u"),
            &[],
            &AtomicBool::new(false),
            &mut |delta| text.push_str(delta)
        )
        .is_err());
        assert_eq!(partial.0.load(Ordering::Relaxed), 1);
        assert_eq!(text, "partial");
    }

    #[test]
    fn retry_after_and_cancellation_do_not_start_another_request() {
        struct Limited;
        impl HttpTransport for Limited {
            fn execute(&self, _: Request) -> Result<Response, TransportError> {
                unreachable!()
            }
            fn execute_stream(
                &self,
                _: Request,
                _: &AtomicBool,
                head: &mut dyn FnMut(ResponseHead),
                _: &mut dyn FnMut(&[u8]) -> Result<(), TransportError>,
            ) -> Result<(), TransportError> {
                head(ResponseHead {
                    status: 429,
                    content_type: None,
                    retry_after: Some(Duration::from_secs(120)),
                });
                Ok(())
            }
        }
        assert_eq!(
            stream_completion(
                &Limited,
                &ProviderProfile::default(),
                None,
                ("s", "u"),
                &[],
                &AtomicBool::new(false),
                &mut |_| {}
            ),
            Err(ClientError::Http(429))
        );
        let transport = scripted(vec![]);
        assert_eq!(
            stream_completion(
                &transport,
                &ProviderProfile::default(),
                None,
                ("s", "u"),
                &[],
                &AtomicBool::new(true),
                &mut |_| {}
            ),
            Err(ClientError::Cancelled)
        );
    }

    #[test]
    fn real_transport_cancels_while_waiting_for_provider_headers() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let endpoint = format!(
            "http://{}/v1/chat/completions",
            listener.local_addr().unwrap()
        );
        let (accepted_tx, accepted_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            accepted_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            drop(stream);
        });
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let worker = thread::spawn(move || {
            stream_completion(
                &super::super::transport::ReqwestTransport,
                &ProviderProfile {
                    endpoint,
                    ..ProviderProfile::default()
                },
                None,
                ("s", "u"),
                &[],
                &worker_cancel,
                &mut |_| {},
            )
        });
        accepted_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        let started = Instant::now();
        cancel.store(true, Ordering::Relaxed);
        assert_eq!(worker.join().unwrap(), Err(ClientError::Cancelled));
        assert!(started.elapsed() < Duration::from_millis(500));
        release_tx.send(()).unwrap();
        server.join().unwrap();
    }

    fn configured_provider_two() -> (super::super::provider::ProviderProfile, Zeroizing<String>) {
        use super::super::credentials::{CredentialStore, SystemCredentialStore};
        assert_eq!(
            std::env::var("BUTTONSCLI_TEST_PROVIDER_TWO").as_deref(),
            Ok("1")
        );
        let root = home::home_dir().unwrap().join(".buttonscli-native");
        let active: Value =
            serde_json::from_slice(&std::fs::read(root.join("active-profile.json")).unwrap())
                .unwrap();
        let profile =
            crate::storage::paths::sanitize_profile_name(active["name"].as_str().unwrap()).unwrap();
        let document: Value = serde_json::from_slice(
            &std::fs::read(root.join("profiles").join(&profile).join("native.json")).unwrap(),
        )
        .unwrap();
        let settings: super::super::provider::ProviderSettings =
            serde_json::from_value(document["preferences"]["provider_settings"].clone()).unwrap();
        let provider = settings
            .providers
            .get(1)
            .expect("Provider 2 is missing")
            .clone();
        let expected = super::super::credentials::reference(&profile, &provider.id);
        assert_eq!(provider.credential_ref.as_deref(), Some(expected.as_str()));
        let key = SystemCredentialStore
            .get(&expected)
            .expect("Provider 2 key is unavailable");
        (provider, key)
    }

    #[test]
    #[ignore = "explicit real-provider model discovery; no completion request"]
    fn configured_provider_two_live_models() {
        let (provider, key) = configured_provider_two();
        let models = super::discover_models(
            &super::super::transport::ReqwestTransport,
            &provider,
            Some(key),
        )
        .expect("Provider 2 model discovery failed");
        println!(
            "Provider 2 models: {} returned; configured model present: {}",
            models.len(),
            models.contains(&provider.model)
        );
        assert!(models.contains(&provider.model));
    }

    #[test]
    #[ignore = "explicit Mistral codestral-latest request; API key supplied only in process memory"]
    fn mistral_codestral_live_ai_flows() {
        let key = Zeroizing::new(
            std::env::var("BUTTONSCLI_TEST_MISTRAL_KEY").expect("Supply the test key explicitly"),
        );
        let provider = super::super::provider::ProviderProfile {
            id: "mistral-codestral".into(),
            name: "Mistral".into(),
            endpoint: "https://api.mistral.ai/v1/chat/completions".into(),
            model: "codestral-latest".into(),
            credential_ref: None,
            models_endpoint: None,
            api: Default::default(),
        };
        let transport = super::super::transport::ReqwestTransport;
        super::test_connection(&transport, &provider, Some(Zeroizing::new(key.to_string())))
            .expect("Mistral codestral-latest minimal completion failed");
        println!("Mistral codestral-latest minimal completion passed.");
        let cancelled = std::sync::atomic::AtomicBool::new(false);
        let mut received_delta = false;
        let answer = super::stream_completion(
            &transport,
            &provider,
            Some(Zeroizing::new(key.to_string())),
            ("You are a concise assistant.", "Reply with OK."),
            &[],
            &cancelled,
            &mut |_| {
                received_delta = true;
            },
        )
        .expect("Mistral codestral-latest AI Help streaming failed");
        assert!(!answer.trim().is_empty() && received_delta);
        println!("Mistral codestral-latest AI Help streaming passed without terminal context.");
        let themes = crate::theme::ThemeCatalog::load();
        let base = crate::theme_files::document_from_theme(themes.get("aurora"), "Provider test");
        let seed = crate::theme_generation::seed_palette(&base);
        let candidate = crate::theme_generation::generate_candidate(
            &transport,
            &provider,
            Some(key),
            "A readable dark navy terminal with teal accents.",
            &seed,
            &base,
            &cancelled,
        )
        .expect("Mistral codestral-latest AI theme generation or native validation failed");
        crate::theme::validate_personal_document(&candidate.document).unwrap();
        println!("Mistral codestral-latest AI theme generation and native validation passed; no theme saved.");
    }
    #[test]
    #[ignore = "explicit real-provider request using the active native profile's Provider 2"]
    fn configured_provider_two_live_connection() {
        use super::super::credentials::{CredentialStore, SystemCredentialStore};
        assert_eq!(
            std::env::var("BUTTONSCLI_TEST_PROVIDER_TWO").as_deref(),
            Ok("1")
        );
        let root = home::home_dir().unwrap().join(".buttonscli-native");
        let active: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("active-profile.json")).unwrap())
                .unwrap();
        let profile =
            crate::storage::paths::sanitize_profile_name(active["name"].as_str().unwrap()).unwrap();
        let document: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("profiles").join(&profile).join("native.json")).unwrap(),
        )
        .unwrap();
        let settings: super::super::provider::ProviderSettings =
            serde_json::from_value(document["preferences"]["provider_settings"].clone()).unwrap();
        let provider = settings.providers.get(1).expect("Provider 2 is missing");
        let expected = super::super::credentials::reference(&profile, &provider.id);
        assert_eq!(provider.credential_ref.as_deref(), Some(expected.as_str()));
        let key = SystemCredentialStore
            .get(&expected)
            .expect("Provider 2 key is unavailable");
        let result = super::test_connection(
            &super::super::transport::ReqwestTransport,
            provider,
            Some(Zeroizing::new(key.to_string())),
        );
        if result.is_err() {
            // A credential-free TLS probe diagnoses transport failures without exposing keys.
            if let Err(error) = reqwest::blocking::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap()
                .get(&provider.endpoint)
                .send()
            {
                println!(
                    "Credential-free transport diagnostic: {:?}",
                    error.without_url()
                );
            }
        }
        result.expect("Provider 2 minimal connection request failed");
        println!(
            "Provider 2: saved credential and configured model passed minimal completion request."
        );
        let cancelled = std::sync::atomic::AtomicBool::new(false);
        let mut received_delta = false;
        let answer = super::stream_completion(
            &super::super::transport::ReqwestTransport,
            provider,
            Some(Zeroizing::new(key.to_string())),
            ("You are a concise assistant.", "Reply with OK."),
            &[],
            &cancelled,
            &mut |_| {
                received_delta = true;
            },
        )
        .expect("Provider 2 streamed AI Help request failed");
        assert!(!answer.trim().is_empty());
        assert!(received_delta);
        println!(
            "Provider 2: AI Help streaming passed with synthetic text and no terminal context."
        );
        let themes = crate::theme::ThemeCatalog::load();
        let base = crate::theme_files::document_from_theme(themes.get("aurora"), "Provider test");
        let seed = crate::theme_generation::seed_palette(&base);
        let candidate = crate::theme_generation::generate_candidate(
            &super::super::transport::ReqwestTransport,
            provider,
            Some(key),
            "A readable dark navy terminal with teal accents.",
            &seed,
            &base,
            &cancelled,
        )
        .expect("Provider 2 AI theme generation or contrast validation failed");
        crate::theme::validate_personal_document(&candidate.document).unwrap();
        println!("Provider 2: AI theme candidate generation and native validation passed; no theme was saved.");
    }
    #[test]
    fn base_urls_expand_and_preserve_custom_prefixes() {
        for (input, expected) in [
            (
                "https://example.test",
                "https://example.test/v1/chat/completions",
            ),
            (
                "https://example.test/v1/",
                "https://example.test/v1/chat/completions",
            ),
            (
                "https://example.test/api/v1",
                "https://example.test/api/v1/chat/completions",
            ),
            (
                "https://example.test/custom/chat",
                "https://example.test/custom/chat/chat/completions",
            ),
        ] {
            assert_eq!(super::completion_url(input).unwrap(), expected);
        }
        assert_eq!(
            super::models_url("https://example.test").unwrap(),
            "https://example.test/v1/models"
        );
        assert!(super::completion_url("https://example.test?api_key=secret").is_err());
    }
    use super::super::transport::{Response, TransportError};
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    struct FakeTransport {
        status: u16,
        body: Vec<u8>,
    }
    impl HttpTransport for FakeTransport {
        fn execute(&self, request: Request) -> Result<Response, TransportError> {
            assert!(request.url.starts_with("https://example.test/"));
            Ok(Response {
                status: self.status,
                body: self.body.clone(),
                content_type: Some("application/json".into()),
            })
        }
    }

    fn local_provider_server(
        status: u16,
        content_type: &'static str,
        body: Vec<u8>,
    ) -> (String, JoinHandle<String>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let request = read_http_request(&mut stream);
            let reason = if (200..300).contains(&status) {
                "OK"
            } else {
                "Error"
            };
            let redirect = if status == 302 {
                "location: http://127.0.0.1:9/redirected\r\n"
            } else {
                ""
            };
            write!(
                stream,
                "HTTP/1.1 {status} {reason}\r\n{redirect}content-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            for chunk in body.chunks(13) {
                stream.write_all(chunk).unwrap();
                stream.flush().unwrap();
                thread::sleep(Duration::from_millis(2));
            }
            String::from_utf8_lossy(&request).into_owned()
        });
        (format!("http://{address}/v1/chat/completions"), handle)
    }

    fn read_http_request(stream: &mut TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        loop {
            let mut chunk = [0_u8; 2048];
            let read = stream.read(&mut chunk).unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
            let Some(header_end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") else {
                continue;
            };
            let header = String::from_utf8_lossy(&request[..header_end]).to_ascii_lowercase();
            let content_length = header
                .lines()
                .find_map(|line| {
                    line.strip_prefix("content-length:")
                        .and_then(|length| length.trim().parse::<usize>().ok())
                })
                .unwrap_or(0);
            if request.len() >= header_end + 4 + content_length {
                break;
            }
        }
        request
    }

    #[test]
    fn model_urls_match_original_suffix_rules() {
        for (endpoint, expected) in [
            (
                "https://example.test/v1/chat/completions",
                "https://example.test/v1/models",
            ),
            (
                "https://example.test/v1/completions",
                "https://example.test/v1/models",
            ),
            (
                "https://example.test/v1/responses",
                "https://example.test/v1/models",
            ),
            ("https://example.test/v1", "https://example.test/v1/models"),
            (
                "https://example.test/v1/models",
                "https://example.test/v1/models",
            ),
        ] {
            assert_eq!(models_url(endpoint).unwrap(), expected);
        }
        assert!(models_url("https://user:key@example.test/v1").is_err());
    }

    #[test]
    fn discovery_and_connection_validate_real_status_and_body_without_echoing_secrets() {
        let provider = ProviderProfile {
            endpoint: "https://example.test/v1/chat/completions".into(),
            model: "manual-model".into(),
            ..ProviderProfile::default()
        };
        let transport = FakeTransport {
            status: 200,
            body: br#"{"data":[{"id":"z"},{"id":"a"}],"models":["a","b"]}"#.to_vec(),
        };
        assert_eq!(
            discover_models(&transport, &provider, None).unwrap(),
            vec!["a", "b", "z"]
        );
        let bad = FakeTransport {
            status: 401,
            body: b"FAKE-SECRET-KEY".to_vec(),
        };
        let error = test_connection(
            &bad,
            &provider,
            Some(Zeroizing::new("FAKE-SECRET-KEY".into())),
        )
        .unwrap_err();
        assert_eq!(error, ClientError::Http(401));
        assert!(!error.to_string().contains("FAKE-SECRET-KEY"));
        for status in [429, 500] {
            assert_eq!(
                test_connection(
                    &FakeTransport {
                        status,
                        body: Vec::new()
                    },
                    &provider,
                    None
                ),
                Err(ClientError::Http(status))
            );
        }
        let success = FakeTransport {
            status: 200,
            body: br#"{"choices":[{"message":{"content":"OK"}}]}"#.to_vec(),
        };
        test_connection(&success, &provider, None).unwrap();
        let invalid = FakeTransport {
            status: 200,
            body: b"{}".to_vec(),
        };
        assert_eq!(
            test_connection(&invalid, &provider, None),
            Err(ClientError::InvalidReply)
        );
    }

    #[test]
    fn reqwest_transport_completes_loopback_provider_check_discovery_and_streaming() {
        let transport = super::super::transport::ReqwestTransport;
        let (endpoint, server) = local_provider_server(
            200,
            "application/json",
            br#"{"choices":[{"message":{"content":"OK"}}]}"#.to_vec(),
        );
        let provider = ProviderProfile {
            endpoint: endpoint.clone(),
            model: "offline-test-model".into(),
            ..ProviderProfile::default()
        };
        test_connection(
            &transport,
            &provider,
            Some(Zeroizing::new("FAKE-LOCAL-KEY".into())),
        )
        .unwrap();
        let request = server.join().unwrap().to_ascii_lowercase();
        assert!(request.starts_with("post /v1/chat/completions "));
        assert!(request.contains("authorization: bearer fake-local-key"));
        assert!(request.contains("offline-test-model"));

        let (endpoint, server) =
            local_provider_server(302, "application/json", b"FAKE-REDIRECT-BODY".to_vec());
        let provider = ProviderProfile {
            endpoint,
            model: "offline-test-model".into(),
            ..ProviderProfile::default()
        };
        let error = test_connection(&transport, &provider, None).unwrap_err();
        assert_eq!(error, ClientError::Http(302));
        assert!(!error.to_string().contains("FAKE-REDIRECT-BODY"));
        assert!(server
            .join()
            .unwrap()
            .starts_with("POST /v1/chat/completions "));

        let (endpoint, server) = local_provider_server(
            200,
            "application/json",
            br#"{"data":[{"id":"z-model"},{"id":"a-model"}]}"#.to_vec(),
        );
        let provider = ProviderProfile {
            endpoint,
            model: "offline-test-model".into(),
            ..ProviderProfile::default()
        };
        assert_eq!(
            discover_models(&transport, &provider, None).unwrap(),
            vec!["a-model", "z-model"]
        );
        assert!(server.join().unwrap().starts_with("GET /v1/models "));

        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"hello \"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"from local\"}}]}\n\n",
            "data: [DONE]\n\n"
        );
        let (endpoint, server) =
            local_provider_server(200, "text/event-stream", body.as_bytes().to_vec());
        let provider = ProviderProfile {
            endpoint,
            model: "offline-test-model".into(),
            ..ProviderProfile::default()
        };
        let mut deltas = Vec::new();
        let answer = stream_completion(
            &transport,
            &provider,
            None,
            ("You are a test assistant.", "Say hello."),
            &[],
            &AtomicBool::new(false),
            &mut |delta| deltas.push(delta.to_owned()),
        )
        .unwrap();
        assert_eq!(answer, "hello from local");
        assert_eq!(deltas.concat(), answer);
        assert!(server.join().unwrap().contains("\"stream\":true"));
    }
}

#[cfg(test)]
mod protocol_tests {
    use super::super::credentials::{CredentialStore, SystemCredentialStore};
    use super::*;

    struct Fixture(std::process::Child);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[test]
    fn all_three_protocols_use_real_http_headers_routes_json_and_streams() {
        use std::io::BufRead;
        let mut fixture = Fixture(
            std::process::Command::new("node")
                .arg(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("tests/fixtures/protocol-provider.mjs"),
                )
                .stdout(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let mut line = String::new();
        std::io::BufReader::new(fixture.0.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let ready: Value = serde_json::from_str(&line).unwrap();
        let base = ready["base"].as_str().unwrap();
        for (prefix, api, expected) in [
            ("chat", ProviderApi::ChatCompletions, "Chat ✓"),
            ("responses", ProviderApi::Responses, "Responses ✓"),
            ("anthropic", ProviderApi::Anthropic, "Messages ✓"),
        ] {
            let provider = ProviderProfile {
                endpoint: format!("{base}/{prefix}/v1"),
                api,
                model: "fixture-chat".into(),
                ..Default::default()
            };
            let key = || Some(Zeroizing::new("fixture-key".into()));
            assert_eq!(
                discover_models(&super::super::transport::ReqwestTransport, &provider, key())
                    .unwrap()
                    .len(),
                3
            );
            test_connection(&super::super::transport::ReqwestTransport, &provider, key()).unwrap();
            let mut streamed = String::new();
            let answer = stream_completion(
                &super::super::transport::ReqwestTransport,
                &provider,
                key(),
                ("system", "user"),
                &[],
                &AtomicBool::new(false),
                &mut |delta| streamed.push_str(delta),
            )
            .unwrap();
            assert_eq!(answer, expected);
            assert_eq!(streamed, expected);
        }
        let mut auto = ProviderProfile {
            endpoint: format!("{base}/anthropic/v1"),
            api: ProviderApi::Auto,
            model: "fixture-claude".into(),
            ..Default::default()
        };
        assert_eq!(
            detect_format(
                &super::super::transport::ReqwestTransport,
                &auto,
                Some(Zeroizing::new("fixture-key".into()))
            )
            .unwrap(),
            ProviderApi::Anthropic
        );
        auto.endpoint = format!("{base}/discover");
        auto.api = ProviderApi::ChatCompletions;
        assert_eq!(
            discover_models(
                &super::super::transport::ReqwestTransport,
                &auto,
                Some(Zeroizing::new("fixture-key".into()))
            )
            .unwrap()
            .len(),
            3
        );
        let error = http_error(
            400,
            br#"{"error":{"message":"bad fixture-key"}}"#,
            Some("fixture-key"),
        )
        .to_string();
        assert!(!error.contains("fixture-key"));
    }

    #[test]
    #[ignore = "explicit user-authorized OpenAI-det key; read-only configuration and short live requests"]
    fn openai_det_live_chat_responses_and_models() {
        let (root, legacy) = crate::storage::paths::production_roots().unwrap();
        let store = crate::storage::store::NativeStore::open(root, legacy.0).unwrap();
        let loaded = store.load().unwrap().unwrap();
        let original = loaded
            .preferences
            .provider_settings
            .providers
            .iter()
            .find(|provider| provider.name == "OpenAI-det")
            .unwrap()
            .clone();
        let expected = super::super::credentials::reference(store.profile_name(), &original.id);
        assert_eq!(original.credential_ref.as_deref(), Some(expected.as_str()));
        let key = SystemCredentialStore.get(&expected).unwrap();
        let transport = super::super::transport::ReqwestTransport;
        let models = discover_models(&transport, &original, Some(key.clone())).unwrap();
        assert!(models.iter().any(|id| id == "gpt-4.1-mini"));
        let mut chat = original.clone();
        chat.api = ProviderApi::ChatCompletions;
        chat.model = "gpt-4.1-mini".into();
        test_connection(&transport, &chat, Some(key.clone())).unwrap();
        let mut streamed = String::new();
        let answer = stream_completion(
            &transport,
            &chat,
            Some(key.clone()),
            ("Reply briefly.", "Reply OK."),
            &[],
            &AtomicBool::new(false),
            &mut |delta| streamed.push_str(delta),
        )
        .unwrap();
        assert!(!answer.is_empty());
        assert_eq!(answer, streamed);
        test_connection(&transport, &original, Some(key)).unwrap();
        println!("OpenAI-det: models list, GPT-4.1-mini Chat Completions JSON/stream, and existing Responses model passed. No saved configuration changed.");
    }

    #[test]
    #[ignore = "explicit configured-provider model discovery only; reads saved OS credentials without writes"]
    fn configured_provider_model_lists_live() {
        let (root, legacy) = crate::storage::paths::production_roots().unwrap();
        let store = crate::storage::store::NativeStore::open(root, legacy.0).unwrap();
        let loaded = store.load().unwrap().unwrap();
        for provider in loaded.preferences.provider_settings.providers {
            if provider.endpoint.is_empty() {
                println!("{}: missing base URL", provider.name);
                continue;
            }
            let expected = super::super::credentials::reference(store.profile_name(), &provider.id);
            assert_eq!(provider.credential_ref.as_deref(), Some(expected.as_str()));
            let key = SystemCredentialStore.get(&expected).unwrap();
            match discover_models(
                &super::super::transport::ReqwestTransport,
                &provider,
                Some(key),
            ) {
                Ok(models) => println!(
                    "{}: {} models; configured model listed: {}",
                    provider.name,
                    models.len(),
                    models.contains(&provider.model)
                ),
                Err(error) => println!("{}: {}", provider.name, error),
            }
        }
    }

    #[test]
    #[ignore = "configured-provider short model requests; explicit diagnosis with saved credentials, no writes"]
    fn configured_provider_short_requests_live() {
        let (root, legacy) = crate::storage::paths::production_roots().unwrap();
        let store = crate::storage::store::NativeStore::open(root, legacy.0).unwrap();
        let loaded = store.load().unwrap().unwrap();
        std::thread::scope(|scope| {
            for provider in loaded.preferences.provider_settings.providers {
                if provider.endpoint.is_empty() {
                    continue;
                }
                let expected =
                    super::super::credentials::reference(store.profile_name(), &provider.id);
                assert_eq!(provider.credential_ref.as_deref(), Some(expected.as_str()));
                let key = SystemCredentialStore.get(&expected).unwrap();
                scope.spawn(move || {
                    let result = test_connection(
                        &super::super::transport::ReqwestTransport,
                        &provider,
                        Some(key),
                    );
                    println!(
                        "{}: {}",
                        provider.name,
                        result.map_or_else(
                            |error| error.to_string(),
                            |_| "model request passed".into()
                        )
                    );
                });
            }
        });
    }
}
