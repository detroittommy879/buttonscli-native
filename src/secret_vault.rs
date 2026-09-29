//! Internal, profile-scoped encrypted snippets. Secrets never enter settings
//! JSON, imports, analytics, or the normal terminal-input observer.

use std::collections::HashSet;
use std::fmt;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

use crate::storage::store::{NativeStore, StoreError};

const MAGIC: &[u8; 8] = b"BCLSVLT1";
const HEADER_BYTES: usize = 8 + 16 + 12;
const TAG_BYTES: usize = 16;
const MAX_FILE_BYTES: usize = 1024 * 1024;
const MAX_PLAINTEXT_BYTES: usize = MAX_FILE_BYTES - HEADER_BYTES - TAG_BYTES;
const MIN_PASSPHRASE_CHARS: usize = 12;
const MAX_PASSPHRASE_BYTES: usize = 1024;
const MAX_ENTRIES: usize = 256;
const MAX_LABEL_CHARS: usize = 128;
const MAX_SECRET_BYTES: usize = 8192;
const ARGON_MEMORY_KIB: u32 = 64 * 1024;
const ARGON_ITERATIONS: u32 = 3;
const ARGON_LANES: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VaultError {
    PassphraseTooShort,
    PassphraseTooLong,
    UnlockFailed,
    AlreadyExists,
    Missing,
    Corrupt,
    UnsupportedVersion,
    TooLarge,
    InvalidEntry,
    TooManyEntries,
    Conflict,
    InvalidProfile,
    Locked,
    Storage,
    Crypto,
}

impl fmt::Display for VaultError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::PassphraseTooShort => "Use a passphrase with at least 12 characters.",
            Self::PassphraseTooLong => "The passphrase exceeds the 1024-byte limit.",
            Self::UnlockFailed => "The passphrase did not unlock this vault.",
            Self::AlreadyExists => "A Quick Secrets vault already exists in this profile.",
            Self::Missing => "No Quick Secrets vault exists in this profile.",
            Self::Corrupt => "The Quick Secrets vault is damaged and was left unchanged.",
            Self::UnsupportedVersion => "This Quick Secrets vault version is not supported.",
            Self::TooLarge => "The Quick Secrets vault exceeds its 1 MiB limit.",
            Self::InvalidEntry => "The entry needs a label and a single-line secret.",
            Self::TooManyEntries => "The vault can contain up to 256 entries.",
            Self::Conflict => "The vault changed in another app window. Lock and unlock it again.",
            Self::InvalidProfile => "The active profile cannot use a Quick Secrets vault.",
            Self::Locked => "Unlock the Quick Secrets vault first.",
            Self::Storage => "The Quick Secrets vault could not be read or saved.",
            Self::Crypto => "The Quick Secrets cryptographic operation failed.",
        })
    }
}

impl std::error::Error for VaultError {}

pub(crate) trait VaultRepository: Send + Sync {
    fn read(&self) -> Result<Option<Vec<u8>>, VaultError>;
    fn compare_and_store(
        &self,
        expected_digest: Option<[u8; 32]>,
        ciphertext: &[u8],
    ) -> Result<[u8; 32], VaultError>;
    fn delete_if_unchanged(&self, expected_digest: [u8; 32]) -> Result<(), VaultError>;
}

impl VaultRepository for NativeStore {
    fn read(&self) -> Result<Option<Vec<u8>>, VaultError> {
        self.read_quick_secrets().map_err(map_store_error)
    }

    fn compare_and_store(
        &self,
        expected_digest: Option<[u8; 32]>,
        ciphertext: &[u8],
    ) -> Result<[u8; 32], VaultError> {
        self.save_quick_secrets(expected_digest, ciphertext)
            .map_err(|error| match (expected_digest, error) {
                (None, StoreError::StaleRevision) => VaultError::AlreadyExists,
                (_, error) => map_store_error(error),
            })
    }

    fn delete_if_unchanged(&self, expected_digest: [u8; 32]) -> Result<(), VaultError> {
        self.delete_quick_secrets(expected_digest)
            .map_err(map_store_error)
    }
}

fn map_store_error(error: StoreError) -> VaultError {
    match error {
        StoreError::StaleRevision => VaultError::Conflict,
        StoreError::TooLarge => VaultError::TooLarge,
        StoreError::InvalidProfile => VaultError::InvalidProfile,
        _ => VaultError::Storage,
    }
}

pub(crate) struct SecretEntry {
    pub(crate) id: String,
    pub(crate) label: String,
    secret: Zeroizing<String>,
    pub(crate) created_at_ms: u64,
}

