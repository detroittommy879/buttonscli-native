//! Sign in with ChatGPT: run AI requests on the person's ChatGPT plan.
//!
//! OAuth authorization code + PKCE through the system browser and a one-shot
//! `127.0.0.1` callback. OpenAI registers this installation during the first
//! sign-in (`dynamic_agent_client`), so there is no client secret. Requests
//! then use the Responses API at `api.openai.com/v1` with the access token as
//! the bearer key, exactly like an API key.
//!
//! The ID token arrives straight from the token endpoint over TLS, so its
//! claims are checked (issuer, audience, nonce, expiry, subject) without a
//! signature check, as OpenID Connect Core 3.1.3.7 allows for that channel.
//!
//! Tokens live in the OS credential store, split into pieces: Windows caps one
//! secret at 1,280 UTF-16 characters and access tokens can be longer.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use super::credentials::{reference, CredentialError, CredentialStore};

const ISSUER: &str = "https://auth.openai.com";
pub(crate) const RESOURCE: &str = super::provider::CHATGPT_RESOURCE;
const SCOPES: &str =
    "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";
const SHARING_SCOPE: &str = "chatgpt.tokens.use.direct";
const CALLBACK_PATH: &str = "/auth/callback";
const DYNAMIC_CLIENT: &str = "dynamic_agent_client";
/// Where people review and limit what apps use from their plan.
pub(crate) const USAGE_URL: &str = "https://chatgpt.com/settings/usage";
/// A provider's `credential_ref` is this prefix plus its normal reference.
const MARKER: &str = super::provider::CHATGPT_MARKER;
const CHUNK: usize = 1_000;
const MAX_CHUNKS: usize = 24;
const SIGN_IN_WAIT: Duration = Duration::from_secs(300);

/// Serializes refreshes: parallel shader lanes ask for a token at the same
/// time, and a refresh token works only once.
static REFRESH: Mutex<()> = Mutex::new(());

/// A saved ChatGPT connection. Never logged or shown.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Connection {
    pub client_id: String,
    pub subject: String,
    pub email: Option<String>,
    pub scopes: Vec<String>,
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Unix seconds.
    pub expires_at: u64,
    /// Unix seconds; the server may ask clients not to refresh before this.
    pub earliest_refresh_at: Option<u64>,
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("scopes", &self.scopes)
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

impl Connection {
    /// The person allowed this app to use their ChatGPT plan.
    pub(crate) fn sharing(&self) -> bool {
        self.scopes.iter().any(|scope| scope == SHARING_SCOPE)
    }
}

/// The `credential_ref` for a provider that signs in with ChatGPT.
pub(crate) fn marker(reference: &str) -> String {
    format!("{MARKER}{reference}")
}

/// The provider reference inside a ChatGPT `credential_ref`, if it is one.
pub(crate) fn marked(credential_ref: &str) -> Option<&str> {
    credential_ref.strip_prefix(MARKER)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_secs())
}

fn random_value() -> Result<String, String> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| "The system random source failed.".to_owned())?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

// ---------------------------------------------------------------- storage

fn piece(reference_: &str, name: &str) -> String {
    reference(reference_, &format!("chatgpt-{name}"))
}

pub(crate) fn save(
    store: &dyn CredentialStore,
    reference_: &str,
    connection: &Connection,
) -> Result<(), CredentialError> {
    let json = Zeroizing::new(serde_json::to_string(connection).expect("connection serializes"));
    let pieces: Vec<&str> = json
        .as_bytes()
        .chunks(CHUNK)
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or_default())
        .collect();
    // JSON from serde_json is ASCII-safe for tokens; refuse anything that
    // would split a character rather than store it corrupted.
    if pieces.len() > MAX_CHUNKS || pieces.concat().len() != json.len() {
        return Err(CredentialError::Unavailable);
    }
    for (index, value) in pieces.iter().enumerate() {
        store.put(&piece(reference_, &index.to_string()), value)?;
    }
    store.put(&piece(reference_, "count"), &pieces.len().to_string())
}

