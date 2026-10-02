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
}

pub(crate) struct Response {
    pub(crate) status: u16,
    pub(crate) body: Vec<u8>,
    pub(crate) content_type: Option<String>,
}

pub(crate) struct ResponseHead {
    pub(crate) status: u16,
    pub(crate) content_type: Option<String>,
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

impl HttpTransport for ReqwestTransport {
    fn execute(&self, request: Request) -> Result<Response, TransportError> {
        let parsed = url::Url::parse(&request.url).map_err(|_| TransportError::InvalidEndpoint)?;
        if !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(TransportError::InvalidEndpoint);
        }
        let client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|_| TransportError::Network)?;
        let mut builder = match request.method {
            Method::Get => client.get(parsed),
            Method::Post => client.post(parsed),
        }
        .header("Accept", "application/json")
        .header("X-Title", "ButtonsCLI Native");
        if let Some(key) = request.key.as_deref() {
            builder = builder.header(reqwest::header::AUTHORIZATION, bearer_header(key)?);
        }
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
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(TransportError::InvalidEndpoint);
        }
        let client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|_| TransportError::Network)?;
        let mut builder = match request.method {
            Method::Get => client.get(parsed),
            Method::Post => client.post(parsed),
        }
        .header("Accept", "application/json, text/event-stream")
        .header("X-Title", "ButtonsCLI Native");
        if let Some(key) = request.key.as_deref() {
            builder = builder.header(reqwest::header::AUTHORIZATION, bearer_header(key)?);
        }
        if let Some(body) = request.body {
            builder = builder.json(&body);
        }
        let mut response = builder.send().map_err(|_| TransportError::Network)?;
        if response
            .content_length()
            .is_some_and(|length| length > MAX_REPLY)
        {
            return Err(TransportError::ResponseTooLarge);
        }
        let head = ResponseHead {
            status: response.status().as_u16(),
            content_type: response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
        };
        on_head(head);
        let mut total = 0_u64;
        let mut buffer = [0_u8; 8192];
        loop {
            if cancelled.load(Ordering::Relaxed) {
                return Err(TransportError::Cancelled);
            }
            let read = response
                .read(&mut buffer)
                .map_err(|_| TransportError::Network)?;
            if read == 0 {
                break;
            }
            total += read as u64;
            if total > MAX_REPLY {
                return Err(TransportError::ResponseTooLarge);
            }
            on_chunk(&buffer[..read])?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
