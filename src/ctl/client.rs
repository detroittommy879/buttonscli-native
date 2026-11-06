//! Finds the running native instance and talks to its loopback control API.
//!
//! A deliberately small HTTP/1.1 client over `std::net::TcpStream`: the API
//! only ever listens on `127.0.0.1`, so there is no TLS, proxy or redirect to
//! handle, and the helper binary stays small and starts fast.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;

/// Environment variable naming a descriptor file or a control directory.
pub const INFO_PATH_ENV: &str = "BUTTONSCLI_CONTROL_INFO_PATH";
const LIVE_CHECK_TIMEOUT: Duration = Duration::from_millis(750);
/// Largest response accepted (reads return at most 200,000 characters).
const MAX_RESPONSE_BYTES: usize = 32 * 1024 * 1024;

/// A validated connection descriptor. The token never leaves this struct
/// except in the `Authorization` header.
pub struct Connection {
    pub info_path: PathBuf,
    pub base_url: String,
    pub instance_id: String,
    pub updated_at_ms: f64,
    port: u16,
    token: String,
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("info_path", &self.info_path)
            .field("base_url", &self.base_url)
            .field("instance_id", &self.instance_id)
            .finish_non_exhaustive()
    }
}

/// `~/.buttonscli-native/control`.
pub fn default_control_dir() -> Result<PathBuf, String> {
    home::home_dir()
        .map(|home| home.join(".buttonscli-native").join("control"))
        .ok_or_else(|| "Could not find the home folder.".to_owned())
}

/// The descriptor named by `BUTTONSCLI_CONTROL_INFO_PATH` (a file, or a
/// directory to search), else the one live instance in the default directory.
pub fn connect() -> Result<Connection, String> {
    match std::env::var_os(INFO_PATH_ENV).filter(|value| !value.is_empty()) {
        Some(path) => {
            let path = PathBuf::from(path);
            if path.is_dir() {
                discover(&path)
            } else {
                read_descriptor(&path)
            }
        }
        None => discover(&default_control_dir()?),
    }
}

/// Proceeds only when exactly one descriptor in `dir` belongs to a live instance.
pub fn discover(dir: &Path) -> Result<Connection, String> {
    let mut live = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_json = path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));
            if !is_json {
                continue;
            }
            if let Ok(connection) = read_descriptor(&path) {
                if is_live(&connection) {
                    live.push(connection);
                }
            }
        }
    }
    match live.len() {
        1 => Ok(live.remove(0)),
        0 => Err(format!(
            "No running native ButtonsCLI instance was found in {}. Start ButtonsCLI or set {INFO_PATH_ENV} to its exact connection file.",
            dir.display()
        )),
        _ => Err(format!(
            "More than one native ButtonsCLI instance is running. Set {INFO_PATH_ENV} to the exact instance file under {}.",
            dir.display()
        )),
    }
}