impl fmt::Debug for SecretEntry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SecretEntry")
            .field("id", &self.id)
            .field("label", &self.label)
            .field("secret", &"[redacted]")
            .field("created_at_ms", &self.created_at_ms)
            .finish()
    }
}

#[derive(Deserialize)]
struct EntryWire {
    id: String,
    label: String,
    secret: String,
    created_at_ms: u64,
}

impl<'de> Deserialize<'de> for SecretEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut wire = EntryWire::deserialize(deserializer)?;
        let secret = Zeroizing::new(std::mem::take(&mut wire.secret));
        wire.secret.zeroize();
        Ok(Self {
            id: wire.id,
            label: wire.label,
            secret,
            created_at_ms: wire.created_at_ms,
        })
    }
}

#[derive(Deserialize)]
struct VaultPayload {
    version: u8,
    entries: Vec<SecretEntry>,
}

#[derive(Serialize)]
struct VaultPayloadRef<'a> {
    version: u8,
    entries: Vec<EntryRef<'a>>,
}

#[derive(Serialize)]
struct EntryRef<'a> {
    id: &'a str,
    label: &'a str,
    secret: &'a str,
    created_at_ms: u64,
}

pub(crate) struct VaultSession {
    profile: String,
    salt: [u8; 16],
    key: Zeroizing<[u8; 32]>,
    expected_digest: Option<[u8; 32]>,
    entries: Vec<SecretEntry>,
    last_activity: Instant,
    locked: bool,
}

impl fmt::Debug for VaultSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VaultSession")
            .field("profile", &self.profile)
            .field("entry_count", &self.entries.len())
            .field("key", &"[redacted]")
            .field("last_activity", &self.last_activity)
            .field("locked", &self.locked)
            .finish()
    }
}

impl VaultSession {
    pub(crate) fn create(
        repository: &impl VaultRepository,
        profile: &str,
        passphrase: Zeroizing<String>,
    ) -> Result<Self, VaultError> {
        validate_passphrase(&passphrase)?;
        validate_profile(profile)?;
        let mut salt = [0_u8; 16];
        getrandom::fill(&mut salt).map_err(|_| VaultError::Crypto)?;
        let key = derive_key(&passphrase, &salt)?;
        let mut session = Self {
            profile: profile.to_owned(),
            salt,
            key,
            expected_digest: None,
            entries: Vec::new(),
            last_activity: Instant::now(),
            locked: false,
        };
        session.persist(repository)?;
        Ok(session)
    }

    pub(crate) fn unlock(
        repository: &impl VaultRepository,
        profile: &str,
        passphrase: Zeroizing<String>,
    ) -> Result<Self, VaultError> {
        validate_passphrase_size(&passphrase)?;
        validate_profile(profile)?;
        let ciphertext = repository.read()?.ok_or(VaultError::Missing)?;
        if ciphertext.len() > MAX_FILE_BYTES {
            return Err(VaultError::TooLarge);
        }
        if ciphertext.len() < HEADER_BYTES + TAG_BYTES {
            return Err(VaultError::Corrupt);
        }
        if ciphertext.get(..MAGIC.len()) != Some(MAGIC.as_slice()) {
            return Err(VaultError::UnsupportedVersion);
        }

        let salt: [u8; 16] = ciphertext[8..24]
            .try_into()
            .map_err(|_| VaultError::Corrupt)?;
        let nonce_bytes: [u8; 12] = ciphertext[24..36]
            .try_into()
            .map_err(|_| VaultError::Corrupt)?;
        let nonce = Nonce::from(nonce_bytes);
        let key = derive_key(&passphrase, &salt)?;
        let cipher = Aes256Gcm::new_from_slice(&key[..]).map_err(|_| VaultError::Crypto)?;
        let aad = associated_data(profile)?;
        let plaintext = Zeroizing::new(
            cipher
                .decrypt(
                    &nonce,
                    Payload {
                        msg: &ciphertext[HEADER_BYTES..],
                        aad: &aad,
                    },
                )
                .map_err(|_| VaultError::UnlockFailed)?,
        );
        if plaintext.len() > MAX_PLAINTEXT_BYTES {
            return Err(VaultError::TooLarge);
        }
        let payload: VaultPayload =
            serde_json::from_slice(&plaintext).map_err(|_| VaultError::Corrupt)?;
        if payload.version != 1 {
            return Err(VaultError::UnsupportedVersion);
        }
        validate_entries(&payload.entries)?;

        Ok(Self {
            profile: profile.to_owned(),
            salt,
            key,
            expected_digest: Some(digest(&ciphertext)),
            entries: payload.entries,
            last_activity: Instant::now(),
            locked: false,
        })
    }

