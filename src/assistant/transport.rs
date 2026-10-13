//! Bounded, redirect-free HTTP for explicitly configured assistant providers.

use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use zeroize::Zeroizing;

const MAX_REPLY: u64 = 1024 * 1024;

#[derive(Clone, Copy)]
pub(crate) enum Method {
    Get,
    Post,
}

pub(crate) struct Request {
    pub(crate) method: Method,
    pub(crate) url: String,
    pub(crate) body: Option<serde_json::Value>,
    pub(crate) key: Option<Zeroizing<String>>,
    pub(crate) anthropic: bool,
}

pub(crate) struct Response {
    pub(crate) status: u16,
    pub(crate) body: Vec<u8>,
    pub(crate) content_type: Option<String>,
}

pub(crate) struct ResponseHead {
    pub(crate) status: u16,
    pub(crate) content_type: Option<String>,
    pub(crate) retry_after: Option<Duration>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum TransportError {
    InvalidEndpoint,
    InvalidCredential,
    Network,
    ResponseTooLarge,
    Cancelled,
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidEndpoint => "invalid provider endpoint",
            Self::InvalidCredential => {
                "API key contains invalid characters; paste only the key and save it again"
            }
            Self::Network => "provider connection failed or timed out",
            Self::ResponseTooLarge => "provider response exceeded 1 MiB",
            Self::Cancelled => "provider request was cancelled",
        })
    }
}

pub(crate) trait HttpTransport: Send + Sync {
    fn execute(&self, request: Request) -> Result<Response, TransportError>;

    fn execute_stream(
        &self,
        request: Request,
        cancelled: &AtomicBool,
        on_head: &mut dyn FnMut(ResponseHead),
        on_chunk: &mut dyn FnMut(&[u8]) -> Result<(), TransportError>,
    ) -> Result<(), TransportError> {
        let response = self.execute(request)?;
        on_head(ResponseHead {
            status: response.status,
            content_type: response.content_type,
            retry_after: None,
        });
        for chunk in response.body.chunks(8192) {
            if cancelled.load(Ordering::Relaxed) {
                return Err(TransportError::Cancelled);
            }
            on_chunk(chunk)?;
        }
        Ok(())
    }
}

pub(crate) struct ReqwestTransport;

fn bearer_header(key: &str) -> Result<reqwest::header::HeaderValue, TransportError> {
    let key = key.trim();
    if key.is_empty() || key.chars().any(char::is_control) {
        return Err(TransportError::InvalidCredential);
    }
    let mut value = reqwest::header::HeaderValue::from_str(&format!("Bearer {key}"))
        .map_err(|_| TransportError::InvalidCredential)?;
    value.set_sensitive(true);
    Ok(value)
}

fn auth_headers(request: &Request) -> Result<reqwest::header::HeaderMap, TransportError> {
    let mut headers = reqwest::header::HeaderMap::new();
    if request.anthropic {
        headers.insert(
            "anthropic-version",
            reqwest::header::HeaderValue::from_static("2023-06-01"),
        );
    }
    if let Some(key) = request.key.as_deref() {
        if request.anthropic {
            let key = key.trim();
            if key.is_empty() || key.chars().any(char::is_control) {
                return Err(TransportError::InvalidCredential);
            }
            let mut value = reqwest::header::HeaderValue::from_str(key)
                .map_err(|_| TransportError::InvalidCredential)?;
            value.set_sensitive(true);
            headers.insert("x-api-key", value);
        } else {
            headers.insert(reqwest::header::AUTHORIZATION, bearer_header(key)?);
        }
    }
    Ok(headers)
}

fn valid_query(request: &Request, url: &url::Url) -> bool {
    url.query().is_none()
        || (matches!(request.method, Method::Get)
            && url.path().ends_with("/models")
            && url.query_pairs().all(|(key, value)| {
                matches!(key.as_ref(), "limit" | "after_id") && value.len() <= 256
            }))
}

impl HttpTransport for ReqwestTransport {
    fn execute(&self, request: Request) -> Result<Response, TransportError> {
        let parsed = url::Url::parse(&request.url).map_err(|_| TransportError::InvalidEndpoint)?;
        if !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || !valid_query(&request, &parsed)
            || parsed.fragment().is_some()
        {
            return Err(TransportError::InvalidEndpoint);
        }
        let client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|_| TransportError::Network)?;
        let mut builder = match request.method {
            Method::Get => client.get(parsed),
            Method::Post => client.post(parsed),
        }
        .header("Accept", "application/json")
        .header("X-Title", "ButtonsCLI Native");
        builder = builder.headers(auth_headers(&request)?);
        if let Some(body) = request.body {
            builder = builder.json(&body);
        }
        let response = builder.send().map_err(|_| TransportError::Network)?;
        let status = response.status().as_u16();
        if response
            .content_length()
            .is_some_and(|length| length > MAX_REPLY)
        {
            return Err(TransportError::ResponseTooLarge);
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let mut body = Vec::new();
        response
            .take(MAX_REPLY + 1)
            .read_to_end(&mut body)
            .map_err(|_| TransportError::Network)?;
        if body.len() as u64 > MAX_REPLY {
            return Err(TransportError::ResponseTooLarge);
        }
        Ok(Response {
            status,
            body,
            content_type,
        })
    }