pub(crate) fn load(
    store: &dyn CredentialStore,
    reference_: &str,
) -> Result<Connection, CredentialError> {
    let count: usize = store
        .get(&piece(reference_, "count"))?
        .parse()
        .map_err(|_| CredentialError::Unavailable)?;
    if count == 0 || count > MAX_CHUNKS {
        return Err(CredentialError::Unavailable);
    }
    let mut json = Zeroizing::new(String::new());
    for index in 0..count {
        json.push_str(&store.get(&piece(reference_, &index.to_string()))?);
    }
    serde_json::from_str(&json).map_err(|_| CredentialError::Unavailable)
}

fn forget(store: &dyn CredentialStore, reference_: &str) {
    let _ = store.delete(&piece(reference_, "count"));
    for index in 0..MAX_CHUNKS {
        if store.delete(&piece(reference_, &index.to_string())) == Err(CredentialError::Missing) {
            break;
        }
    }
}

/// One random ID per profile, so OpenAI can tell installations apart.
fn host_id(store: &dyn CredentialStore, profile: &str) -> Result<String, String> {
    let key = reference(profile, "chatgpt-host-id");
    if let Ok(value) = store.get(&key) {
        return Ok(value.to_string());
    }
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| "The system random source failed.".to_owned())?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    let id = format!(
        "urn:uuid:{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    );
    store
        .put(&key, &id)
        .map_err(|_| "The OS credential store is unavailable.".to_owned())?;
    Ok(id)
}

// ---------------------------------------------------------------- HTTP

struct Discovery {
    authorization: String,
    token: String,
    revocation: Option<String>,
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| "Could not start a secure connection.".to_owned())
}

fn discover(client: &reqwest::blocking::Client) -> Result<Discovery, String> {
    let failed = || "ChatGPT sign-in settings could not be checked. Try again.".to_owned();
    let value: Value = client
        .get(format!("{ISSUER}/.well-known/openid-configuration"))
        .send()
        .ok()
        .filter(|response| response.status().is_success())
        .and_then(|response| response.json().ok())
        .ok_or_else(failed)?;
    if value["issuer"] != ISSUER {
        return Err(failed());
    }
    let endpoint = |key: &str| {
        value[key]
            .as_str()
            .filter(|url| {
                url::Url::parse(url).is_ok_and(|url| url.origin().ascii_serialization() == ISSUER)
            })
            .map(str::to_owned)
    };
    Ok(Discovery {
        authorization: endpoint("authorization_endpoint").ok_or_else(failed)?,
        token: endpoint("token_endpoint").ok_or_else(failed)?,
        revocation: endpoint("revocation_endpoint"),
    })
}

fn token_request(
    client: &reqwest::blocking::Client,
    discovery: &Discovery,
    form: &[(&str, &str)],
) -> Result<Value, String> {
    let response = client
        .post(&discovery.token)
        .header("accept", "application/json")
        .form(form)
        .send()
        .map_err(|_| "Could not reach ChatGPT. Check your connection and try again.".to_owned())?;
    let status = response.status();
    let body: Value = response.json().unwrap_or(Value::Null);
    if !status.is_success() {
        let code = body["error"].as_str().unwrap_or("unknown_error");
        return Err(match code {
            "invalid_grant" => "Your ChatGPT connection has expired. Sign in again.".into(),
            _ => format!("ChatGPT sign-in failed ({code})."),
        });
    }
    Ok(body)
}

