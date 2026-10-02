//! Native client for broad, server-controlled feature rollout flags.
//!
//! Defaults and failures are closed. The runtime config contains no account or
//! machine identity and is fetched off the GUI thread.

use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, RwLock};

use serde::Deserialize;

use crate::assistant::transport::{HttpTransport, Method, Request, ReqwestTransport};
use crate::features::access::RuntimeAccess;

const RUNTIME_CONFIG_URL: &str = "https://buttonscli.com/bcli-metrics/api/v1/runtime-config";
const HTTP: ReqwestTransport = ReqwestTransport;
// The native replacement service is not deployed. Never fall back to the old
// metrics host until its replacement contract and rollout are reviewed.
pub(crate) const LEGACY_METRICS_ENABLED: bool = false;
static CONFIG_REFRESH_STARTED: AtomicBool = AtomicBool::new(false);
static CACHED_RUNTIME_CONFIG: OnceLock<RwLock<RuntimeConfig>> = OnceLock::new();

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct RuntimeConfig {
    pub(crate) all_free: bool,
    pub(crate) pro_enabled: bool,
    pub(crate) updated_at: Option<String>,
}

impl RuntimeConfig {
    pub(crate) fn access(&self) -> RuntimeAccess {
        RuntimeAccess {
            all_free: self.all_free,
            pro_enabled: self.pro_enabled,
            ..RuntimeAccess::default()
        }
    }
}

fn runtime_config_cache() -> &'static RwLock<RuntimeConfig> {
    CACHED_RUNTIME_CONFIG.get_or_init(|| RwLock::new(RuntimeConfig::default()))
}

pub(crate) fn current_runtime_config() -> RuntimeConfig {
    runtime_config_cache()
        .read()
        .map(|config| config.clone())
        .unwrap_or_default()
}

/// Fetch broad rollout flags without delaying GUI or terminal startup.
pub(crate) fn refresh_runtime_config(context: egui::Context) {
    if !LEGACY_METRICS_ENABLED
        || std::env::var("BUTTONSCLI_NATIVE_DISABLE_REMOTE_CONFIG").is_ok_and(|value| value == "1")
        || CONFIG_REFRESH_STARTED.swap(true, Ordering::AcqRel)
    {
        return;
    }
    let result = std::thread::Builder::new()
        .name("buttonscli-runtime-config".into())
        .spawn(move || loop {
            let config = RuntimeConfigClient::production()
                .and_then(|client| client.fetch())
                .unwrap_or_default();
            if let Ok(mut current) = runtime_config_cache().write() {
                *current = config;
            }
            context.request_repaint();
            std::thread::sleep(std::time::Duration::from_secs(300));
        });
    if result.is_err() {
        CONFIG_REFRESH_STARTED.store(false, Ordering::Release);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeConfigError {
    Disabled,
    Endpoint,
    Network,
    Http(u16),
    ResponseTooLarge,
    InvalidReply,
}

impl std::fmt::Display for RuntimeConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disabled => {
                formatter.write_str("hosted runtime config is disabled pending the native service")
            }
            Self::Endpoint => formatter.write_str("runtime config endpoint is invalid"),
            Self::Network => formatter.write_str("could not reach ButtonsCLI runtime config"),
            Self::Http(status) => write!(formatter, "runtime config returned HTTP {status}"),
            Self::ResponseTooLarge => formatter.write_str("runtime config response exceeded 1 MiB"),
            Self::InvalidReply => formatter.write_str("runtime config response is invalid"),
        }
    }
}

pub(crate) struct RuntimeConfigClient<'a> {
    endpoint: url::Url,
    transport: &'a dyn HttpTransport,
}

impl RuntimeConfigClient<'static> {
    pub(crate) fn production() -> Result<Self, RuntimeConfigError> {
        if !LEGACY_METRICS_ENABLED {
            return Err(RuntimeConfigError::Disabled);
        }
        Self::new(RUNTIME_CONFIG_URL, &HTTP)
    }
}

impl<'a> RuntimeConfigClient<'a> {
    fn new(endpoint: &str, transport: &'a dyn HttpTransport) -> Result<Self, RuntimeConfigError> {
        Ok(Self {
            endpoint: parse_service_url(endpoint)?,
            transport,
        })
    }

