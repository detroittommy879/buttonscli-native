//! Hosted account API adapter. Tokens are held in zeroizing memory and are
//! never formatted, logged, or placed in Preferences.

use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{OnceLock, RwLock};

use serde::Deserialize;
use serde_json::{json, Value};
use zeroize::Zeroizing;

use crate::assistant::credentials::CredentialStore;
use crate::assistant::transport::{HttpTransport, Method, Request, ReqwestTransport};
use crate::features::access::{Entitlement, GrantSource, Plan};
use crate::features::catalog::FeatureKey;

const AUTH_BASE_URL: &str = "https://auth.buttonscli.com";
const HTTP: ReqwestTransport = ReqwestTransport;
static ACCOUNT_GENERATION: AtomicU64 = AtomicU64::new(0);
static CURRENT_ENTITLEMENT: OnceLock<RwLock<Option<Entitlement>>> = OnceLock::new();

fn entitlement_cache() -> &'static RwLock<Option<Entitlement>> {
    CURRENT_ENTITLEMENT.get_or_init(|| RwLock::new(None))
}

pub(crate) fn current_entitlement() -> Option<Entitlement> {
    let grant = entitlement_cache().read().ok()?.clone()?;
    let now = unix_now();
    (grant.expires_at_unix > now).then_some(grant)
}

pub(crate) fn set_current_entitlement(grant: Option<Entitlement>) -> u64 {
    let generation = ACCOUNT_GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
    if let Ok(mut current) = entitlement_cache().write() {
        *current = grant;
    }
    generation
}

fn clear_entitlement_if_generation(generation: u64) {
    if ACCOUNT_GENERATION.load(Ordering::Acquire) == generation {
        if let Ok(mut current) = entitlement_cache().write() {
            *current = None;
        }
    }
}

pub(crate) fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(u64::MAX)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AccountUser {
    pub(crate) id: String,
    pub(crate) email: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AccountEntitlements {
    pub(crate) plan: Plan,
    pub(crate) email: Option<String>,
    pub(crate) active_features: Vec<FeatureKey>,
    pub(crate) last_synced_at: Option<String>,
}

impl AccountEntitlements {
    pub(crate) fn server_grant(&self, expires_at_unix: u64) -> Entitlement {
        Entitlement {
            plan: self.plan,
            source: GrantSource::Server,
            expires_at_unix,
            active_features: self.active_features.clone(),
        }
    }
}

/// Session tokens never appear in Debug output and zeroize their backing buffer.
pub(crate) struct SessionToken(Zeroizing<String>);

impl SessionToken {
    fn new(value: String) -> Self {
        Self(Zeroizing::new(value))
    }

    pub(crate) fn expose(&self) -> &str {
        self.0.as_str()
    }

    pub(crate) fn duplicate_for_refresh(&self) -> Self {
        Self::new(self.expose().to_owned())
    }
}

impl std::fmt::Debug for SessionToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SessionToken([redacted])")
    }
}

#[derive(Debug)]
pub(crate) struct AccountSession {
    pub(crate) token: SessionToken,
    pub(crate) user: AccountUser,
    pub(crate) entitlements: AccountEntitlements,
    pub(crate) expires_at_unix: u64,
}

impl AccountSession {
    pub(crate) fn entitlement(&self) -> Entitlement {
        self.entitlements.server_grant(self.expires_at_unix)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LoginCodeResult {
    pub(crate) delivery: String,
    pub(crate) expires_in_seconds: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AccountError {
    Endpoint,
    Network,
    ResponseTooLarge,
    Http(u16),
    InvalidReply,
    InvalidSession,
    CredentialStore,
}

impl std::fmt::Display for AccountError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Endpoint => formatter.write_str("account service endpoint is invalid"),
            Self::Network => formatter.write_str("could not reach the ButtonsCLI account service"),
            Self::ResponseTooLarge => {
                formatter.write_str("account service response exceeded 1 MiB")
            }
            Self::Http(status) => write!(formatter, "account service returned HTTP {status}"),
            Self::InvalidReply => {
                formatter.write_str("account service returned an invalid response")
            }
            Self::InvalidSession => formatter.write_str("account session is invalid or expired"),
            Self::CredentialStore => formatter.write_str("OS credential store is unavailable"),
        }
    }
}

pub(crate) struct AccountClient<'a> {
    base: url::Url,
    transport: &'a dyn HttpTransport,
}

impl AccountClient<'static> {
    pub(crate) fn production() -> Result<Self, AccountError> {
        Self::new(AUTH_BASE_URL, &HTTP)
    }
}