/// Turns a token response into a connection, keeping identity from `previous`.
fn connection_from(
    body: &Value,
    client_id: &str,
    identity: (String, Option<String>),
    previous_scopes: Option<&[String]>,
    previous_refresh: Option<&str>,
) -> Result<Connection, String> {
    let incomplete = || "ChatGPT returned incomplete credentials. Please sign in again.".to_owned();
    let scopes: Vec<String> = match body["scope"].as_str() {
        Some(scope) => scope.split_whitespace().map(str::to_owned).collect(),
        // A refresh may omit unchanged scopes.
        None => previous_scopes.ok_or_else(incomplete)?.to_vec(),
    };
    let access_token = body["access_token"]
        .as_str()
        .filter(|token| !token.is_empty())
        .ok_or_else(incomplete)?;
    if !body["token_type"]
        .as_str()
        .is_some_and(|kind| kind.eq_ignore_ascii_case("bearer"))
    {
        return Err(incomplete());
    }
    let expires_in = body["expires_in"]
        .as_u64()
        .filter(|seconds| *seconds > 0)
        .ok_or_else(incomplete)?;
    let refresh_token = body["refresh_token"]
        .as_str()
        .filter(|token| !token.is_empty())
        .or(previous_refresh)
        .map(str::to_owned);
    if scopes.iter().any(|scope| scope == "offline_access") && refresh_token.is_none() {
        return Err(incomplete());
    }
    let earliest_refresh_at = match &body["earliest_refresh_at"] {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => utc_seconds(text),
        _ => None,
    };
    Ok(Connection {
        client_id: client_id.to_owned(),
        subject: identity.0,
        email: identity.1,
        scopes,
        access_token: access_token.to_owned(),
        refresh_token,
        expires_at: now() + expires_in,
        earliest_refresh_at,
    })
}

/// Parses `YYYY-MM-DDTHH:MM:SS[.fff]Z` into Unix seconds. Anything else is
/// ignored, which only means the app may try a refresh a little early.
fn utc_seconds(text: &str) -> Option<u64> {
    let text = text.strip_suffix('Z')?;
    let (date, time) = text.split_once('T')?;
    let mut date = date.splitn(3, '-').map(|part| part.parse::<i64>().ok());
    let (year, month, day) = (date.next()??, date.next()??, date.next()??);
    let mut time = time
        .split('.')
        .next()?
        .splitn(3, ':')
        .map(|p| p.parse::<i64>().ok());
    let (hour, minute, second) = (time.next()??, time.next()??, time.next()??);
    // Days from civil date (Howard Hinnant's algorithm).
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + hour * 3_600 + minute * 60 + second).ok()
}

/// Checks ID token claims; returns the subject and email.
fn identity(
    id_token: &str,
    client_id: &str,
    nonce: Option<&str>,
) -> Result<(String, Option<String>), String> {
    let invalid = || "The ChatGPT identity could not be verified. Please sign in again.".to_owned();
    let payload = id_token.split('.').nth(1).ok_or_else(invalid)?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .map_err(|_| invalid())?;
    let claims: Value = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
    let audience_ok = match &claims["aud"] {
        Value::String(audience) => audience == client_id,
        Value::Array(audiences) => {
            audiences.iter().any(|audience| audience == client_id)
                && (audiences.len() == 1 || claims["azp"] == client_id)
        }
        _ => false,
    };
    let subject = claims["sub"]
        .as_str()
        .filter(|subject| !subject.is_empty())
        .ok_or_else(invalid)?;
    if claims["iss"] != ISSUER
        || !audience_ok
        || claims["azp"].as_str().is_some_and(|azp| azp != client_id)
        || claims["exp"].as_u64().is_none_or(|exp| exp + 300 < now())
        || nonce.is_some_and(|nonce| claims["nonce"] != nonce)
    {
        return Err(invalid());
    }
    Ok((
        subject.to_owned(),
        claims["email"].as_str().map(str::to_owned),
    ))
}

// ---------------------------------------------------------------- sign-in

struct Callback {
    code: String,
    client_id: String,
}

/// Waits for the browser to return to `127.0.0.1:<port>/auth/callback`.
fn wait_for_callback(
    listener: &TcpListener,
    port: u16,
    state: &str,
    saved_client: Option<&str>,
    cancelled: &AtomicBool,
) -> Result<Callback, String> {
    listener
        .set_nonblocking(true)
        .map_err(|_| "The local sign-in listener could not start.".to_owned())?;
    let deadline = Instant::now() + SIGN_IN_WAIT;
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err("Sign-in was cancelled.".into());
        }
        if Instant::now() > deadline {
            return Err("Sign-in timed out. Try again.".into());
        }
        let stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(_) => return Err("The local sign-in listener stopped. Try again.".into()),
        };
        if let Some(result) = handle_callback(stream, port, state, saved_client) {
            return result;
        }
    }
}