    fn execute_stream(
        &self,
        request: Request,
        cancelled: &AtomicBool,
        on_head: &mut dyn FnMut(ResponseHead),
        on_chunk: &mut dyn FnMut(&[u8]) -> Result<(), TransportError>,
    ) -> Result<(), TransportError> {
        let parsed = url::Url::parse(&request.url).map_err(|_| TransportError::InvalidEndpoint)?;
        if !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || !valid_query(&request, &parsed)
            || parsed.fragment().is_some()
        {
            return Err(TransportError::InvalidEndpoint);
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| TransportError::Network)?;
        let _runtime_context = runtime.enter();
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|_| TransportError::Network)?;
        let mut builder = match request.method {
            Method::Get => client.get(parsed),
            Method::Post => client.post(parsed),
        }
        .header("Accept", "application/json, text/event-stream")
        .header("X-Title", "ButtonsCLI Native");
        builder = builder.headers(auth_headers(&request)?);
        if let Some(body) = request.body {
            builder = builder.json(&body);
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        let mut response = cancellable(&runtime, cancelled, deadline, builder.send())?;
        if response
            .content_length()
            .is_some_and(|length| length > MAX_REPLY)
        {
            return Err(TransportError::ResponseTooLarge);
        }
        let head = ResponseHead {
            status: response.status().as_u16(),
            retry_after: response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(retry_after),
            content_type: response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
        };
        on_head(head);
        let mut total = 0_u64;
        loop {
            if cancelled.load(Ordering::Relaxed) {
                return Err(TransportError::Cancelled);
            }
            let Some(chunk) = cancellable(&runtime, cancelled, deadline, response.chunk())? else {
                break;
            };
            total += chunk.len() as u64;
            if total > MAX_REPLY {
                return Err(TransportError::ResponseTooLarge);
            }
            on_chunk(&chunk)?;
        }
        Ok(())
    }
}

fn cancellable<T>(
    runtime: &tokio::runtime::Runtime,
    cancelled: &AtomicBool,
    deadline: std::time::Instant,
    future: impl std::future::Future<Output = Result<T, reqwest::Error>>,
) -> Result<T, TransportError> {
    runtime.block_on(async {
        let mut future = std::pin::pin!(future);
        loop {
            if cancelled.load(Ordering::Relaxed) {
                return Err(TransportError::Cancelled);
            }
            if std::time::Instant::now() >= deadline {
                return Err(TransportError::Network);
            }
            match tokio::time::timeout(Duration::from_millis(25), future.as_mut()).await {
                Ok(result) => return result.map_err(|_| TransportError::Network),
                Err(_) => continue,
            }
        }
    })
}

fn retry_after(value: &str) -> Option<Duration> {
    value
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
        .or_else(|| {
            httpdate::parse_http_date(value).ok().map(|date| {
                date.duration_since(std::time::SystemTime::now())
                    .unwrap_or_default()
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_headers_use_only_the_expected_auth_and_model_query_is_bounded() {
        let mut request = Request {
            method: Method::Get,
            url: "https://api.anthropic.com/v1/models?limit=500".into(),
            body: None,
            key: Some(Zeroizing::new(" fixture-key ".into())),
            anthropic: true,
        };
        let headers = auth_headers(&request).unwrap();
        assert_eq!(headers["x-api-key"], "fixture-key");
        assert!(headers["x-api-key"].is_sensitive());
        assert_eq!(headers["anthropic-version"], "2023-06-01");
        assert!(!headers.contains_key(reqwest::header::AUTHORIZATION));
        assert!(valid_query(
            &request,
            &url::Url::parse(&request.url).unwrap()
        ));
        assert!(!valid_query(
            &request,
            &url::Url::parse("https://example.com/v1/models?api_key=secret").unwrap()
        ));
        request.anthropic = false;
        let headers = auth_headers(&request).unwrap();
        assert_eq!(
            headers[reqwest::header::AUTHORIZATION],
            "Bearer fixture-key"
        );
        assert!(!headers.contains_key("x-api-key"));
        request.method = Method::Post;
        assert!(!valid_query(
            &request,
            &url::Url::parse(&request.url).unwrap()
        ));
    }

    #[test]
    fn pasted_key_whitespace_is_trimmed_and_embedded_controls_are_rejected() {
        let value = bearer_header("  FAKE-KEY\r\n ").unwrap();
        assert_eq!(value.to_str().unwrap(), "Bearer FAKE-KEY");
        assert!(value.is_sensitive());
        for invalid in ["\r\n ", "FAKE\nKEY", "FAKE\0KEY"] {
            assert_eq!(
                bearer_header(invalid).unwrap_err(),
                TransportError::InvalidCredential
            );
        }
    }
}