impl<'a> AccountClient<'a> {
    fn new(base: &str, transport: &'a dyn HttpTransport) -> Result<Self, AccountError> {
        let mut base = parse_service_url(base)?;
        if !base.path().ends_with('/') {
            base.set_path(&format!("{}/", base.path()));
        }
        Ok(Self { base, transport })
    }

    pub(crate) fn start_email_login(
        &self,
        email: &str,
        install_id: Option<&str>,
    ) -> Result<LoginCodeResult, AccountError> {
        let email = email.trim();
        if email.is_empty() || email.len() > 320 || !valid_optional_text(install_id, 128) {
            return Err(AccountError::InvalidReply);
        }
        let response = self.send(
            Method::Post,
            "/api/v1/auth/email/start",
            Some(json!({"email": email, "installId": install_id})),
            None,
        )?;
        let payload: StartLoginPayload = parse_ok_json(response)?;
        if !payload.ok || payload.expires_in_seconds == 0 || payload.delivery.len() > 32 {
            return Err(AccountError::InvalidReply);
        }
        Ok(LoginCodeResult {
            delivery: payload.delivery,
            expires_in_seconds: payload.expires_in_seconds,
        })
    }

    pub(crate) fn verify_email_login(
        &self,
        email: &str,
        code: &str,
        install_id: Option<&str>,
        device_name: Option<&str>,
    ) -> Result<AccountSession, AccountError> {
        let email = email.trim();
        let code = code.trim();
        if email.is_empty()
            || email.len() > 320
            || code.is_empty()
            || code.len() > 128
            || !valid_optional_text(install_id, 128)
            || !valid_optional_text(device_name, 128)
        {
            return Err(AccountError::InvalidReply);
        }
        let response = self.send(
            Method::Post,
            "/api/v1/auth/email/verify",
            Some(json!({
                "email": email,
                "code": code,
                "installId": install_id,
                "deviceName": device_name,
            })),
            None,
        )?;
        let payload: VerifyLoginPayload = parse_ok_json(response)?;
        let expiry = parse_rfc3339_unix(&payload.expires_at).ok_or(AccountError::InvalidSession)?;
        if !payload.ok
            || payload.token.trim().is_empty()
            || payload.token.len() > 4096
            || payload.token.chars().any(char::is_whitespace)
        {
            return Err(AccountError::InvalidSession);
        }
        Ok(AccountSession {
            token: SessionToken::new(payload.token),
            user: parse_user(payload.user)?,
            entitlements: parse_entitlements(payload.entitlements)?,
            expires_at_unix: expiry,
        })
    }

    /// Revalidate a keyring token online before restoring any account grant.
    pub(crate) fn restore_session(
        &self,
        token: Zeroizing<String>,
        expires_at_unix: u64,
    ) -> Result<AccountSession, AccountError> {
        let now = unix_now();
        if expires_at_unix <= now || token.is_empty() || token.len() > 4096 {
            return Err(AccountError::InvalidSession);
        }
        let token = SessionToken(token);
        let (user, entitlements) = self.current_account(&token)?;
        Ok(AccountSession {
            token,
            user,
            entitlements,
            expires_at_unix,
        })
    }

    pub(crate) fn current_account(
        &self,
        token: &SessionToken,
    ) -> Result<(AccountUser, AccountEntitlements), AccountError> {
        let response = self.send(Method::Get, "/api/v1/me", None, Some(token.expose()))?;
        let payload: CurrentAccountPayload = parse_ok_json(response)?;
        Ok((
            parse_user(payload.user)?,
            parse_entitlements(payload.entitlements)?,
        ))
    }

    pub(crate) fn resolve_entitlements(
        &self,
        token: &SessionToken,
    ) -> Result<AccountEntitlements, AccountError> {
        let response = self.send(
            Method::Post,
            "/api/v1/entitlements/resolve",
            Some(json!({})),
            Some(token.expose()),
        )?;
        let payload: EntitlementPayload = parse_ok_json(response)?;
        parse_entitlements(payload)
    }

    pub(crate) fn logout(&self, token: &SessionToken) -> Result<(), AccountError> {
        let response = self.send(
            Method::Post,
            "/api/v1/auth/logout",
            None,
            Some(token.expose()),
        )?;
        let payload: LogoutPayload = parse_ok_json(response)?;
        if payload.ok {
            Ok(())
        } else {
            Err(AccountError::InvalidReply)
        }
    }