/// Answers one loopback request. `None` means it was unrelated; keep waiting.
fn handle_callback(
    mut stream: TcpStream,
    port: u16,
    state: &str,
    saved_client: Option<&str>,
) -> Option<Result<Callback, String>> {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let mut request = Vec::new();
    let mut buffer = [0_u8; 2048];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") && request.len() < 16 * 1024 {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => request.extend_from_slice(&buffer[..read]),
        }
    }
    let request = String::from_utf8_lossy(&request);
    let mut lines = request.lines();
    let target = lines
        .next()
        .and_then(|line| line.strip_prefix("GET "))
        .and_then(|line| line.split(' ').next());
    let host_ok = lines.any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.eq_ignore_ascii_case("host") && value.trim() == format!("127.0.0.1:{port}")
        })
    });
    let url = target
        .filter(|_| host_ok)
        .and_then(|target| url::Url::parse(&format!("http://127.0.0.1:{port}{target}")).ok())
        .filter(|url| url.path() == CALLBACK_PATH);
    let Some(url) = url else {
        respond(&mut stream, "404 Not Found", "Not found.");
        return None;
    };
    let values = |name: &str| -> Vec<String> {
        url.query_pairs()
            .filter(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
            .collect()
    };
    let states = values("state");
    if states.len() != 1 || !same(states[0].as_bytes(), state.as_bytes()) {
        respond(
            &mut stream,
            "400 Bad Request",
            "Invalid sign-in state. Return to the browser tab that started sign-in.",
        );
        return None;
    }
    respond(
        &mut stream,
        "200 OK",
        "ButtonsCLI is finishing your ChatGPT connection. You can close this tab.",
    );
    if let Some(error) = values("error").first() {
        return Some(Err(match error.as_str() {
            "access_denied" => "Sign-in was cancelled in the browser.".into(),
            other => format!("ChatGPT sign-in failed ({}).", truncate_code(other)),
        }));
    }
    let codes = values("code");
    let returned = values("client_id");
    let client_id = returned.first().map(String::as_str).or(saved_client);
    let valid_client = client_id.is_some_and(|id| {
        !id.is_empty()
            && id.len() <= 200
            && id != DYNAMIC_CLIENT
            && id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    });
    if codes.len() != 1
        || returned.len() > 1
        || !valid_client
        || saved_client.is_some_and(|saved| returned.first().is_some_and(|id| id != saved))
    {
        return Some(Err(
            "ChatGPT did not finish registering the app. Try signing in again.".into(),
        ));
    }
    Some(Ok(Callback {
        code: codes[0].clone(),
        client_id: client_id.unwrap_or_default().to_owned(),
    }))
}

fn truncate_code(code: &str) -> String {
    code.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .take(60)
        .collect()
}

fn same(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .fold(0_u8, |diff, (a, b)| diff | (a ^ b))
            == 0
}

