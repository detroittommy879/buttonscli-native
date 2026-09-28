//! Provider connection checks and model discovery. Responses are bounded by transport.

use std::collections::BTreeSet;

use serde_json::{json, Value};
use zeroize::Zeroizing;

use super::provider::{validate_endpoint, ProviderProfile};
use super::transport::{HttpTransport, Method, Request, TransportError};

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ClientError {
    Endpoint,
    Model,
    Http(u16),
    Transport(TransportError),
    InvalidReply,
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Endpoint => f.write_str("configure a valid chat completions endpoint"),
            Self::Model => f.write_str("enter a model ID"),
            Self::Http(status) => write!(f, "provider returned HTTP {status}"),
            Self::Transport(error) => write!(f, "{error}"),
            Self::InvalidReply => f.write_str("provider returned an invalid response"),
        }
    }
}

pub(crate) fn models_url(endpoint: &str) -> Result<String, ClientError> {
    validate_endpoint(endpoint).map_err(|_| ClientError::Endpoint)?;
    let mut url = url::Url::parse(endpoint).map_err(|_| ClientError::Endpoint)?;
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
        url: provider.endpoint.clone(),
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

#[cfg(test)]
mod tests {
    use super::super::transport::{Response, TransportError};
    use super::*;

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
            })
        }
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
}