    fn send(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        bearer: Option<&str>,
    ) -> Result<crate::assistant::transport::Response, AccountError> {
        if bearer.is_some_and(|token| {
            token.is_empty() || token.len() > 4096 || token.chars().any(char::is_whitespace)
        }) {
            return Err(AccountError::InvalidSession);
        }
        let url = self
            .base
            .join(path.trim_start_matches('/'))
            .map_err(|_| AccountError::Endpoint)?;
        self.transport
            .execute(Request {
                method,
                url: url.into(),
                body,
                key: bearer.map(|token| Zeroizing::new(token.to_owned())),
            })
            .map_err(|error| match error {
                crate::assistant::transport::TransportError::InvalidEndpoint => {
                    AccountError::Endpoint
                }
                crate::assistant::transport::TransportError::ResponseTooLarge => {
                    AccountError::ResponseTooLarge
                }
                crate::assistant::transport::TransportError::Network
                | crate::assistant::transport::TransportError::InvalidCredential
                | crate::assistant::transport::TransportError::Cancelled => AccountError::Network,
            })
    }
}

/// Refresh server grants every five minutes and close access on revocation or
/// an unsuccessful refresh. A generation guard prevents a logout racing this
/// background request from restoring stale access.
pub(crate) fn start_entitlement_refresh(
    token: SessionToken,
    expires_at_unix: u64,
    context: egui::Context,
) {
    let generation = ACCOUNT_GENERATION.load(Ordering::Acquire);
    let result = std::thread::Builder::new()
        .name("buttonscli-entitlement-refresh".into())
        .spawn(move || loop {
            for _ in 0..300 {
                std::thread::sleep(std::time::Duration::from_secs(1));
                if ACCOUNT_GENERATION.load(Ordering::Acquire) != generation
                    || expires_at_unix <= unix_now()
                {
                    return;
                }
            }
            let refreshed =
                AccountClient::production().and_then(|client| client.resolve_entitlements(&token));
            if ACCOUNT_GENERATION.load(Ordering::Acquire) != generation {
                return;
            }
            match refreshed {
                Ok(entitlements) => {
                    let grant = entitlements.server_grant(expires_at_unix);
                    if let Ok(mut current) = entitlement_cache().write() {
                        *current = Some(grant);
                    }
                }
                Err(_) => clear_entitlement_if_generation(generation),
            }
            context.request_repaint();
        });
    if result.is_err() {
        clear_entitlement_if_generation(generation);
    }
}

/// Store token and expiry as two independent, profile-scoped OS credentials.
/// If either write fails, both entries are removed to avoid a partial session.
pub(crate) fn save_session_credentials(
    store: &dyn CredentialStore,
    token_reference: &str,
    expiry_reference: &str,
    session: &AccountSession,
) -> Result<(), crate::assistant::credentials::CredentialError> {
    let expiry = session.expires_at_unix.to_string();
    store.put(token_reference, session.token.expose())?;
    if let Err(error) = store.put(expiry_reference, &expiry) {
        let _ = store.delete(token_reference);
        let _ = store.delete(expiry_reference);
        return Err(error);
    }
    Ok(())
}

pub(crate) fn remove_session_credentials(
    store: &dyn CredentialStore,
    token_reference: &str,
    expiry_reference: &str,
) {
    let _ = store.delete(token_reference);
    let _ = store.delete(expiry_reference);
}

fn parse_service_url(value: &str) -> Result<url::Url, AccountError> {
    let url = url::Url::parse(value).map_err(|_| AccountError::Endpoint)?;
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
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(AccountError::Endpoint);
    }
    Ok(url)
}

fn valid_optional_text(value: Option<&str>, limit: usize) -> bool {
    value.is_none_or(|text| text.len() <= limit && !text.chars().any(char::is_control))
}