    pub(crate) fn profile(&self) -> &str {
        &self.profile
    }

    pub(crate) fn entries(&self) -> &[SecretEntry] {
        &self.entries
    }

    pub(crate) fn add(
        &mut self,
        repository: &impl VaultRepository,
        label: &str,
        secret: &str,
    ) -> Result<(), VaultError> {
        self.ensure_unlocked()?;
        let label = label.trim();
        if !valid_entry_text(label, secret) {
            return Err(VaultError::InvalidEntry);
        }
        if self.entries.len() >= MAX_ENTRIES {
            return Err(VaultError::TooManyEntries);
        }
        let mut id_bytes = [0_u8; 16];
        getrandom::fill(&mut id_bytes).map_err(|_| VaultError::Crypto)?;
        let id = id_bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if self.entries.iter().any(|entry| entry.id == id) {
            return Err(VaultError::Crypto);
        }
        self.entries.push(SecretEntry {
            id,
            label: label.to_owned(),
            secret: Zeroizing::new(secret.to_owned()),
            created_at_ms: now_ms(),
        });
        if let Err(error) = self.persist(repository) {
            self.entries.pop();
            return Err(error);
        }
        self.touch();
        Ok(())
    }

    pub(crate) fn remove(
        &mut self,
        repository: &impl VaultRepository,
        id: &str,
    ) -> Result<(), VaultError> {
        self.ensure_unlocked()?;
        let Some(index) = self.entries.iter().position(|entry| entry.id == id) else {
            return Err(VaultError::InvalidEntry);
        };
        let removed = self.entries.remove(index);
        if let Err(error) = self.persist(repository) {
            self.entries.insert(index, removed);
            return Err(error);
        }
        drop(removed);
        self.touch();
        Ok(())
    }

    pub(crate) fn prepare_send(
        &mut self,
        id: &str,
    ) -> Result<egui_term::SensitiveInput, VaultError> {
        self.ensure_unlocked()?;
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .ok_or(VaultError::InvalidEntry)?;
        let bytes = entry.secret.as_bytes().to_vec();
        self.touch();
        Ok(egui_term::SensitiveInput::new(bytes))
    }

    pub(crate) fn idle_deadline(&self, auto_lock_minutes: u32) -> Option<Instant> {
        (auto_lock_minutes > 0)
            .then(|| self.last_activity + Duration::from_secs(u64::from(auto_lock_minutes) * 60))
    }

    pub(crate) fn expire_if_idle(&mut self, now: Instant, auto_lock_minutes: u32) -> bool {
        if self
            .idle_deadline(auto_lock_minutes)
            .is_some_and(|deadline| now >= deadline)
        {
            self.lock();
            true
        } else {
            false
        }
    }

    pub(crate) fn forget(mut self, repository: &impl VaultRepository) -> Result<(), VaultError> {
        self.ensure_unlocked()?;
        let digest = self.expected_digest.ok_or(VaultError::Missing)?;
        repository.delete_if_unchanged(digest)?;
        self.lock();
        Ok(())
    }

    pub(crate) fn lock(&mut self) {
        self.key.zeroize();
        self.entries.clear();
        self.expected_digest = None;
        self.locked = true;
    }

    fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    fn ensure_unlocked(&self) -> Result<(), VaultError> {
        if self.locked {
            Err(VaultError::Locked)
        } else {
            Ok(())
        }
    }

    fn persist(&mut self, repository: &impl VaultRepository) -> Result<(), VaultError> {
        self.ensure_unlocked()?;
        let payload = VaultPayloadRef {
            version: 1,
            entries: self
                .entries
                .iter()
                .map(|entry| EntryRef {
                    id: &entry.id,
                    label: &entry.label,
                    secret: entry.secret.as_str(),
                    created_at_ms: entry.created_at_ms,
                })
                .collect(),
        };
        let plaintext =
            Zeroizing::new(serde_json::to_vec(&payload).map_err(|_| VaultError::Corrupt)?);
        if plaintext.len() > MAX_PLAINTEXT_BYTES {
            return Err(VaultError::TooLarge);
        }
        let mut nonce_bytes = [0_u8; 12];
        getrandom::fill(&mut nonce_bytes).map_err(|_| VaultError::Crypto)?;
        let cipher = Aes256Gcm::new_from_slice(&self.key[..]).map_err(|_| VaultError::Crypto)?;
        let aad = associated_data(&self.profile)?;
        let nonce = Nonce::from(nonce_bytes);
        let encrypted = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: &plaintext,
                    aad: &aad,
                },
            )
            .map_err(|_| VaultError::Crypto)?;
        if encrypted.len() + HEADER_BYTES > MAX_FILE_BYTES {
            return Err(VaultError::TooLarge);
        }
        let mut file = Vec::with_capacity(HEADER_BYTES + encrypted.len());
        file.extend_from_slice(MAGIC);
        file.extend_from_slice(&self.salt);
        file.extend_from_slice(&nonce_bytes);
        file.extend_from_slice(&encrypted);
        self.expected_digest = Some(repository.compare_and_store(self.expected_digest, &file)?);
        Ok(())
    }
}