fn is_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Reads and validates one descriptor, with the same rules as the Node helpers.
pub fn read_descriptor(path: &Path) -> Result<Connection, String> {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let raw = std::fs::read_to_string(&absolute).map_err(|error| {
        format!(
            "Could not read native ButtonsCLI connection info at {}. {error}",
            absolute.display()
        )
    })?;
    let invalid = || format!("Invalid native control descriptor: {}", absolute.display());
    let parsed: Value = serde_json::from_str(&raw).map_err(|_| invalid())?;
    let text = |key: &str| parsed.get(key).and_then(Value::as_str).unwrap_or_default();
    let instance_id = text("instanceId");
    let token = text("authToken");
    let file_name = absolute
        .file_name()
        .map(|name| name.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let endpoint = url::Url::parse(text("baseUrl")).map_err(|_| invalid())?;
    let updated_at_ms = parsed.get("updatedAtMs").and_then(Value::as_f64);
    let valid = parsed.get("schemaVersion").and_then(Value::as_u64) == Some(1)
        && is_hex(instance_id, 32)
        && file_name == format!("{instance_id}.json")
        && is_hex(token, 64)
        && endpoint.scheme() == "http"
        && endpoint.host_str() == Some("127.0.0.1")
        && endpoint.port().is_some()
        && endpoint.username().is_empty()
        && endpoint.password().is_none()
        && endpoint.path() == "/"
        && endpoint.query().is_none()
        && endpoint.fragment().is_none()
        && parsed.get("capabilities").is_some_and(Value::is_array)
        && updated_at_ms.is_some_and(f64::is_finite);
    if !valid {
        return Err(invalid());
    }
    let port = endpoint.port().unwrap_or_default();
    Ok(Connection {
        info_path: absolute,
        base_url: format!("http://127.0.0.1:{port}"),
        instance_id: instance_id.to_owned(),
        updated_at_ms: updated_at_ms.unwrap_or_default(),
        port,
        token: token.to_owned(),
    })
}

fn is_live(connection: &Connection) -> bool {
    matches!(
        connection.request("GET", "/v1/status", None, LIVE_CHECK_TIMEOUT),
        Ok(status) if status.get("instanceId").and_then(Value::as_str)
            == Some(connection.instance_id.as_str())
    )
}

/// Default deadline for a request: 10 s, plus the server's own limits for
/// paced delivery (30 s) and `run` waits (up to 60 s).
pub fn request_timeout(path: &str, body: Option<&Value>) -> Duration {
    let field = |key: &str| body.and_then(|body| body.get(key));
    let delivery = if field("delivery").and_then(Value::as_str) == Some("slow-typed") {
        30_000
    } else {
        0
    };
    let wait = if path.starts_with("/v1/tabs/") && path.ends_with("/run") {
        field("maxWaitMs")
            .and_then(Value::as_f64)
            .unwrap_or(8_000.0)
            .clamp(0.0, 60_000.0) as u64
    } else {
        0
    };
    Duration::from_millis(10_000 + delivery + wait)
}

/// Percent-encodes a path segment like JavaScript's `encodeURIComponent`.
pub fn encode_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// A JSON number the server's integer fields accept: whole values are sent
/// without a fraction (`120`, not `120.0`).
pub fn number(value: f64) -> Value {
    if value.fract() == 0.0 && value >= 0.0 && value <= u64::MAX as f64 {
        Value::from(value as u64)
    } else {
        Value::from(value)
    }
}

enum Failure {
    Timeout,
    Connect(String),
}

impl Connection {
    /// Sends one request and returns the JSON body. A timed-out POST is never
    /// repeated: its input may already have reached the terminal.
    pub fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        timeout: Duration,
    ) -> Result<Value, String> {
        let (status, text) = match self.exchange(method, path, body, timeout) {
            Ok(response) => response,
            Err(Failure::Timeout) => {
                let advice = if method == "POST" {
                    " Delivery may have begun; check the target before repeating the request."
                } else {
                    ""
                };
                return Err(format!("Control API request timed out.{advice}"));
            }
            Err(Failure::Connect(detail)) => return Err(self.connection_failure(&detail)),
        };
        let payload = if text.trim().is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text)
                .map_err(|_| format!("Control API returned invalid JSON (HTTP {status})."))?
        };
        if !(200..300).contains(&status) {
            return Err(payload
                .get("error")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .unwrap_or_else(|| format!("Control API returned {status}.")));
        }
        Ok(payload)
    }

    /// `request` with the default deadline for that route.
    pub fn call(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Value, String> {
        self.request(method, path, body, request_timeout(path, body))
    }

    fn connection_failure(&self, detail: &str) -> String {
        let mut message = format!(
            "Failed to reach ButtonsCLI control API at {}. Connection info: {}.",
            self.base_url,
            self.info_path.display()
        );
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| since.as_millis() as f64)
            .unwrap_or_default();
        if self.updated_at_ms > 0.0 && now_ms >= self.updated_at_ms {
            let minutes = ((now_ms - self.updated_at_ms) / 60_000.0).round();
            message.push_str(&format!(" Recorded {minutes} minute(s) ago."));
        }
        message.push_str(
            " The local connection file is probably stale or ButtonsCLI is not currently running. Start or restart ButtonsCLI so it rewrites the control API listener info.",
        );
        if !detail.is_empty() {
            message.push_str(&format!(" Error: {detail}."));
        }
        message
    }

    fn exchange(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        timeout: Duration,
    ) -> Result<(u16, String), Failure> {
        let deadline = Instant::now() + timeout.max(Duration::from_millis(1));
        let remaining = || {
            deadline
                .checked_duration_since(Instant::now())
                .filter(|left| !left.is_zero())
                .ok_or(Failure::Timeout)
        };
        let timed_out = |error: &std::io::Error| {
            matches!(
                error.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            )
        };
        let address = SocketAddr::from(([127, 0, 0, 1], self.port));
        let mut stream = TcpStream::connect_timeout(&address, remaining()?).map_err(|error| {
            if timed_out(&error) {
                Failure::Timeout
            } else {
                Failure::Connect(error.to_string())
            }
        })?;
        let _ = stream.set_nodelay(true);
        let body = body.map(Value::to_string).unwrap_or_default();
        let mut request = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\nAccept: application/json\r\nConnection: close\r\n",
            port = self.port,
            token = self.token,
        );
        if method != "GET" || !body.is_empty() {
            request.push_str(&format!(
                "Content-Type: application/json\r\nContent-Length: {}\r\n",
                body.len()
            ));
        }
        request.push_str("\r\n");
        request.push_str(&body);
        let io = |error: std::io::Error| {
            if timed_out(&error) {
                Failure::Timeout
            } else {
                Failure::Connect(error.to_string())
            }
        };
        stream.set_write_timeout(Some(remaining()?)).map_err(io)?;
        stream.write_all(request.as_bytes()).map_err(io)?;
        let mut response = Vec::new();
        let mut chunk = [0_u8; 16 * 1024];
        loop {
            stream.set_read_timeout(Some(remaining()?)).map_err(io)?;
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => {
                    response.extend_from_slice(&chunk[..read]);
                    if response.len() > MAX_RESPONSE_BYTES {
                        return Err(Failure::Connect("response too large".into()));
                    }
                    if response_complete(&response) {
                        break;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(io(error)),
            }
        }
        parse_response(&response).map_err(Failure::Connect)
    }
}