    pub(crate) fn fetch(&self) -> Result<RuntimeConfig, RuntimeConfigError> {
        let response = self
            .transport
            .execute(Request {
                method: Method::Get,
                url: self.endpoint.to_string(),
                body: None,
                key: None,
            })
            .map_err(|error| match error {
                crate::assistant::transport::TransportError::InvalidEndpoint => {
                    RuntimeConfigError::Endpoint
                }
                crate::assistant::transport::TransportError::ResponseTooLarge => {
                    RuntimeConfigError::ResponseTooLarge
                }
                crate::assistant::transport::TransportError::Network
                | crate::assistant::transport::TransportError::InvalidCredential
                | crate::assistant::transport::TransportError::Cancelled => {
                    RuntimeConfigError::Network
                }
            })?;
        if !(200..300).contains(&response.status) {
            return Err(RuntimeConfigError::Http(response.status));
        }
        let payload: RuntimeConfigPayload =
            serde_json::from_slice(&response.body).map_err(|_| RuntimeConfigError::InvalidReply)?;
        Ok(RuntimeConfig {
            all_free: payload.all_free.unwrap_or(false),
            pro_enabled: payload.pro_enabled.unwrap_or(false),
            updated_at: payload.updated_at.filter(|value| value.len() <= 64),
        })
    }
}

fn parse_service_url(value: &str) -> Result<url::Url, RuntimeConfigError> {
    let url = url::Url::parse(value).map_err(|_| RuntimeConfigError::Endpoint)?;
    let local_http = url.scheme() == "http"
        && (url
            .host_str()
            .is_some_and(|host| host.eq_ignore_ascii_case("localhost"))
            || url.host().and_then(|host| match host {
                url::Host::Ipv4(address) => Some(IpAddr::V4(address).is_loopback()),
                url::Host::Ipv6(address) => Some(IpAddr::V6(address).is_loopback()),
                url::Host::Domain(_) => None,
            }) == Some(true));
    if (url.scheme() != "https" && !local_http)
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(RuntimeConfigError::Endpoint);
    }
    Ok(url)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeConfigPayload {
    all_free: Option<bool>,
    pro_enabled: Option<bool>,
    updated_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::assistant::transport::{Response, TransportError};

    #[derive(Default)]
    struct MockTransport {
        response: Mutex<Option<Response>>,
        url: Mutex<Option<String>>,
    }

    impl MockTransport {
        fn json(body: &str, status: u16) -> Self {
            Self {
                response: Mutex::new(Some(Response {
                    status,
                    body: body.as_bytes().to_vec(),
                    content_type: Some("application/json".into()),
                })),
                url: Mutex::new(None),
            }
        }
    }

    impl HttpTransport for MockTransport {
        fn execute(&self, request: Request) -> Result<Response, TransportError> {
            assert!(matches!(request.method, Method::Get));
            assert!(request.body.is_none());
            assert!(request.key.is_none());
            *self.url.lock().unwrap() = Some(request.url);
            self.response
                .lock()
                .unwrap()
                .take()
                .ok_or(TransportError::Network)
        }
    }

    fn client<'a>(transport: &'a dyn HttpTransport) -> RuntimeConfigClient<'a> {
        RuntimeConfigClient::new("http://127.0.0.1:38173/api/v1/runtime-config", transport).unwrap()
    }

    #[test]
    fn old_metrics_runtime_config_is_disabled_before_constructing_a_client() {
        assert!(matches!(
            RuntimeConfigClient::production(),
            Err(RuntimeConfigError::Disabled)
        ));
    }

    #[test]
    fn runtime_config_defaults_missing_flags_to_closed_and_honors_valid_payload() {
        let missing = MockTransport::json("{}", 200);
        assert_eq!(client(&missing).fetch().unwrap(), RuntimeConfig::default());

        let transport = MockTransport::json(
            r#"{"allFree":true,"proEnabled":true,"updatedAt":"2026-09-29T00:00:00Z"}"#,
            200,
        );
        let config = client(&transport).fetch().unwrap();
        assert!(config.all_free && config.pro_enabled);
        assert_eq!(config.updated_at.as_deref(), Some("2026-09-29T00:00:00Z"));
        assert!(config.access().all_free && config.access().pro_enabled);
        assert_eq!(
            *transport.url.lock().unwrap(),
            Some("http://127.0.0.1:38173/api/v1/runtime-config".into())
        );
    }

    #[test]
    fn errors_malformed_payload_and_cleartext_remote_endpoint_fail_closed() {
        let server_error = MockTransport::json("{}", 503);
        assert_eq!(
            client(&server_error).fetch().unwrap_err(),
            RuntimeConfigError::Http(503)
        );
        let invalid_json = MockTransport::json("not json", 200);
        assert_eq!(
            client(&invalid_json).fetch().unwrap_err(),
            RuntimeConfigError::InvalidReply
        );
        assert_eq!(
            RuntimeConfigClient::new("http://config.example.test/api", &server_error).err(),
            Some(RuntimeConfigError::Endpoint)
        );
        assert_eq!(
            RuntimeConfigClient::new("https://user:password@example.test/config", &server_error)
                .err(),
            Some(RuntimeConfigError::Endpoint)
        );
    }
}