fn respond(stream: &mut TcpStream, status: &str, message: &str) {
    let body = format!(
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>ButtonsCLI</title>\
         <style>body{{font:17px system-ui;max-width:32rem;margin:18vh auto;padding:24px;\
         background:#14121c;color:#eee}}</style><h1>ButtonsCLI</h1><p>{message}</p></html>"
    );
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\ncontent-type: text/html; charset=utf-8\r\ncache-control: no-store\r\n\
         referrer-policy: no-referrer\r\ncontent-security-policy: default-src 'none'; \
         style-src 'unsafe-inline'\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
}

/// Opens the browser for sign-in and waits for the result. Blocking; run it
/// on a worker thread. `open` receives the authorization URL (no secrets).
pub(crate) fn sign_in(
    store: &dyn CredentialStore,
    profile: &str,
    reference_: &str,
    cancelled: &AtomicBool,
    open: &dyn Fn(&str),
) -> Result<Connection, String> {
    let previous = load(store, reference_).ok();
    let client = client()?;
    let discovery = discover(&client)?;
    let (state, nonce, verifier) = (random_value()?, random_value()?, random_value()?);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|_| "The local sign-in listener could not start.".to_owned())?;
    let port = listener
        .local_addr()
        .map_err(|_| "The local sign-in listener could not start.".to_owned())?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{port}{CALLBACK_PATH}");
    let mut url = url::Url::parse(&discovery.authorization).map_err(|_| "Bad sign-in URL.")?;
    {
        let mut query = url.query_pairs_mut();
        let saved_client = previous.as_ref().map(|c| c.client_id.as_str());
        query
            .append_pair("client_id", saved_client.unwrap_or(DYNAMIC_CLIENT))
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", &redirect_uri)
            .append_pair("scope", SCOPES)
            .append_pair("resource", RESOURCE)
            .append_pair("state", &state)
            .append_pair("nonce", &nonce)
            .append_pair("code_challenge_method", "S256")
            .append_pair("code_challenge", &challenge)
            .append_pair("ext_agent_host_id", &host_id(store, profile)?);
        if saved_client.is_none() {
            query.append_pair("agent_name_hint", "ButtonsCLI");
        }
        if let Some(email) = previous.as_ref().and_then(|c| c.email.as_deref()) {
            query.append_pair("login_hint", email);
        }
    }
    open(url.as_str());
    let callback = wait_for_callback(
        &listener,
        port,
        &state,
        previous.as_ref().map(|c| c.client_id.as_str()),
        cancelled,
    )?;
    drop(listener);
    let body = token_request(
        &client,
        &discovery,
        &[
            ("grant_type", "authorization_code"),
            ("client_id", &callback.client_id),
            ("code", &callback.code),
            ("code_verifier", &verifier),
            ("redirect_uri", &redirect_uri),
            ("resource", RESOURCE),
        ],
    )?;
    let id_token = body["id_token"]
        .as_str()
        .ok_or("ChatGPT did not return a verifiable identity. Try signing in again.")?;
    let person = identity(id_token, &callback.client_id, Some(&nonce))?;
    if previous
        .as_ref()
        .is_some_and(|previous| previous.subject != person.0)
    {
        return Err("This sign-in returned a different ChatGPT account. Add a separate ChatGPT connection for it.".into());
    }
    let connection = connection_from(&body, &callback.client_id, person, None, None)?;
    save(store, reference_, &connection)
        .map_err(|_| "Signed in, but the OS credential store could not save it.".to_owned())?;
    Ok(connection)
}

/// A current access token for requests, refreshing it when needed.
pub(crate) fn access_token(
    store: &dyn CredentialStore,
    reference_: &str,
) -> Result<Zeroizing<String>, CredentialError> {
    let _guard = REFRESH
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let connection = load(store, reference_)?;
    if !connection.sharing() {
        return Err(CredentialError::SignInNeeded);
    }
    if connection.expires_at > now() + 60 {
        return Ok(Zeroizing::new(connection.access_token));
    }
    if connection
        .earliest_refresh_at
        .is_some_and(|earliest| earliest > now())
        && connection.expires_at > now()
    {
        return Ok(Zeroizing::new(connection.access_token));
    }
    let refreshed =
        refresh(store, reference_, &connection).map_err(|_| CredentialError::SignInNeeded)?;
    Ok(Zeroizing::new(refreshed.access_token))
}

fn refresh(
    store: &dyn CredentialStore,
    reference_: &str,
    previous: &Connection,
) -> Result<Connection, String> {
    let refresh_token = previous
        .refresh_token
        .as_deref()
        .ok_or("Your ChatGPT connection has expired. Sign in again.")?;
    let client = client()?;
    let discovery = discover(&client)?;
    let body = token_request(
        &client,
        &discovery,
        &[
            ("grant_type", "refresh_token"),
            ("client_id", &previous.client_id),
            ("refresh_token", refresh_token),
            ("resource", RESOURCE),
        ],
    )?;
    let person = match body["id_token"].as_str() {
        Some(id_token) => identity(id_token, &previous.client_id, None)?,
        None => (previous.subject.clone(), previous.email.clone()),
    };
    let account_matches = person.0 == previous.subject;
    let connection = connection_from(
        &body,
        &previous.client_id,
        if account_matches {
            person
        } else {
            (previous.subject.clone(), previous.email.clone())
        },
        Some(&previous.scopes),
        Some(refresh_token),
    )?;
    // The old refresh token is spent: save the new one before anything else.
    save(store, reference_, &connection).map_err(|_| {
        "The OS credential store could not save the refreshed connection.".to_owned()
    })?;
    if !account_matches {
        return Err("The refreshed ChatGPT account does not match. Sign in again.".into());
    }
    Ok(connection)
}

/// The OS-saved key for `provider`: a fresh access token for a ChatGPT sign-in,
/// else its saved API key. `None` when it has neither.
pub(crate) fn saved_key(
    store: &dyn CredentialStore,
    profile: &str,
    provider: &super::provider::ProviderProfile,
) -> Result<Option<Zeroizing<String>>, CredentialError> {
    let reference_ = reference(profile, &provider.id);
    match provider.credential_ref.as_deref() {
        Some(stored) if marked(stored) == Some(reference_.as_str()) => {
            access_token(store, &reference_).map(Some)
        }
        Some(stored) if stored == reference_ => store.get(&reference_).map(Some),
        _ => Ok(None),
    }
}

/// Puts the sign-in marker back on OpenAI connections whose tokens are still
/// saved but whose `credential_ref` is empty (builds before Oct 30 dropped it
/// while normalizing settings). Returns whether anything changed.
pub(crate) fn reconnect_saved(
    store: &dyn CredentialStore,
    profile: &str,
    settings: &mut super::provider::ProviderSettings,
) -> bool {
    let mut changed = false;
    for provider in &mut settings.providers {
        if provider.credential_ref.is_some()
            || !(provider.chatgpt_plan || provider.endpoint.trim_end_matches('/') == RESOURCE)
        {
            continue;
        }
        let reference_ = reference(profile, &provider.id);
        if load(store, &reference_).is_ok() {
            provider.credential_ref = Some(marker(&reference_));
            provider.chatgpt_plan = true;
            changed = true;
        }
    }
    if changed {
        settings.normalize(&mut Vec::new());
    }
    changed
}

/// Forgets the connection here and asks OpenAI to end it (best effort).
pub(crate) fn sign_out(store: &dyn CredentialStore, reference_: &str) -> Result<(), String> {
    let _guard = REFRESH
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let connection = load(store, reference_).ok();
    forget(store, reference_);
    let Some(token) = connection.as_ref().and_then(|c| c.refresh_token.clone()) else {
        return Ok(());
    };
    let revoked = client().ok().and_then(|client| {
        let endpoint = discover(&client).ok()?.revocation?;
        client
            .post(endpoint)
            .form(&[
                ("token", token.as_str()),
                ("token_type_hint", "refresh_token"),
                ("client_id", &connection.as_ref()?.client_id),
            ])
            .send()
            .ok()
            .filter(|response| response.status().is_success())
    });
    match revoked {
        Some(_) => Ok(()),
        None => Err("Signed out here, but ChatGPT did not confirm it. You can also remove the app in ChatGPT Settings.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::credentials::SessionCredentialStore;

    fn token(claims: Value) -> String {
        format!(
            "e30.{}.sig",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap())
        )
    }

    fn connection(access: usize) -> Connection {
        Connection {
            client_id: "client_1".into(),
            subject: "user-1".into(),
            email: Some("person@example.test".into()),
            scopes: vec![SHARING_SCOPE.into(), "offline_access".into()],
            access_token: "a".repeat(access),
            refresh_token: Some("FAKE-REFRESH".into()),
            expires_at: now() + 3_600,
            earliest_refresh_at: None,
        }
    }

    #[test]
    fn long_tokens_round_trip_through_credential_pieces() {
        let store = SessionCredentialStore::default();
        let reference_ = reference("profile", "provider");
        // Longer than one Windows credential, and shrinking leaves no stale data.
        save(&store, &reference_, &connection(5_000)).unwrap();
        save(&store, &reference_, &connection(10)).unwrap();
        let loaded = load(&store, &reference_).unwrap();
        assert_eq!(loaded.access_token.len(), 10);
        assert_eq!(
            access_token(&store, &reference_).unwrap().as_str(),
            "a".repeat(10)
        );
        assert!(!format!("{loaded:?}").contains("FAKE-REFRESH"));
        forget(&store, &reference_);
        assert_eq!(
            load(&store, &reference_).err(),
            Some(CredentialError::Missing)
        );
    }

    #[test]
    fn sign_in_marker_survives_normalizing_and_is_restored_when_dropped() {
        use super::super::provider::{ProviderApi, ProviderProfile, ProviderSettings};
        let store = SessionCredentialStore::default();
        let provider = |id: &str, endpoint: &str| ProviderProfile {
            id: id.into(),
            name: id.into(),
            endpoint: endpoint.into(),
            ..ProviderProfile::default()
        };
        let mut settings = ProviderSettings {
            providers: vec![
                provider("plan", "https://api.openai.com/v1/"),
                provider("other", "https://openrouter.ai/api/v1"),
            ],
            ..ProviderSettings::default()
        };
        let plan = reference("profile", "plan");
        settings.providers[0].credential_ref = Some(marker(&plan));
        settings.normalize(&mut Vec::new());
        assert_eq!(
            settings.providers[0].credential_ref.as_deref(),
            Some(marker(&plan).as_str())
        );
        assert!(settings.providers[0].chatgpt_plan);
        assert_eq!(settings.providers[0].api, ProviderApi::Responses);

        // What earlier builds saved: no marker, tokens still in the store.
        settings.providers[0].credential_ref = None;
        assert!(!reconnect_saved(&store, "profile", &mut settings));
        save(&store, &plan, &connection(10)).unwrap();
        save(&store, &reference("profile", "other"), &connection(10)).unwrap();
        assert!(reconnect_saved(&store, "profile", &mut settings));
        assert_eq!(
            settings.providers[0].credential_ref.as_deref(),
            Some(marker(&plan).as_str())
        );
        // Only OpenAI's endpoint can be a ChatGPT connection.
        assert_eq!(settings.providers[1].credential_ref, None);
        assert!(!reconnect_saved(&store, "profile", &mut settings));
    }

    /// Every AI task's first model on the active profile, as the app would
    /// send it after reconnecting. Reads native.json without saving it.
    #[test]
    #[ignore = "sends real requests; set BUTTONSCLI_TEST_LIVE_CHATGPT=1"]
    fn live_chatgpt_sign_in_reaches_every_ai_task() {
        use super::super::provider::{AiTask, ProviderSettings};
        use crate::assistant::credentials::SystemCredentialStore;
        assert_eq!(
            std::env::var("BUTTONSCLI_TEST_LIVE_CHATGPT").as_deref(),
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
        let mut settings: ProviderSettings =
            serde_json::from_value(document["preferences"]["provider_settings"].clone()).unwrap();
        settings.normalize(&mut Vec::new());
        println!(
            "reconnected: {}",
            reconnect_saved(&SystemCredentialStore, &profile, &mut settings)
        );
        let transport = super::super::transport::ReqwestTransport;
        for task in [
            AiTask::Help,
            AiTask::Theme,
            AiTask::Shader,
            AiTask::ShaderJudge,
            AiTask::Simulation,
        ] {
            let Some(provider) = settings.chain(task).into_iter().next() else {
                println!("{task:?}: no provider");
                continue;
            };
            let key = saved_key(&SystemCredentialStore, &profile, &provider);
            let result = key.map_err(|error| error.to_string()).and_then(|key| {
                super::super::client::stream_completion(
                    &transport,
                    &provider,
                    key,
                    ("Reply briefly.", "Reply OK."),
                    &[],
                    &AtomicBool::new(false),
                    &mut |_| {},
                )
                .map_err(|error| error.to_string())
            });
            println!(
                "{task:?} via {} ({}, plan={}): {:?}",
                provider.name,
                provider.model,
                provider.chatgpt_plan,
                result.map(|reply| reply.chars().take(40).collect::<String>())
            );
        }
        let plan = settings
            .providers
            .iter()
            .find(|provider| provider.chatgpt_plan)
            .expect("a ChatGPT connection");
        let key = saved_key(&SystemCredentialStore, &profile, plan).unwrap();
        println!(
            "test_connection: {:?}",
            super::super::client::test_connection(&transport, plan, key.clone())
                .map_err(|error| error.to_string())
        );
        println!(
            "models: {:?}",
            super::super::client::discover_models(&transport, plan, key)
                .map(|models| models.len())
                .map_err(|error| error.to_string())
        );
    }

    #[test]
    fn plan_usage_needs_the_sharing_scope() {
        let store = SessionCredentialStore::default();
        let reference_ = reference("profile", "provider");
        let mut signed_in = connection(10);
        signed_in.scopes.retain(|scope| scope != SHARING_SCOPE);
        save(&store, &reference_, &signed_in).unwrap();
        assert!(access_token(&store, &reference_).is_err());
    }

    #[test]
    fn identity_claims_are_checked() {
        let good = json_claims("client_1", "n1");
        assert_eq!(
            identity(&token(good.clone()), "client_1", Some("n1")).unwrap(),
            ("user-1".into(), Some("person@example.test".into()))
        );
        assert!(identity(&token(good.clone()), "other", Some("n1")).is_err());
        assert!(identity(&token(good.clone()), "client_1", Some("n2")).is_err());
        let mut wrong_issuer = good.clone();
        wrong_issuer["iss"] = "https://auth.example.test".into();
        assert!(identity(&token(wrong_issuer), "client_1", Some("n1")).is_err());
        let mut expired = good;
        expired["exp"] = (now() - 3_600).into();
        assert!(identity(&token(expired), "client_1", Some("n1")).is_err());
    }

    fn json_claims(audience: &str, nonce: &str) -> Value {
        serde_json::json!({
            "iss": ISSUER, "aud": audience, "sub": "user-1", "nonce": nonce,
            "exp": now() + 600, "iat": now(), "email": "person@example.test"
        })
    }

    #[test]
    fn token_responses_need_complete_credentials() {
        let person = ("user-1".to_owned(), None);
        let body = serde_json::json!({
            "access_token": "AT", "token_type": "Bearer", "expires_in": 3600,
            "refresh_token": "RT", "scope": format!("openid offline_access {SHARING_SCOPE}"),
            "earliest_refresh_at": "2026-10-29T08:00:00Z"
        });
        let made = connection_from(&body, "client_1", person.clone(), None, None).unwrap();
        assert!(made.sharing());
        assert_eq!(made.earliest_refresh_at, Some(1_793_260_800));
        let mut no_refresh = body.clone();
        no_refresh["refresh_token"] = Value::Null;
        assert!(connection_from(&no_refresh, "client_1", person.clone(), None, None).is_err());
        // A refresh may leave out unchanged scopes and keep the refresh token.
        let mut refreshed = no_refresh;
        refreshed["scope"] = Value::Null;
        let scopes = made.scopes.clone();
        let kept =
            connection_from(&refreshed, "client_1", person, Some(&scopes), Some("RT")).unwrap();
        assert_eq!(kept.refresh_token.as_deref(), Some("RT"));
    }

    #[test]
    fn callback_accepts_only_its_own_state_host_and_path() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let send = |request: String| {
            std::thread::spawn(move || {
                let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
                stream.write_all(request.as_bytes()).unwrap();
                let mut reply = String::new();
                let _ = stream.read_to_string(&mut reply);
                reply
            })
        };
        let request =
            |path: &str, host: &str| format!("GET {path} HTTP/1.1\r\nHost: {host}\r\n\r\n");
        let host = format!("127.0.0.1:{port}");
        let wrong_state = send(request("/auth/callback?state=nope&code=c", &host));
        let wrong_host = send(request("/auth/callback?state=S&code=c", "evil.example"));
        let good = send(request(
            "/auth/callback?state=S&code=CODE&client_id=client_9",
            &host,
        ));
        let callback =
            wait_for_callback(&listener, port, "S", None, &AtomicBool::new(false)).unwrap();
        assert_eq!(
            (callback.code.as_str(), callback.client_id.as_str()),
            ("CODE", "client_9")
        );
        assert!(good.join().unwrap().starts_with("HTTP/1.1 200"));
        // Requests still queued are refused when the listener closes.
        drop(listener);
        for reply in [wrong_state.join().unwrap(), wrong_host.join().unwrap()] {
            assert!(!reply.starts_with("HTTP/1.1 200"), "{reply}");
        }
    }
}