fn split_head(raw: &[u8]) -> Option<(&[u8], &[u8])> {
    raw.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|end| (&raw[..end], &raw[end + 4..]))
}

fn header<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    head.lines().skip(1).find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.trim()
            .eq_ignore_ascii_case(name)
            .then_some(value.trim())
    })
}

/// True once a `Content-Length` body has fully arrived, so we need not wait
/// for the server to close the connection.
fn response_complete(raw: &[u8]) -> bool {
    let Some((head, body)) = split_head(raw) else {
        return false;
    };
    let head = String::from_utf8_lossy(head);
    header(&head, "content-length")
        .and_then(|len| len.parse::<usize>().ok())
        .is_some_and(|len| body.len() >= len)
}

/// Status code and UTF-8 body of a raw HTTP/1.1 response (fixed-length,
/// chunked or read-to-close).
pub(crate) fn parse_response(raw: &[u8]) -> Result<(u16, String), String> {
    let (head, body) = split_head(raw).ok_or("incomplete HTTP response")?;
    let head = String::from_utf8_lossy(head);
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or("malformed HTTP status line")?;
    let body = if header(&head, "transfer-encoding")
        .is_some_and(|value| value.to_ascii_lowercase().contains("chunked"))
    {
        decode_chunked(body)?
    } else if let Some(len) = header(&head, "content-length").and_then(|len| len.parse().ok()) {
        body.get(..len).ok_or("truncated HTTP body")?.to_vec()
    } else {
        body.to_vec()
    };
    String::from_utf8(body)
        .map(|text| (status, text))
        .map_err(|_| "HTTP body is not UTF-8".to_owned())
}

