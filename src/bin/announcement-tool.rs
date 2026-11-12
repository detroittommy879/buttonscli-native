//! Offline authoring utility. Never copy private keys to the web host.
use base64::{engine::general_purpose::STANDARD, Engine};
use buttonscli::announcements::{self, Envelope};
use ed25519_dalek::{Signer, SigningKey};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use zeroize::Zeroizing;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn write_new(path: &Path, bytes: &[u8], secret: bool) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(if secret { 0o600 } else { 0o644 });
    }
    #[cfg(not(unix))]
    let _ = secret;
    let mut file = options
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
}

fn read_bounded(path: &str, limit: usize) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("input exceeds size limit".into());
    }
    Ok(bytes)
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["keygen", private_dir, public_out] => {
            let private_dir = Path::new(private_dir);
            // Require an existing directory, so the operator can restrict its ACL first.
            if !private_dir.is_dir() { return Err("create a private, access-restricted key directory first".into()); }
            if Path::new(public_out).exists() || ["primary-2026.key", "recovery-2026.key"].iter().any(|name| private_dir.join(name).exists()) {
                return Err("key/public output already exists; refusing to replace signing identities".into());
            }
            let mut keys = BTreeMap::new();
            for id in ["primary-2026", "recovery-2026"] {
                let mut seed = Zeroizing::new([0u8; 32]);
                getrandom::fill(seed.as_mut()).map_err(|e| e.to_string())?;
                let key = SigningKey::from_bytes(&seed);
                write_new(&private_dir.join(format!("{id}.key")), seed.as_ref(), true)?;
                let hex: String = key.verifying_key().as_bytes().iter().map(|b| format!("{b:02x}")).collect();
                keys.insert(id, hex);
            }
            write_new(Path::new(public_out), &serde_json::to_vec_pretty(&keys).map_err(|e| e.to_string())?, false)?;
            println!("Created primary/recovery keys in {} and public keys in {public_out}. Back up both keys offline; keep recovery offline. Rebuild the app after installing the public keys in assets/announcement-keys.json.", private_dir.display());
        }
        ["draft", sequence, output] => {
            let sequence: u64 = sequence.parse().map_err(|_| "sequence must be a positive integer")?;
            if sequence == 0 { return Err("sequence must be positive".into()); }
            let issued_at = announcements::now();
            let payload = serde_json::json!({
                "schema":1, "product":"com.buttonscli.native.notices", "sequence":sequence,
                "issued_at":issued_at, "expires_at":issued_at + 7 * 24 * 60 * 60,
                "slots":{
                    "status":{"id":format!("example-status-{sequence}"), "category":"news", "title":"Example developer notice", "url":"https://buttonscli.com"},
                    "popup":{"id":format!("example-popup-{sequence}"), "category":"news", "title":"An example ButtonsCLI notice", "body":"This is a demonstration of signed developer messages. No action is needed. You can close this message or turn off developer notices in Settings → Workspace.", "url":"https://buttonscli.com"},
                    "reserved":null
                }
            });
            write_new(Path::new(output), &serde_json::to_vec_pretty(&payload).map_err(|e| e.to_string())?, false)?;
            println!("Created {output}. Edit its slots before signing; use null for an unused slot.");
        }
        ["sign", key_file, key_id, payload_file, output] => {
            let seed = Zeroizing::new(read_bounded(key_file, 32)?);
            let seed: &[u8; 32] = seed.as_slice().try_into().map_err(|_| "key must be exactly 32 raw bytes")?;
            let key = SigningKey::from_bytes(seed);
            let payload = String::from_utf8(read_bounded(payload_file, announcements::MAX_PAYLOAD_BYTES)?).map_err(|_| "payload must be UTF-8 without BOM")?;
            announcements::validate_payload(&payload, announcements::now())?;
            let signature = key.sign(&announcements::signing_bytes(&payload));
            let envelope = Envelope { key_id: key_id.to_string(), payload, signature: STANDARD.encode(signature.to_bytes()) };
            let bytes = serde_json::to_vec_pretty(&envelope).map_err(|e| e.to_string())?;
            if bytes.len() > announcements::MAX_ENVELOPE_BYTES { return Err("signed envelope exceeds size limit".into()); }
            announcements::verify_document(&bytes, announcements::embedded_keys(), announcements::now())?;
            write_new(Path::new(output), &bytes, false)?;
            println!("Validated and signed {output}. Upload as messages.json.txt, then rename to messages.json to activate.");
        }
        ["verify", file] => {
            let bytes = read_bounded(file, announcements::MAX_ENVELOPE_BYTES)?;
            let feed = announcements::verify_document(&bytes, announcements::embedded_keys(), announcements::now())?;
            println!("Valid signature and payload: sequence {}, expires {}.", feed.sequence, feed.expires_at);
        }
        _ => return Err("Usage:\n  announcement-tool keygen <existing-private-directory> <new-public-keys.json>\n  announcement-tool draft <sequence> <new-payload.json>\n  announcement-tool sign <raw-key-file> <key-id> <payload.json> <new-output.json.txt>\n  announcement-tool verify <envelope.json[.txt]>".into()),
    }
    Ok(())
}
