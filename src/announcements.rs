//! Signed, bounded developer notices. This channel never executes code or installs updates.
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const FEED_URL: &str = "https://buttonscli.com/app-notices/messages.json";
pub const MAX_ENVELOPE_BYTES: usize = 64 * 1024;
pub const MAX_PAYLOAD_BYTES: usize = 24 * 1024;
pub const SIGNING_DOMAIN: &[u8] = b"ButtonsCLI announcements v1\0";
const MAX_LIFETIME: u64 = 30 * 24 * 60 * 60;
const CLOCK_SKEW: u64 = 300;
const TRUSTED_KEYS: &str = include_str!("../assets/announcement-keys.json");

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub enabled: bool,
    pub promotions: bool,
    pub last_sequence: u64,
    pub last_digest: String,
    pub dismissed: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub key_id: String,
    /// Exact UTF-8 payload bytes are signed; no JSON canonicalization dependency.
    pub payload: String,
    pub signature: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Feed {
    pub schema: u32,
    pub product: String,
    pub sequence: u64,
    pub issued_at: u64,
    pub expires_at: u64,
    #[serde(default)]
    pub slots: Slots,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Slots {
    pub status: Option<Message>,
    pub popup: Option<Message>,
    /// Reserved data is signed and bounded, but never rendered or interpreted.
    pub reserved: Option<serde_json::Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Security,
    News,
    Promotion,
}

impl Category {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Security => "Security notice",
            Self::News => "Product news",
            Self::Promotion => "Promotion",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub id: String,
    pub category: Category,
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub min_version: Option<String>,
    #[serde(default)]
    pub max_version: Option<String>,
    #[serde(default)]
    pub platforms: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct VerifiedFeed {
    pub feed: Feed,
    pub digest: String,
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

pub fn signing_bytes(payload: &str) -> Vec<u8> {
    [SIGNING_DOMAIN, payload.as_bytes()].concat()
}

pub fn decode_key(hex: &str) -> Result<[u8; 32], String> {
    if hex.len() != 64 || !hex.is_ascii() {
        return Err("expected a 32-byte hex key".into());
    }
    let mut bytes = [0; 32];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|_| "invalid hex key")?;
    }
    Ok(bytes)
}

pub fn embedded_keys() -> &'static str {
    TRUSTED_KEYS
}

pub fn validate_payload(payload: &str, at: u64) -> Result<Feed, String> {
    if payload.len() > MAX_PAYLOAD_BYTES {
        return Err("payload exceeds 24 KiB".into());
    }
    let feed: Feed = serde_json::from_str(payload).map_err(|_| "invalid payload JSON/schema")?;
    if feed.schema != 1 || feed.product != "com.buttonscli.native.notices" || feed.sequence == 0 {
        return Err("wrong schema, product or sequence".into());
    }
    if feed.issued_at > at.saturating_add(CLOCK_SKEW)
        || feed.expires_at <= at
        || feed.expires_at <= feed.issued_at
        || feed.expires_at - feed.issued_at > MAX_LIFETIME
    {
        return Err("notice is expired, future-dated or lasts over 30 days".into());
    }
    for (message, status) in [(&feed.slots.status, true), (&feed.slots.popup, false)] {
        if let Some(message) = message {
            validate_message(message, status)?;
        }
    }
    if let (Some(status), Some(popup)) = (&feed.slots.status, &feed.slots.popup) {
        if status.id == popup.id {
            return Err("each slot needs a unique message id".into());
        }
    }
    Ok(feed)
}