fn validate_passphrase(passphrase: &str) -> Result<(), VaultError> {
    validate_passphrase_size(passphrase)?;
    if passphrase.chars().count() < MIN_PASSPHRASE_CHARS {
        Err(VaultError::PassphraseTooShort)
    } else {
        Ok(())
    }
}

fn validate_passphrase_size(passphrase: &str) -> Result<(), VaultError> {
    if passphrase.is_empty() || passphrase.len() > MAX_PASSPHRASE_BYTES {
        Err(VaultError::PassphraseTooLong)
    } else {
        Ok(())
    }
}

fn validate_profile(profile: &str) -> Result<(), VaultError> {
    if profile.is_empty()
        || profile.len() > 256
        || profile.chars().any(char::is_control)
        || profile.contains(['/', '\\'])
    {
        Err(VaultError::InvalidProfile)
    } else {
        Ok(())
    }
}

fn valid_entry_text(label: &str, secret: &str) -> bool {
    !label.is_empty()
        && label.chars().count() <= MAX_LABEL_CHARS
        && !label.chars().any(char::is_control)
        && !secret.is_empty()
        && secret.len() <= MAX_SECRET_BYTES
        && !secret
            .chars()
            .any(|character| matches!(character, '\0' | '\r' | '\n'))
}

fn validate_entries(entries: &[SecretEntry]) -> Result<(), VaultError> {
    if entries.len() > MAX_ENTRIES {
        return Err(VaultError::TooManyEntries);
    }
    let mut ids = HashSet::with_capacity(entries.len());
    for entry in entries {
        if !valid_entry_text(&entry.label, &entry.secret)
            || entry.id.is_empty()
            || entry.id.len() > 128
            || entry.id.chars().any(char::is_control)
            || !ids.insert(entry.id.as_str())
        {
            return Err(VaultError::Corrupt);
        }
    }
    Ok(())
}

fn derive_key(passphrase: &str, salt: &[u8; 16]) -> Result<Zeroizing<[u8; 32]>, VaultError> {
    let params = Params::new(ARGON_MEMORY_KIB, ARGON_ITERATIONS, ARGON_LANES, Some(32))
        .map_err(|_| VaultError::Crypto)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0_u8; 32]);
    argon
        .hash_password_into(passphrase.as_bytes(), salt, &mut *key)
        .map_err(|_| VaultError::Crypto)?;
    Ok(key)
}

