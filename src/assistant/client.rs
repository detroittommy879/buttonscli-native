//! Provider connection checks and model discovery. Responses are bounded by transport.

use std::cell::Cell;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{json, Value};
use zeroize::Zeroizing;

use super::provider::{validate_endpoint, ProviderProfile};
use super::reply::{flatten_content, AnswerBuffer, ReplyError, SseDecoder, StreamItem};
use super::transport::{HttpTransport, Method, Request, ResponseHead, TransportError};

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ClientError {
    Endpoint,
    Model,
    Http(u16),
    Transport(TransportError),
    InvalidReply,
    Reply(ReplyError),
    Cancelled,
    RequestTooLarge,
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Endpoint => f.write_str("configure a valid chat completions endpoint"),
            Self::Model => f.write_str("enter a model ID"),
            Self::Http(429) => f.write_str("provider returned HTTP 429 (rate limit or quota); wait or check the provider account"),
            Self::Http(status) => write!(f, "provider returned HTTP {status}"),
            Self::Transport(error) => write!(f, "{error}"),
            Self::InvalidReply => f.write_str("provider returned an invalid response"),
            Self::Reply(error) => write!(f, "assistant response could not be read: {error:?}"),
            Self::Cancelled => f.write_str("provider request was cancelled"),
            Self::RequestTooLarge => f.write_str("provider request exceeded 1 MiB"),
        }
    }
}

pub(crate) fn completion_url(endpoint: &str) -> Result<String, ClientError> {
    validate_endpoint(endpoint).map_err(|_| ClientError::Endpoint)?;
    let mut url = url::Url::parse(endpoint).map_err(|_| ClientError::Endpoint)?;
    let path = url.path().trim_end_matches('/');
    if path.is_empty() {
        url.set_path("/v1/chat/completions");
    } else if path.ends_with("/v1") {
        let path = format!("{path}/chat/completions");
        url.set_path(&path);
    }
    Ok(url.to_string())
}

pub(crate) fn models_url(endpoint: &str) -> Result<String, ClientError> {
    let endpoint = completion_url(endpoint)?;
    let mut url = url::Url::parse(&endpoint).map_err(|_| ClientError::Endpoint)?;
    let mut segments: Vec<String> = url
        .path_segments()
        .ok_or(ClientError::Endpoint)?
        .filter(|segment| !segment.is_empty())
        .map(str::to_owned)
        .collect();
    if segments.ends_with(&["chat".into(), "completions".into()]) {
        segments.truncate(segments.len() - 2);
    } else if matches!(
        segments.last().map(String::as_str),
        Some("completions" | "responses")
    ) {
        segments.pop();
    } else if !matches!(segments.last().map(String::as_str), Some("models")) {
        if let Some(index) = segments.iter().rposition(|segment| segment == "v1") {
            segments.truncate(index + 1);
        }
    }
    if segments.last().is_none_or(|segment| segment != "models") {
        segments.push("models".into());
    }
    {
        let mut path = url.path_segments_mut().map_err(|_| ClientError::Endpoint)?;
        path.clear();
        for segment in segments {
            path.push(&segment);
        }
    }
    Ok(url.to_string())
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
                if let Some(id) = id.map(str::trim).filter(|id| !id.is_empty()) {
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
    let url = models_url(&provider.endpoint)?;
    let response = transport
        .execute(Request {
            method: Method::Get,
            url,
            body: None,
            key,
        })
        .map_err(ClientError::Transport)?;
    if !(200..300).contains(&response.status) {
        return Err(ClientError::Http(response.status));
    }
    collect_models(&response.body)
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
    let response = transport.execute(Request {
        method: Method::Post,
        url: completion_url(&provider.endpoint)?,
        body: Some(json!({"model": provider.model, "messages": [{"role": "user", "content": "Reply OK."}], "max_tokens": 4, "stream": false})),
        key,
    }).map_err(ClientError::Transport)?;
    if !(200..300).contains(&response.status) {
        return Err(ClientError::Http(response.status));
    }
    let value: Value =
        serde_json::from_slice(&response.body).map_err(|_| ClientError::InvalidReply)?;
    if value["choices"]
        .as_array()
        .is_some_and(|choices| !choices.is_empty())
    {
        Ok(())
    } else {
        Err(ClientError::InvalidReply)
    }
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
    let (system_prompt, prompt) = prompts;
    validate_endpoint(&provider.endpoint).map_err(|_| ClientError::Endpoint)?;
    if provider.model.trim().is_empty() {
        return Err(ClientError::Model);
    }
    let status = Cell::new(0_u16);
    let is_sse = Cell::new(false);
    let mut raw = Vec::new();
    let mut decoder = SseDecoder::default();
    let mut answer = AnswerBuffer::default();
    let mut parser_error = None;
    let mut on_head = |head: ResponseHead| {
        status.set(head.status);
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
    let mut messages = Vec::with_capacity(history.len() + 2);
    messages.push(json!({"role": "system", "content": system_prompt}));
    messages.extend(history.iter().map(|(assistant, content)| {
        json!({
            "role": if *assistant { "assistant" } else { "user" },
            "content": content,
        })
    }));
    messages.push(json!({"role": "user", "content": prompt}));
    let request_body = json!({"model": provider.model, "messages": messages, "stream": true});
    if serde_json::to_vec(&request_body)
        .map_err(|_| ClientError::InvalidReply)?
        .len()
        > 1024 * 1024
    {
        return Err(ClientError::RequestTooLarge);
    }
    let request = Request {
        method: Method::Post,
        url: completion_url(&provider.endpoint)?,
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
        return Err(ClientError::Http(status.get()));
    }
    if is_sse.get() {
        for item in decoder.finish().map_err(ClientError::Reply)? {
            if let StreamItem::Delta(delta) = item {
                answer.append(&delta).map_err(ClientError::Reply)?;
                on_delta(&delta);
            }
        }
        if answer.text().is_empty() {
            return Err(ClientError::InvalidReply);
        }
        Ok(answer.into_string())
    } else {
        let body = completion_content(&raw)?;
        let mut bounded = AnswerBuffer::default();
        bounded.append(&body).map_err(ClientError::Reply)?;
        on_delta(bounded.text());
        Ok(bounded.into_string())
    }
}

fn completion_content(body: &[u8]) -> Result<String, ClientError> {
    let value: Value = serde_json::from_slice(body).map_err(|_| ClientError::InvalidReply)?;
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
    fn base_urls_expand_without_changing_custom_completion_paths() {
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
                "https://example.test/custom/chat",
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