fn text_valid(text: &str, limit: usize, multiline: bool) -> bool {
    text.chars().count() <= limit
        && !text.chars().any(|c| {
            (c.is_control() && !(multiline && matches!(c, '\n' | '\t')))
                || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
}

fn validate_message(message: &Message, status: bool) -> Result<(), String> {
    if message.id.is_empty()
        || message.id.len() > 64
        || !message
            .id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        || message.title.trim().is_empty()
        || !text_valid(&message.title, if status { 160 } else { 96 }, false)
        || !text_valid(&message.body, if status { 0 } else { 4096 }, true)
    {
        return Err("invalid id or text limit/control character".into());
    }
    if let Some(url) = &message.url {
        let parsed = url::Url::parse(url).map_err(|_| "invalid link")?;
        if url.len() > 2048
            || !text_valid(url, 2048, false)
            || parsed.scheme() != "https"
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
        {
            return Err("links must be HTTPS without credentials, at most 2048 bytes".into());
        }
    }
    let parse_version =
        |text: &str| semver::Version::parse(text).map_err(|_| "invalid version range");
    let min = message
        .min_version
        .as_deref()
        .map(parse_version)
        .transpose()?;
    let max = message
        .max_version
        .as_deref()
        .map(parse_version)
        .transpose()?;
    if min
        .as_ref()
        .zip(max.as_ref())
        .is_some_and(|(min, max)| min > max)
        || message.platforms.len() > 3
        || message
            .platforms
            .iter()
            .any(|p| !matches!(p.as_str(), "windows" | "linux" | "macos"))
    {
        return Err("invalid version/platform targeting".into());
    }
    Ok(())
}

pub fn verify_document(bytes: &[u8], keys: &str, at: u64) -> Result<Feed, String> {
    verify(bytes, keys, at).map(|verified| verified.feed)
}

fn verify(bytes: &[u8], keys: &str, at: u64) -> Result<VerifiedFeed, String> {
    if bytes.len() > MAX_ENVELOPE_BYTES || keys.len() > 4096 {
        return Err("document too large".into());
    }
    let envelope: Envelope = serde_json::from_slice(bytes).map_err(|_| "invalid envelope")?;
    if envelope.payload.len() > MAX_PAYLOAD_BYTES || envelope.signature.len() != 88 {
        return Err("invalid payload/signature size".into());
    }
    let keys: BTreeMap<String, String> =
        serde_json::from_str(keys).map_err(|_| "invalid trust store")?;
    if keys.len() > 4 {
        return Err("too many trusted keys".into());
    }
    let key = keys.get(&envelope.key_id).ok_or("unknown signing key")?;
    let key = VerifyingKey::from_bytes(&decode_key(key)?).map_err(|_| "invalid public key")?;
    let signature = STANDARD
        .decode(envelope.signature)
        .map_err(|_| "invalid signature encoding")?;
    let signature = Signature::from_slice(&signature).map_err(|_| "invalid signature")?;
    key.verify_strict(&signing_bytes(&envelope.payload), &signature)
        .map_err(|_| "untrusted signature")?;
    // Only parse the untrusted payload after authenticating its exact bytes.
    let feed = validate_payload(&envelope.payload, at)?;
    let digest = format!("{:x}", Sha256::digest(envelope.payload.as_bytes()));
    Ok(VerifiedFeed { feed, digest })
}

impl Preferences {
    /// Reverting UI settings must never undo an authenticated sequence or dismissal.
    pub(crate) fn retain_history_from(&mut self, current: &Self) {
        self.last_sequence = current.last_sequence;
        self.last_digest.clone_from(&current.last_digest);
        self.dismissed.clone_from(&current.dismissed);
    }

    pub(crate) fn accept(&mut self, verified: &VerifiedFeed) -> bool {
        let sequence = verified.feed.sequence;
        if sequence < self.last_sequence
            || (sequence == self.last_sequence && self.last_digest != verified.digest)
        {
            return false;
        }
        self.last_sequence = sequence;
        self.last_digest.clone_from(&verified.digest);
        true
    }

    pub(crate) fn dismiss(&mut self, id: &str) {
        if !self.dismissed.iter().any(|saved| saved == id) {
            self.dismissed.push(id.to_owned());
        }
        if self.dismissed.len() > 128 {
            self.dismissed.drain(..self.dismissed.len() - 128);
        }
    }

    pub(crate) fn shows(&self, message: &Message, version: &str, platform: &str) -> bool {
        if !self.enabled
            || (message.category == Category::Promotion && !self.promotions)
            || self.dismissed.contains(&message.id)
            || (!message.platforms.is_empty() && !message.platforms.iter().any(|p| p == platform))
        {
            return false;
        }
        let Ok(mut version) = semver::Version::parse(version) else {
            return false;
        };
        version.build = semver::BuildMetadata::EMPTY;
        message
            .min_version
            .as_ref()
            .is_none_or(|min| semver::Version::parse(min).is_ok_and(|min| version >= min))
            && message
                .max_version
                .as_ref()
                .is_none_or(|max| semver::Version::parse(max).is_ok_and(|max| version <= max))
    }
}

pub(crate) fn spawn(ctx: egui::Context) -> Option<Receiver<VerifiedFeed>> {
    if std::env::var("BUTTONSCLI_NO_ANNOUNCEMENTS").is_ok_and(|v| v == "1") {
        return None;
    }
    let (sender, receiver) = channel();
    std::thread::Builder::new()
        .name("signed-notices".into())
        .spawn(move || {
            let result = (|| -> Result<VerifiedFeed, String> {
                let client = reqwest::blocking::Client::builder()
                    .timeout(Duration::from_secs(15))
                    .connect_timeout(Duration::from_secs(5))
                    .redirect(reqwest::redirect::Policy::none())
                    .https_only(true)
                    .user_agent("ButtonsCLI-signed-notices")
                    .build()
                    .map_err(|_| "HTTP setup failed")?;
                let response = client
                    .get(FEED_URL)
                    .send()
                    .map_err(|_| "notice request failed")?;
                if response.status() != reqwest::StatusCode::OK {
                    return Err("feed unavailable".into());
                }
                if response
                    .content_length()
                    .is_some_and(|len| len > MAX_ENVELOPE_BYTES as u64)
                {
                    return Err("feed too large".into());
                }
                let mut bytes = Vec::new();
                response
                    .take(MAX_ENVELOPE_BYTES as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|_| "feed read failed")?;
                verify(&bytes, TRUSTED_KEYS, now())
            })();
            match result {
                Ok(feed) => {
                    let _ = sender.send(feed);
                    ctx.request_repaint();
                }
                Err(reason) => log::debug!("signed notices ignored: {reason}"),
            }
        })
        .ok()
        .map(|_| receiver)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::{json, Value};

    fn payload() -> Value {
        json!({"schema":1, "product":"com.buttonscli.native.notices", "sequence":7,
            "issued_at":1000, "expires_at":2000, "slots":{
                "status":{"id":"security-1", "category":"security", "title":"Please update"},
                "popup":{"id":"news-1", "category":"news", "title":"A developer message", "body":"Plain text only"},
                "reserved":{"future":"inert"}}})
    }

    fn signed(value: &Value, key_id: &str, seed: u8) -> (Vec<u8>, String) {
        let key = SigningKey::from_bytes(&[seed; 32]);
        let payload = value.to_string();
        let signature = STANDARD.encode(key.sign(&signing_bytes(&payload)).to_bytes());
        let envelope = Envelope {
            key_id: key_id.into(),
            payload,
            signature,
        };
        let public: String = key
            .verifying_key()
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        (
            serde_json::to_vec(&envelope).unwrap(),
            json!({key_id:public}).to_string(),
        )
    }

    #[test]
    fn accepts_primary_and_recovery_but_rejects_tampering_and_unknown_keys() {
        for id in ["primary", "recovery"] {
            let (bytes, keys) = signed(&payload(), id, 42);
            assert!(verify(&bytes, &keys, 1500).is_ok());
            let mut envelope: Envelope = serde_json::from_slice(&bytes).unwrap();
            envelope.payload = envelope
                .payload
                .replace("Please update", "Please install malware");
            assert!(verify(&serde_json::to_vec(&envelope).unwrap(), &keys, 1500).is_err());
            assert!(verify(&bytes, "{}", 1500).is_err());
            let (_, other_keys) = signed(&payload(), id, 43);
            assert!(verify(&bytes, &other_keys, 1500).is_err());
        }
        // A release signature (or raw Ed25519 signature) cannot be reused here.
        let (bytes, keys) = signed(&payload(), "primary", 42);
        let mut envelope: Envelope = serde_json::from_slice(&bytes).unwrap();
        envelope.signature = STANDARD.encode(
            SigningKey::from_bytes(&[42; 32])
                .sign(envelope.payload.as_bytes())
                .to_bytes(),
        );
        assert!(verify(&serde_json::to_vec(&envelope).unwrap(), &keys, 1500).is_err());
    }

    #[test]
    fn rejects_expiry_future_dates_wrong_product_and_excessive_lifetimes() {
        let (bytes, keys) = signed(&payload(), "primary", 42);
        assert!(verify(&bytes, &keys, 2000).is_err());
        assert!(verify(&bytes, &keys, 699).is_err());
        for (field, value) in [
            ("schema", json!(2)),
            ("product", json!("other")),
            ("sequence", json!(0)),
            ("expires_at", json!(1000 + MAX_LIFETIME + 1)),
            ("issued_at", json!(2000)),
        ] {
            let mut p = payload();
            p[field] = value;
            let (bytes, keys) = signed(&p, "primary", 42);
            assert!(verify(&bytes, &keys, 1500).is_err(), "{field}");
        }
    }

    #[test]
    fn bounded_text_links_and_reserved_data_fail_closed() {
        let mut p = payload();
        p["slots"]["popup"]["body"] = json!("字".repeat(4096));
        assert!(
            validate_payload(&p.to_string(), 1500).is_ok(),
            "limits count Unicode characters"
        );
        p["slots"]["popup"]["body"] = json!("a".repeat(4097));
        assert!(validate_payload(&p.to_string(), 1500).is_err());
        for url in [
            "javascript:alert(1)",
            "http://buttonscli.com",
            "file:///a",
            "https://user:password@buttonscli.com",
            &format!("https://buttonscli.com/{}", "a".repeat(2048)),
        ] {
            let mut p = payload();
            p["slots"]["popup"]["url"] = json!(url);
            assert!(validate_payload(&p.to_string(), 1500).is_err());
        }
        for text in ["x\u{1b}[2J", "x\u{202e}exe", "line\nbreak", ""] {
            let mut p = payload();
            p["slots"]["status"]["title"] = json!(text);
            assert!(validate_payload(&p.to_string(), 1500).is_err());
        }
        let mut p = payload();
        p["slots"]["status"]["title"] = json!("a".repeat(161));
        assert!(validate_payload(&p.to_string(), 1500).is_err());
        let mut p = payload();
        p["slots"]["reserved"] = json!("a".repeat(MAX_PAYLOAD_BYTES));
        assert!(validate_payload(&p.to_string(), 1500).is_err());
        assert!(verify(&vec![b' '; MAX_ENVELOPE_BYTES + 1], "{}", 1500).is_err());
        assert!(verify(br#"{"payload":"broken"}"#, "{}", 1500).is_err());
        let mut p = payload();
        p["slots"]["popup"]["id"] = json!("security-1");
        assert!(validate_payload(&p.to_string(), 1500).is_err());
    }

    #[test]
    fn replay_state_survives_serialization_and_allows_only_identical_same_sequence() {
        let (bytes, keys) = signed(&payload(), "primary", 42);
        let feed = verify(&bytes, &keys, 1500).unwrap();
        let mut prefs = Preferences::default();
        assert!(prefs.accept(&feed));
        prefs = serde_json::from_str(&serde_json::to_string(&prefs).unwrap()).unwrap();
        assert!(prefs.accept(&feed));
        let mut other = feed.clone();
        other.digest = "changed".into();
        assert!(!prefs.accept(&other));
        other.feed.sequence = 6;
        assert!(!prefs.accept(&other));
        other.feed.sequence = 8;
        assert!(prefs.accept(&other));
    }

    #[test]
    fn opt_in_promotions_targeting_and_dismissal_are_independent() {
        let feed = validate_payload(&payload().to_string(), 1500).unwrap();
        let mut message = feed.slots.popup.unwrap();
        let mut prefs = Preferences::default();
        assert!(!prefs.shows(&message, "1.1.0", "windows"));
        prefs.enabled = true;
        assert!(prefs.shows(&message, "1.1.0", "windows"));
        message.category = Category::Promotion;
        assert!(!prefs.shows(&message, "1.1.0", "windows"));
        prefs.promotions = true;
        message.min_version = Some("1.1.0".into());
        message.max_version = Some("1.2.0".into());
        message.platforms = vec!["windows".into()];
        assert!(prefs.shows(&message, "1.1.0+abcdef", "windows"));
        assert!(!prefs.shows(&message, "1.0.0", "windows"));
        assert!(!prefs.shows(&message, "1.3.0", "windows"));
        assert!(!prefs.shows(&message, "1.1.0", "linux"));
        prefs.dismiss(&message.id);
        assert!(!prefs.shows(&message, "1.1.0", "windows"));
        for i in 0..200 {
            prefs.dismiss(&format!("message-{i}"));
        }
        assert_eq!(prefs.dismissed.len(), 128);
    }
}