fn associated_data(profile: &str) -> Result<Vec<u8>, VaultError> {
    validate_profile(profile)?;
    let length = u32::try_from(profile.len()).map_err(|_| VaultError::InvalidProfile)?;
    let mut aad = Vec::with_capacity(MAGIC.len() + 4 + profile.len());
    aad.extend_from_slice(MAGIC);
    aad.extend_from_slice(&length.to_le_bytes());
    aad.extend_from_slice(profile.as_bytes());
    Ok(aad)
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryVaultRepository {
        bytes: Mutex<Option<Vec<u8>>>,
    }

    impl VaultRepository for MemoryVaultRepository {
        fn read(&self) -> Result<Option<Vec<u8>>, VaultError> {
            Ok(self.bytes.lock().unwrap().clone())
        }

        fn compare_and_store(
            &self,
            expected_digest: Option<[u8; 32]>,
            ciphertext: &[u8],
        ) -> Result<[u8; 32], VaultError> {
            let mut bytes = self.bytes.lock().unwrap();
            let current_digest = bytes.as_deref().map(digest);
            if current_digest != expected_digest {
                return Err(if bytes.is_some() && expected_digest.is_none() {
                    VaultError::AlreadyExists
                } else {
                    VaultError::Conflict
                });
            }
            *bytes = Some(ciphertext.to_vec());
            Ok(digest(ciphertext))
        }

        fn delete_if_unchanged(&self, expected_digest: [u8; 32]) -> Result<(), VaultError> {
            let mut bytes = self.bytes.lock().unwrap();
            if bytes.as_deref().map(digest) != Some(expected_digest) {
                return Err(VaultError::Conflict);
            }
            *bytes = None;
            Ok(())
        }
    }

    fn passphrase(value: &str) -> Zeroizing<String> {
        Zeroizing::new(value.to_owned())
    }

    #[test]
    fn encrypts_entries_and_wrong_passphrase_leaves_original_bytes_unchanged() {
        let repository = MemoryVaultRepository::default();
        let mut vault = VaultSession::create(
            &repository,
            "default",
            passphrase("correct horse battery staple"),
        )
        .unwrap();
        vault
            .add(&repository, "Build token", "s3cr3t-value")
            .unwrap();
        let encrypted_before = repository.read().unwrap().unwrap();
        assert!(!encrypted_before
            .windows(b"s3cr3t-value".len())
            .any(|part| part == b"s3cr3t-value"));

        let wrong = VaultSession::unlock(
            &repository,
            "default",
            passphrase("incorrect horse battery staple"),
        );
        assert_eq!(wrong.unwrap_err(), VaultError::UnlockFailed);
        assert_eq!(repository.read().unwrap().unwrap(), encrypted_before);

        let unlocked = VaultSession::unlock(
            &repository,
            "default",
            passphrase("correct horse battery staple"),
        )
        .unwrap();
        assert_eq!(unlocked.entries()[0].label, "Build token");
        assert_eq!(unlocked.entries()[0].secret.as_str(), "s3cr3t-value");
        assert!(format!("{:?}", unlocked.entries()[0]).contains("[redacted]"));
    }

    #[test]
    fn profile_binding_and_corrupt_file_recovery_are_fail_closed() {
        let repository = MemoryVaultRepository::default();
        let _vault = VaultSession::create(
            &repository,
            "profile-one",
            passphrase("correct horse battery staple"),
        )
        .unwrap();
        let saved = repository.read().unwrap().unwrap();
        assert_eq!(
            VaultSession::unlock(
                &repository,
                "profile-two",
                passphrase("correct horse battery staple"),
            )
            .unwrap_err(),
            VaultError::UnlockFailed
        );

        let mut damaged = saved.clone();
        *damaged.last_mut().unwrap() ^= 0x40;
        *repository.bytes.lock().unwrap() = Some(damaged.clone());
        assert_eq!(
            VaultSession::unlock(
                &repository,
                "profile-one",
                passphrase("correct horse battery staple"),
            )
            .unwrap_err(),
            VaultError::UnlockFailed
        );
        assert_eq!(repository.read().unwrap().unwrap(), damaged);
    }

    #[test]
    fn auto_lock_and_explicit_forget_clear_state_without_password_recovery() {
        let repository = Arc::new(MemoryVaultRepository::default());
        let mut vault = VaultSession::create(
            repository.as_ref(),
            "default",
            passphrase("correct horse battery staple"),
        )
        .unwrap();
        vault
            .add(repository.as_ref(), "token", "local-secret")
            .unwrap();
        let deadline = vault.idle_deadline(15).unwrap();
        assert!(vault.idle_deadline(0).is_none());
        assert!(!vault.expire_if_idle(deadline - Duration::from_secs(1), 15));
        assert!(vault.expire_if_idle(deadline, 15));
        assert!(vault.entries().is_empty());

        let vault = VaultSession::unlock(
            repository.as_ref(),
            "default",
            passphrase("correct horse battery staple"),
        )
        .unwrap();
        vault.forget(repository.as_ref()).unwrap();
        assert!(repository.read().unwrap().is_none());
        assert_eq!(
            VaultSession::unlock(
                repository.as_ref(),
                "default",
                passphrase("correct horse battery staple"),
            )
            .unwrap_err(),
            VaultError::Missing
        );
    }

    #[test]
    fn entries_are_single_line_bounded_and_conflicting_writes_do_not_overwrite() {
        let repository = MemoryVaultRepository::default();
        assert_eq!(
            VaultSession::create(&repository, "default", passphrase("short")).unwrap_err(),
            VaultError::PassphraseTooShort
        );
        let mut first = VaultSession::create(
            &repository,
            "default",
            passphrase("correct horse battery staple"),
        )
        .unwrap();
        assert_eq!(
            first.add(&repository, "bad", "multi\nline"),
            Err(VaultError::InvalidEntry)
        );
        let mut second = VaultSession::unlock(
            &repository,
            "default",
            passphrase("correct horse battery staple"),
        )
        .unwrap();
        first.add(&repository, "first", "first-secret").unwrap();
        assert_eq!(
            second.add(&repository, "second", "second-secret"),
            Err(VaultError::Conflict)
        );
        assert!(second.entries().is_empty());
    }
}