fn parse_ok_json<T: for<'de> Deserialize<'de>>(
    response: crate::assistant::transport::Response,
) -> Result<T, AccountError> {
    if !(200..300).contains(&response.status) {
        return Err(AccountError::Http(response.status));
    }
    serde_json::from_slice(&response.body).map_err(|_| AccountError::InvalidReply)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartLoginPayload {
    ok: bool,
    delivery: String,
    expires_in_seconds: u64,
}

#[derive(Deserialize)]
struct UserPayload {
    id: String,
    email: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EntitlementPayload {
    plan: String,
    email: Option<String>,
    #[serde(default)]
    active_features: Vec<String>,
    last_synced_at: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerifyLoginPayload {
    ok: bool,
    token: String,
    expires_at: String,
    user: UserPayload,
    entitlements: EntitlementPayload,
}

#[derive(Deserialize)]
struct CurrentAccountPayload {
    user: UserPayload,
    entitlements: EntitlementPayload,
}

#[derive(Deserialize)]
struct LogoutPayload {
    ok: bool,
}

fn parse_user(payload: UserPayload) -> Result<AccountUser, AccountError> {
    if payload.id.trim().is_empty()
        || payload.id.len() > 128
        || payload.email.trim().is_empty()
        || payload.email.len() > 320
    {
        return Err(AccountError::InvalidReply);
    }
    Ok(AccountUser {
        id: payload.id,
        email: payload.email,
    })
}

fn parse_entitlements(payload: EntitlementPayload) -> Result<AccountEntitlements, AccountError> {
    let plan = match payload.plan.as_str() {
        "free" => Plan::Free,
        "pro" => Plan::Pro,
        "enterprise" => Plan::Enterprise,
        _ => return Err(AccountError::InvalidReply),
    };
    let active_features = payload
        .active_features
        .iter()
        .filter_map(|name| FeatureKey::ALL.into_iter().find(|key| key.as_str() == name))
        .collect();
    Ok(AccountEntitlements {
        plan,
        email: payload.email.filter(|email| email.len() <= 320),
        active_features,
        last_synced_at: payload.last_synced_at.filter(|value| value.len() <= 64),
    })
}

fn parse_rfc3339_unix(value: &str) -> Option<u64> {
    let (date, time) = value.split_once('T')?;
    let mut date_parts = date.split('-');
    let year: i64 = date_parts.next()?.parse().ok()?;
    let month: i64 = date_parts.next()?.parse().ok()?;
    let day: i64 = date_parts.next()?.parse().ok()?;
    if date_parts.next().is_some() || !(1970..=9999).contains(&year) || !(1..=12).contains(&month) {
        return None;
    }
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = match month {
        2 if leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if !(1..=month_days).contains(&day) {
        return None;
    }
    let (clock, offset_seconds) = if let Some(clock) = time.strip_suffix('Z') {
        (clock, 0_i64)
    } else {
        let (at, sign_char) = time
            .char_indices()
            .skip(1)
            .find_map(|(at, ch)| matches!(ch, '+' | '-').then_some((at, ch)))?;
        let (clock, offset) = time.split_at(at);
        let sign = if sign_char == '+' { 1_i64 } else { -1_i64 };
        let (hours, minutes) = offset[1..].split_once(':')?;
        let hours: i64 = hours.parse().ok()?;
        let minutes: i64 = minutes.parse().ok()?;
        if hours > 23 || minutes > 59 {
            return None;
        }
        (clock, sign * (hours * 3600 + minutes * 60))
    };
    let clock = clock.split('.').next()?;
    let mut parts = clock.split(':');
    let hour: i64 = parts.next()?.parse().ok()?;
    let minute: i64 = parts.next()?.parse().ok()?;
    let second: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let adjusted_year = year - i64::from(month <= 2);
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let month_of_year = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_of_year + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days_since_epoch = era * 146_097 + day_of_era - 719_468;
    let local_seconds = days_since_epoch
        .checked_mul(86_400)?
        .checked_add(hour * 3600 + minute * 60 + second)?;
    u64::try_from(local_seconds.checked_sub(offset_seconds)?).ok()
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::assistant::credentials::{CredentialError, SessionCredentialStore};
    use crate::assistant::transport::{Response, TransportError};

    #[derive(Default)]
    struct MockTransport {
        responses: Mutex<Vec<Response>>,
        requests: Mutex<Vec<CapturedRequest>>,
    }

    #[derive(Debug)]
    struct CapturedRequest {
        method: &'static str,
        url: String,
        body: Option<Value>,
        bearer: Option<String>,
    }

    impl MockTransport {
        fn with_json(responses: &[(&str, u16)]) -> Self {
            Self {
                responses: Mutex::new(
                    responses
                        .iter()
                        .map(|(body, status)| Response {
                            status: *status,
                            body: body.as_bytes().to_vec(),
                            content_type: Some("application/json".into()),
                        })
                        .collect(),
                ),
                requests: Mutex::new(Vec::new()),
            }
        }
    }

    impl HttpTransport for MockTransport {
        fn execute(&self, request: Request) -> Result<Response, TransportError> {
            self.requests.lock().unwrap().push(CapturedRequest {
                method: match request.method {
                    Method::Get => "GET",
                    Method::Post => "POST",
                },
                url: request.url,
                body: request.body,
                bearer: request.key.map(|secret| secret.to_string()),
            });
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                return Err(TransportError::Network);
            }
            Ok(responses.remove(0))
        }
    }

    fn client<'a>(transport: &'a dyn HttpTransport) -> AccountClient<'a> {
        AccountClient::new("http://127.0.0.1:8787", transport).unwrap()
    }

    const LOGIN_RESPONSE: &str = r#"{"ok":true,"token":"secret-session-token","expiresAt":"2099-01-02T03:04:05.500Z","user":{"id":"u1","email":"a@example.test"},"entitlements":{"plan":"free","email":"a@example.test","activeFeatures":["aiHelp","futureFeature"],"source":"remote","lastSyncedAt":"2099-01-01T00:00:00Z"}}"#;
    const ME_RESPONSE: &str = r#"{"user":{"id":"u1","email":"a@example.test"},"entitlements":{"plan":"free","email":"a@example.test","activeFeatures":["aiHelp"],"source":"remote","lastSyncedAt":"2099-01-01T00:00:00Z"}}"#;

    #[test]
    fn email_login_me_resolve_and_logout_match_hosted_contract() {
        let transport = MockTransport::with_json(&[
            (
                r#"{"ok":true,"delivery":"email","expiresInSeconds":900}"#,
                200,
            ),
            (LOGIN_RESPONSE, 200),
            (ME_RESPONSE, 200),
            (
                r#"{"plan":"free","email":"a@example.test","activeFeatures":["aiHelp"],"lastSyncedAt":"2099-01-01T00:00:00Z"}"#,
                200,
            ),
            (r#"{"ok":true}"#, 200),
            (ME_RESPONSE, 200),
        ]);
        let client = client(&transport);
        let code = client
            .start_email_login(" a@example.test ", Some("install-one"))
            .unwrap();
        assert_eq!(code.delivery, "email");
        assert_eq!(code.expires_in_seconds, 900);
        let session = client
            .verify_email_login(
                "a@example.test",
                "246810",
                Some("install-one"),
                Some("Desktop"),
            )
            .unwrap();
        assert_eq!(session.user.email, "a@example.test");
        assert_eq!(
            session.entitlements.active_features,
            vec![FeatureKey::AiHelp]
        );
        assert_eq!(session.entitlement().source, GrantSource::Server);
        assert_eq!(format!("{:?}", session.token), "SessionToken([redacted])");
        assert!(session.expires_at_unix > 4_000_000_000);

        let (user, entitlements) = client.current_account(&session.token).unwrap();
        assert_eq!(user.id, "u1");
        assert_eq!(entitlements.active_features, vec![FeatureKey::AiHelp]);
        let refreshed = client.resolve_entitlements(&session.token).unwrap();
        assert_eq!(refreshed.plan, Plan::Free);
        client.logout(&session.token).unwrap();
        let restored = client
            .restore_session(
                Zeroizing::new("restored-session-token".into()),
                4_100_000_000,
            )
            .unwrap();
        assert_eq!(restored.user.id, "u1");

        let requests = transport.requests.lock().unwrap();
        assert_eq!(requests[0].method, "POST");
        assert!(requests[0].url.ends_with("/api/v1/auth/email/start"));
        assert_eq!(
            requests[0].body.as_ref().unwrap()["email"],
            "a@example.test"
        );
        assert_eq!(requests[1].body.as_ref().unwrap()["code"], "246810");
        assert!(requests[1].url.ends_with("/api/v1/auth/email/verify"));
        assert_eq!(requests[2].method, "GET");
        assert!(requests[2].url.ends_with("/api/v1/me"));
        assert_eq!(requests[2].bearer.as_deref(), Some("secret-session-token"));
        assert!(requests[3].url.ends_with("/api/v1/entitlements/resolve"));
        assert_eq!(requests[3].bearer.as_deref(), Some("secret-session-token"));
        assert!(requests[4].url.ends_with("/api/v1/auth/logout"));
        assert!(requests[5].url.ends_with("/api/v1/me"));
        assert_eq!(
            requests[5].bearer.as_deref(),
            Some("restored-session-token")
        );
    }

    #[test]
    fn malformed_grants_auth_failures_and_bad_expiries_do_not_create_access() {
        let unauthorized = MockTransport::with_json(&[("{}", 401)]);
        assert_eq!(
            client(&unauthorized).current_account(&SessionToken::new("token".into())),
            Err(AccountError::Http(401))
        );

        let invalid_plan =
            MockTransport::with_json(&[(r#"{"plan":"future","activeFeatures":["aiHelp"]}"#, 200)]);
        assert_eq!(
            client(&invalid_plan).resolve_entitlements(&SessionToken::new("token".into())),
            Err(AccountError::InvalidReply)
        );

        assert_eq!(parse_rfc3339_unix("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_rfc3339_unix("1970-01-01T01:00:00+01:00"), Some(0));
        assert_eq!(parse_rfc3339_unix("2025-02-29T00:00:00Z"), None);
        assert_eq!(parse_rfc3339_unix("not-a-date"), None);
    }

    #[test]
    fn keyring_session_persistence_keeps_token_out_of_preferences_and_rolls_back() {
        let store = SessionCredentialStore::default();
        let token_ref = crate::assistant::credentials::account_token_reference("default");
        let expiry_ref = crate::assistant::credentials::account_expiry_reference("default");
        let session = AccountSession {
            token: SessionToken::new("fake-account-token".into()),
            user: AccountUser {
                id: "u1".into(),
                email: "a@example.test".into(),
            },
            entitlements: AccountEntitlements {
                plan: Plan::Free,
                email: None,
                active_features: Vec::new(),
                last_synced_at: None,
            },
            expires_at_unix: 1234,
        };
        save_session_credentials(&store, &token_ref, &expiry_ref, &session).unwrap();
        assert_eq!(
            store.get(&token_ref).unwrap().as_str(),
            "fake-account-token"
        );
        assert_eq!(store.get(&expiry_ref).unwrap().as_str(), "1234");
        assert!(!format!("{session:?}").contains("fake-account-token"));
        remove_session_credentials(&store, &token_ref, &expiry_ref);
        assert_eq!(store.get(&token_ref).err(), Some(CredentialError::Missing));
        assert_eq!(store.get(&expiry_ref).err(), Some(CredentialError::Missing));
    }

    #[test]
    fn failed_expiry_storage_rolls_back_the_token_write() {
        struct FailExpiryStore {
            inner: SessionCredentialStore,
            expiry_reference: String,
        }

        impl CredentialStore for FailExpiryStore {
            fn put(&self, reference: &str, value: &str) -> Result<(), CredentialError> {
                if reference == self.expiry_reference {
                    return Err(CredentialError::Unavailable);
                }
                self.inner.put(reference, value)
            }

            fn get(&self, reference: &str) -> Result<Zeroizing<String>, CredentialError> {
                self.inner.get(reference)
            }

            fn delete(&self, reference: &str) -> Result<(), CredentialError> {
                self.inner.delete(reference)
            }
        }

        let token_reference = crate::assistant::credentials::account_token_reference("default");
        let expiry_reference = crate::assistant::credentials::account_expiry_reference("default");
        let store = FailExpiryStore {
            inner: SessionCredentialStore::default(),
            expiry_reference: expiry_reference.clone(),
        };
        let session = AccountSession {
            token: SessionToken::new("temporary-account-token".into()),
            user: AccountUser {
                id: "u1".into(),
                email: "a@example.test".into(),
            },
            entitlements: AccountEntitlements {
                plan: Plan::Free,
                email: None,
                active_features: Vec::new(),
                last_synced_at: None,
            },
            expires_at_unix: 1234,
        };
        assert_eq!(
            save_session_credentials(&store, &token_reference, &expiry_reference, &session),
            Err(CredentialError::Unavailable)
        );
        assert_eq!(
            store.get(&token_reference).err(),
            Some(CredentialError::Missing)
        );
    }

    #[test]
    fn cached_server_grant_closes_at_session_expiry() {
        let active = AccountEntitlements {
            plan: Plan::Free,
            email: Some("a@example.test".into()),
            active_features: vec![FeatureKey::AiHelp],
            last_synced_at: None,
        }
        .server_grant(unix_now().saturating_add(60));
        set_current_entitlement(Some(active));
        assert_eq!(
            current_entitlement().unwrap().active_features,
            vec![FeatureKey::AiHelp]
        );

        let expired = AccountEntitlements {
            plan: Plan::Pro,
            email: None,
            active_features: vec![FeatureKey::AiHelp],
            last_synced_at: None,
        }
        .server_grant(unix_now());
        set_current_entitlement(Some(expired));
        assert_eq!(current_entitlement(), None);
        set_current_entitlement(None);
    }
}