fn decode_chunked(mut body: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    loop {
        let line_end = body
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or("truncated chunked body")?;
        let size_text = String::from_utf8_lossy(&body[..line_end]);
        let size_text = size_text.split(';').next().unwrap_or_default().trim();
        let size = usize::from_str_radix(size_text, 16).map_err(|_| "bad chunk size")?;
        body = &body[line_end + 2..];
        if size == 0 {
            return Ok(out);
        }
        out.extend_from_slice(body.get(..size).ok_or("truncated chunk")?);
        body = body.get(size + 2..).ok_or("truncated chunk")?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fixed_chunked_and_close_delimited_bodies() {
        let fixed = b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\n\r\n{\"a\":1}";
        assert_eq!(parse_response(fixed).unwrap(), (200, "{\"a\":1}".into()));
        let chunked =
            b"HTTP/1.1 404 Not Found\r\ntransfer-encoding: chunked\r\n\r\n4\r\n{\"a\"\r\n3;x=y\r\n:1}\r\n0\r\n\r\n";
        assert_eq!(parse_response(chunked).unwrap(), (404, "{\"a\":1}".into()));
        let closed = b"HTTP/1.1 500 Oops\r\n\r\nplain";
        assert_eq!(parse_response(closed).unwrap(), (500, "plain".into()));
        assert!(parse_response(b"HTTP/1.1 200").is_err());
        assert!(response_complete(fixed));
        assert!(!response_complete(&fixed[..fixed.len() - 1]));
    }

    #[test]
    fn encodes_like_encode_uri_component_and_sends_integers() {
        assert_eq!(encode_component("tab-3"), "tab-3");
        assert_eq!(encode_component("my tab/é?"), "my%20tab%2F%C3%A9%3F");
        assert_eq!(number(120.0).to_string(), "120");
        assert_eq!(number(1.5).to_string(), "1.5");
    }

    #[test]
    fn route_deadlines_follow_server_limits() {
        assert_eq!(request_timeout("/v1/status", None), Duration::from_secs(10));
        let paced = serde_json::json!({"delivery": "slow-typed"});
        assert_eq!(
            request_timeout("/v1/tabs/active/send", Some(&paced)),
            Duration::from_secs(40)
        );
        let run = serde_json::json!({"maxWaitMs": 999_999});
        assert_eq!(
            request_timeout("/v1/tabs/active/run", Some(&run)),
            Duration::from_secs(70)
        );
        assert_eq!(
            request_timeout("/v1/tabs/active/run", None),
            Duration::from_secs(18)
        );
    }

    #[test]
    fn descriptors_are_validated_like_the_node_helpers() {
        let dir = std::env::temp_dir().join(format!("buttonsclictl-desc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let id = "0123456789abcdef0123456789abcdef";
        let good = serde_json::json!({
            "schemaVersion": 1,
            "instanceId": id,
            "authToken": "a".repeat(64),
            "baseUrl": "http://127.0.0.1:4567",
            "capabilities": [],
            "updatedAtMs": 1,
        });
        let path = dir.join(format!("{id}.json"));
        std::fs::write(&path, good.to_string()).unwrap();
        let connection = read_descriptor(&path).unwrap();
        assert_eq!(connection.base_url, "http://127.0.0.1:4567");
        assert!(
            !format!("{connection:?}").contains(&"a".repeat(64)),
            "token hidden"
        );
        for (key, value) in [
            ("baseUrl", serde_json::json!("http://localhost:4567")),
            ("baseUrl", serde_json::json!("http://127.0.0.1:4567/x")),
            ("authToken", serde_json::json!("short")),
            ("schemaVersion", serde_json::json!(2)),
        ] {
            let mut bad = good.clone();
            bad[key] = value;
            std::fs::write(&path, bad.to_string()).unwrap();
            assert!(read_descriptor(&path).is_err(), "{key}");
        }
        let renamed = dir.join("other.json");
        std::fs::write(&renamed, good.to_string()).unwrap();
        assert!(
            read_descriptor(&renamed).is_err(),
            "file name must match the id"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
