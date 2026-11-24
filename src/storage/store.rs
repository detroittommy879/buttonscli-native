use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::assistant::provider::ProviderSettings;
use crate::look_format::{self, HeldBundle};
use crate::settings::Preferences;

use super::backups::{backup_bytes, backup_file, BackupReason};
use super::paths::{sanitize_profile_name, NativeDataRoot};

const MAX_NATIVE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_QUICK_SECRETS_BYTES: u64 = 1024 * 1024;
const SCHEMA_VERSION: u32 = 1;

#[derive(Debug)]
pub enum StoreError {
    InvalidProfile,
    InvalidDocument,
    UnsupportedVersion,
    TooLarge,
    OriginalRootCollision,
    OutsideNativeRoot,
    Busy,
    StaleRevision,
    ImportCollision,
    ThemeCollision,
    FontCollision,
    InstallerCollision,
    SourceChanged,
    ThemeChanged,
    /// The theme file on disk has this newer format: never overwrite it.
    NewerTheme(u64),
    ProfileExists,
    ActiveProfile,
    Io(io::Error),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidProfile => write!(f, "native profile metadata is invalid"),
            Self::InvalidDocument => write!(f, "native settings document is invalid"),
            Self::UnsupportedVersion => write!(f, "native settings schema is unsupported"),
            Self::TooLarge => write!(f, "native settings document exceeds its size limit"),
            Self::OriginalRootCollision => write!(
                f,
                "native settings root resolves into the original settings folder"
            ),
            Self::OutsideNativeRoot => {
                write!(f, "native settings path resolves outside the native root")
            }
            Self::Busy => write!(f, "another native instance is saving settings"),
            Self::StaleRevision => write!(f, "native settings changed in another instance"),
            Self::ImportCollision => {
                write!(f, "import destination already exists; refresh the preview")
            }
            Self::ThemeCollision => write!(f, "theme file already exists"),
            Self::FontCollision => write!(f, "font file already exists"),
            Self::InstallerCollision => {
                write!(f, "a different bundled installer file already exists")
            }
            Self::SourceChanged => write!(f, "original settings changed; refresh the preview"),
            Self::ThemeChanged => write!(
                f,
                "this theme changed in another window; reload it before saving"
            ),
            Self::NewerTheme(version) => write!(
                f,
                "this theme file was made by a newer ButtonsCLI (theme format {version}; this \
                 version reads up to {}), so it is not overwritten; save under a new name",
                look_format::THEME_FORMAT_VERSION
            ),
            Self::ProfileExists => write!(f, "a profile with that name already exists"),
            Self::ActiveProfile => {
                write!(f, "switch to another profile before renaming this one")
            }
            Self::Io(error) => write!(f, "native settings I/O: {error}"),
        }
    }
}

impl std::error::Error for StoreError {}
impl From<io::Error> for StoreError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Serialize, Deserialize)]
struct NativeDocument {
    schema_version: u32,
    revision: u64,
    preferences: Preferences,
}

pub struct LoadedNative {
    pub preferences: Preferences,
    pub revision: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct ProfileSummary {
    pub name: String,
    pub active: bool,
    pub themes: usize,
    pub modified: Option<std::time::SystemTime>,
}

#[derive(Clone)]
pub struct NativeStore {
    root: NativeDataRoot,
    legacy_root: PathBuf,
    profile: String,
}

impl NativeStore {
    pub fn open(root: NativeDataRoot, legacy_root: PathBuf) -> Result<Self, StoreError> {
        guard_distinct_roots(&root.0, &legacy_root)?;
        let metadata = root.0.join("active-profile.json");
        let profile = if metadata.exists() {
            ensure_native_path(&root.0, &metadata)?;
            let bytes = read_limited(&metadata, 64 * 1024)?;
            #[derive(Deserialize)]
            struct ActiveProfile {
                name: String,
            }
            let active: ActiveProfile =
                serde_json::from_slice(&bytes).map_err(|_| StoreError::InvalidProfile)?;
            sanitize_profile_name(&active.name).map_err(|_| StoreError::InvalidProfile)?
        } else {
            "default".into()
        };
        Ok(Self {
            root,
            legacy_root,
            profile,
        })
    }

    pub(crate) fn profile_dir(&self) -> PathBuf {
        self.root.0.join("profiles").join(&self.profile)
    }

    pub(crate) fn profile_name(&self) -> &str {
        &self.profile
    }

    pub(crate) fn prepare_history_profile(&self) -> Result<PathBuf, StoreError> {
        let _lock = self.acquire_root_lock()?;
        let profile = self.profile_dir();
        ensure_native_path(&self.root.0, &profile)?;
        fs::create_dir_all(&profile)?;
        ensure_native_path(&self.root.0, &profile)?;
        Ok(profile)
    }

    pub(crate) fn root_dir(&self) -> &Path {
        &self.root.0
    }

    /// Read the active profile's encrypted Quick Secrets payload without
    /// projecting or rewriting any other settings.
    pub(crate) fn read_quick_secrets(&self) -> Result<Option<Vec<u8>>, StoreError> {
        guard_distinct_roots(&self.root.0, &self.legacy_root)?;
        let path = self.profile_dir().join("quick-secrets.vault");
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        ensure_native_path(&self.root.0, &path)?;
        if !metadata.is_file() {
            return Err(StoreError::InvalidDocument);
        }
        Ok(Some(read_limited(&path, MAX_QUICK_SECRETS_BYTES)?))
    }

    /// Compare-and-replace ciphertext under the native store lock. This
    /// prevents two unlocked app instances from silently overwriting edits.
    pub(crate) fn save_quick_secrets(
        &self,
        expected_digest: Option<[u8; 32]>,
        bytes: &[u8],
    ) -> Result<[u8; 32], StoreError> {
        if bytes.len() as u64 > MAX_QUICK_SECRETS_BYTES {
            return Err(StoreError::TooLarge);
        }
        guard_distinct_roots(&self.root.0, &self.legacy_root)?;
        fs::create_dir_all(&self.root.0)?;
        let lock_path = self.root.0.join(".native.lock");
        ensure_native_path(&self.root.0, &lock_path)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        lock.try_lock_exclusive().map_err(|error| {
            if error.kind() == io::ErrorKind::WouldBlock {
                StoreError::Busy
            } else {
                StoreError::Io(error)
            }
        })?;
        let path = self.profile_dir().join("quick-secrets.vault");
        let current = self.read_quick_secrets()?;
        let current_digest = current
            .as_deref()
            .map(|current| Sha256::digest(current).into());
        if current_digest != expected_digest {
            return Err(StoreError::StaleRevision);
        }
        ensure_native_path(&self.root.0, &self.profile_dir())?;
        fs::create_dir_all(self.profile_dir())?;
        ensure_native_path(&self.root.0, &path)?;
        atomic_replace(&path, bytes)?;
        Ok(Sha256::digest(bytes).into())
    }

    /// Delete only the active profile's vault after the caller has confirmed
    /// the destructive reset.
    pub(crate) fn delete_quick_secrets(&self, expected_digest: [u8; 32]) -> Result<(), StoreError> {
        guard_distinct_roots(&self.root.0, &self.legacy_root)?;
        let lock_path = self.root.0.join(".native.lock");
        ensure_native_path(&self.root.0, &lock_path)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        lock.try_lock_exclusive().map_err(|error| {
            if error.kind() == io::ErrorKind::WouldBlock {
                StoreError::Busy
            } else {
                StoreError::Io(error)
            }
        })?;
        let path = self.profile_dir().join("quick-secrets.vault");
        let Some(current) = self.read_quick_secrets()? else {
            return Err(StoreError::StaleRevision);
        };
        let current_digest: [u8; 32] = Sha256::digest(&current).into();
        if current_digest != expected_digest {
            return Err(StoreError::StaleRevision);
        }
        ensure_native_path(&self.root.0, &path)?;
        fs::remove_file(path)?;
        Ok(())
    }

    /// Confirmed reset path for a forgotten passphrase. The digest is captured
    /// before the locked delete, which still detects a concurrent replacement.
    pub(crate) fn forget_quick_secrets(&self) -> Result<(), StoreError> {
        let Some(current) = self.read_quick_secrets()? else {
            return Err(StoreError::StaleRevision);
        };
        let expected: [u8; 32] = Sha256::digest(&current).into();
        self.delete_quick_secrets(expected)
    }

    pub(crate) fn native_path(&self) -> PathBuf {
        self.profile_dir().join("native.json")
    }

    pub fn load(&self) -> Result<Option<LoadedNative>, StoreError> {
        guard_distinct_roots(&self.root.0, &self.legacy_root)?;
        let path = self.native_path();
        if !path.exists() {
            return Ok(None);
        }
        ensure_native_path(&self.root.0, &path)?;
        let bytes = read_limited(&path, MAX_NATIVE_BYTES)?;
        let parsed = parse_document(&bytes);
        let reason = if parsed.is_ok() {
            BackupReason::Startup
        } else {
            BackupReason::Questionable
        };
        backup_bytes_once(&self.root.0, &self.profile, &path, &bytes, reason);
        let has_provider_settings = serde_json::from_slice::<serde_json::Value>(&bytes)
            .ok()
            .and_then(|value| value.get("preferences")?.get("provider_settings").cloned())
            .is_some();
        let ParsedNative {
            mut document,
            bundles,
        } = parsed?;
        for held in &bundles.held {
            tracing::warn!(
                "theme bundle {:?} is kept in native.json unchanged but not loaded: {}",
                held.name,
                held.reason
            );
        }
        for upgraded in &bundles.upgraded {
            tracing::info!("native.json: {upgraded}");
        }
        // Installed themes are not known yet; the rest is checked here.
        for bundle in &bundles.kept {
            let report = look_format::validate_bundle(bundle, |_| true);
            if !report.warnings.is_empty() {
                let name = bundle.get("name").and_then(|name| name.as_str());
                tracing::warn!(
                    "theme bundle {:?}: {}",
                    name.unwrap_or_default(),
                    report.summary(5)
                );
            }
        }
        if !has_provider_settings {
            let compatibility = self.profile_dir().join("legacy-compatible.json");
            if compatibility.is_file() {
                ensure_native_path(&self.root.0, &compatibility)?;
                let safe: serde_json::Value =
                    serde_json::from_slice(&read_limited(&compatibility, MAX_NATIVE_BYTES)?)
                        .map_err(|_| StoreError::InvalidDocument)?;
                if let Some(assistant) = safe.get("assistant") {
                    document.preferences.provider_settings =
                        ProviderSettings::from_legacy(assistant).0;
                }
            }
        }
        Ok(Some(LoadedNative {
            preferences: document.preferences,
            revision: document.revision,
        }))
    }

    pub fn save(
        &self,
        expected_revision: Option<u64>,
        preferences: &Preferences,
    ) -> Result<u64, StoreError> {
        guard_distinct_roots(&self.root.0, &self.legacy_root)?;
        fs::create_dir_all(&self.root.0)?;
        guard_distinct_roots(&self.root.0, &self.legacy_root)?;
        let lock_path = self.root.0.join(".native.lock");
        ensure_native_path(&self.root.0, &lock_path)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        lock.try_lock_exclusive().map_err(|error| {
            if error.kind() == io::ErrorKind::WouldBlock {
                StoreError::Busy
            } else {
                StoreError::Io(error)
            }
        })?;
        let path = self.native_path();
        ensure_native_path(&self.root.0, &path)?;
        let existing = if path.exists() {
            Some(parse_document(&read_limited(&path, MAX_NATIVE_BYTES)?)?)
        } else {
            None
        };
        let actual_revision = existing.as_ref().map(|parsed| parsed.document.revision);
        if actual_revision != expected_revision {
            return Err(StoreError::StaleRevision);
        }
        let revision = actual_revision
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(StoreError::StaleRevision)?;
        let mut safe_preferences = preferences.clone();
        safe_preferences
            .provider_settings
            .normalize(&mut Vec::new());
        let document = NativeDocument {
            schema_version: SCHEMA_VERSION,
            revision,
            preferences: safe_preferences,
        };
        let kept = existing.map(|parsed| parsed.bundles).unwrap_or_default();
        let encoded = encode_document(&document, &kept)?;
        if encoded.len() as u64 > MAX_NATIVE_BYTES {
            return Err(StoreError::TooLarge);
        }
        let metadata = serde_json::json!({"name": self.profile});
        let metadata_path = self.root.0.join("active-profile.json");
        ensure_native_path(&self.root.0, &metadata_path)?;
        atomic_replace(
            &metadata_path,
            serde_json::to_string(&metadata).unwrap().as_bytes(),
        )?;
        fs::create_dir_all(self.profile_dir())?;
        ensure_native_path(&self.root.0, &path)?;
        backup_file(&self.root.0, &self.profile, &path, BackupReason::Routine);
        atomic_replace(&path, &encoded)?;
        Ok(revision)
    }

    /// Publish a fully staged, new profile. Nothing in the original root is writable here.
    pub(crate) fn import_profile(
        &self,
        profile: &str,
        preferences: &Preferences,
        compatible_config: &[u8],
        themes: &[(String, Vec<u8>)],
        manifest: &[u8],
        verify_source: impl FnOnce() -> bool,
    ) -> Result<Self, StoreError> {
        sanitize_profile_name(profile).map_err(|_| StoreError::InvalidProfile)?;
        guard_distinct_roots(&self.root.0, &self.legacy_root)?;
        fs::create_dir_all(&self.root.0)?;
        guard_distinct_roots(&self.root.0, &self.legacy_root)?;
        let lock_path = self.root.0.join(".native.lock");
        ensure_native_path(&self.root.0, &lock_path)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        lock.try_lock_exclusive().map_err(|error| {
            if error.kind() == io::ErrorKind::WouldBlock {
                StoreError::Busy
            } else {
                StoreError::Io(error)
            }
        })?;
        let profiles = self.root.0.join("profiles");
        ensure_native_path(&self.root.0, &profiles)?;
        fs::create_dir_all(&profiles)?;
        let destination = profiles.join(profile);
        ensure_native_path(&self.root.0, &destination)?;
        if destination.exists() {
            return Err(StoreError::ImportCollision);
        }
        let document = NativeDocument {
            schema_version: SCHEMA_VERSION,
            revision: 1,
            preferences: preferences.clone(),
        };
        let native_bytes =
            serde_json::to_vec_pretty(&document).map_err(|_| StoreError::InvalidDocument)?;
        if native_bytes.len() as u64 > MAX_NATIVE_BYTES
            || compatible_config.len() as u64 > MAX_NATIVE_BYTES
        {
            return Err(StoreError::TooLarge);
        }
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| StoreError::InvalidDocument)?
            .as_nanos();
        let staging = profiles.join(format!(".import-{}-{nonce}", std::process::id()));
        ensure_native_path(&self.root.0, &staging)?;
        fs::create_dir(&staging)?;
        let result = (|| -> Result<(), StoreError> {
            write_new(&staging.join("native.json"), &native_bytes)?;
            write_new(&staging.join("legacy-compatible.json"), compatible_config)?;
            write_new(&staging.join("import-manifest.json"), manifest)?;
            if !themes.is_empty() {
                fs::create_dir(staging.join("themes"))?;
                for (name, bytes) in themes {
                    let filename = Path::new(name);
                    if filename.components().count() != 1
                        || !filename
                            .extension()
                            .and_then(|s| s.to_str())
                            .is_some_and(|s| s.eq_ignore_ascii_case("json"))
                    {
                        return Err(StoreError::InvalidDocument);
                    }
                    write_new(&staging.join("themes").join(filename), bytes)?;
                }
            }
            if !verify_source() {
                return Err(StoreError::SourceChanged);
            }
            ensure_native_path(&self.root.0, &destination)?;
            if destination.exists() {
                return Err(StoreError::ImportCollision);
            }
            fs::rename(&staging, &destination)?;
            let metadata_path = self.root.0.join("active-profile.json");
            ensure_native_path(&self.root.0, &metadata_path)?;
            if let Err(error) = atomic_replace(
                &metadata_path,
                serde_json::json!({"name":profile}).to_string().as_bytes(),
            ) {
                fs::remove_dir_all(&destination)?;
                return Err(error);
            }
            Ok(())
        })();
        if result.is_err() {
            // This directory was created by this transaction and has never held user data.
            let _ = fs::remove_dir_all(&staging);
        }
        result?;
        Ok(Self {
            root: self.root.clone(),
            legacy_root: self.legacy_root.clone(),
            profile: profile.to_owned(),
        })
    }

    /// Every native profile on disk, so the app can show and switch between them.
    pub(crate) fn list_profiles(&self) -> Vec<ProfileSummary> {
        let profiles = self.root.0.join("profiles");
        let Ok(entries) = fs::read_dir(&profiles) else {
            return Vec::new();
        };
        let mut list: Vec<ProfileSummary> = entries
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_dir())
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                // Hidden staging folders and unsafe names are never profiles.
                if name.starts_with('.') || sanitize_profile_name(&name).ok()? != name {
                    return None;
                }
                let path = entry.path();
                if ensure_native_path(&self.root.0, &path).is_err() {
                    return None;
                }
                let themes = fs::read_dir(path.join("themes"))
                    .map(|themes| {
                        themes
                            .filter_map(Result::ok)
                            .filter(|theme| {
                                theme
                                    .path()
                                    .extension()
                                    .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
                            })
                            .count()
                    })
                    .unwrap_or(0);
                let modified = fs::metadata(path.join("native.json"))
                    .and_then(|metadata| metadata.modified())
                    .ok();
                Some(ProfileSummary {
                    active: name == self.profile,
                    name,
                    themes,
                    modified,
                })
            })
            .collect();
        list.sort_by(|a, b| b.modified.cmp(&a.modified).then(a.name.cmp(&b.name)));
        list
    }

    /// Make another existing native profile the active one. Nothing is copied or removed.
    pub(crate) fn switch_profile(&self, name: &str) -> Result<Self, StoreError> {
        let profile = sanitize_profile_name(name).map_err(|_| StoreError::InvalidProfile)?;
        if profile != name {
            return Err(StoreError::InvalidProfile);
        }
        let _lock = self.acquire_root_lock()?;
        let directory = self.root.0.join("profiles").join(&profile);
        ensure_native_path(&self.root.0, &directory)?;
        if !directory.is_dir() {
            return Err(StoreError::InvalidProfile);
        }
        let metadata_path = self.root.0.join("active-profile.json");
        ensure_native_path(&self.root.0, &metadata_path)?;
        atomic_replace(
            &metadata_path,
            serde_json::json!({ "name": profile })
                .to_string()
                .as_bytes(),
        )?;
        Ok(Self {
            root: self.root.clone(),
            legacy_root: self.legacy_root.clone(),
            profile,
        })
    }

    /// The requested profile name if it is already safe, so users see
    /// exactly what they typed (no silent `_` replacements).
    fn checked_profile_name(name: &str) -> Result<String, StoreError> {
        let name = name.trim();
        let profile = sanitize_profile_name(name).map_err(|_| StoreError::InvalidProfile)?;
        if profile != name || profile.starts_with('.') {
            return Err(StoreError::InvalidProfile);
        }
        Ok(profile)
    }

    /// Create an empty profile folder (default settings, no themes). It is
    /// not made active.
    pub(crate) fn create_profile(&self, name: &str) -> Result<String, StoreError> {
        let profile = Self::checked_profile_name(name)?;
        let _lock = self.acquire_root_lock()?;
        let profiles = self.root.0.join("profiles");
        ensure_native_path(&self.root.0, &profiles)?;
        fs::create_dir_all(&profiles)?;
        let directory = profiles.join(&profile);
        ensure_native_path(&self.root.0, &directory)?;
        if directory.exists() {
            return Err(StoreError::ProfileExists);
        }
        fs::create_dir(&directory)?;
        fs::create_dir(directory.join("themes"))?;
        Ok(profile)
    }

    /// Rename a profile that is not active. Its saved references to its
    /// own themes (`personal:<old>:<stem>`) are rewritten to the new name.
    pub(crate) fn rename_profile(&self, old: &str, new: &str) -> Result<String, StoreError> {
        let old = Self::checked_profile_name(old)?;
        let new = Self::checked_profile_name(new)?;
        if old == self.profile {
            return Err(StoreError::ActiveProfile);
        }
        let _lock = self.acquire_root_lock()?;
        let profiles = self.root.0.join("profiles");
        let from = profiles.join(&old);
        let to = profiles.join(&new);
        ensure_native_path(&self.root.0, &from)?;
        ensure_native_path(&self.root.0, &to)?;
        if !from.is_dir() {
            return Err(StoreError::InvalidProfile);
        }
        // Windows paths are case-insensitive: "work" -> "Work" renames the
        // same folder, not a collision.
        if to.exists() && !old.eq_ignore_ascii_case(&new) {
            return Err(StoreError::ProfileExists);
        }
        fs::rename(&from, &to)?;
        let settings = to.join("native.json");
        if let Ok(text) = fs::read_to_string(&settings) {
            let renamed =
                text.replace(&format!("\"personal:{old}:"), &format!("\"personal:{new}:"));
            if renamed != text {
                atomic_replace(&settings, renamed.as_bytes())?;
            }
        }
        Ok(new)
    }

    /// Bundles in native.json this app keeps but cannot use (made by a newer
    /// ButtonsCLI, or unreadable). Read from disk on each call.
    pub(crate) fn held_bundles(&self) -> Result<Vec<HeldBundle>, StoreError> {
        let path = self.native_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        ensure_native_path(&self.root.0, &path)?;
        Ok(parse_document(&read_limited(&path, MAX_NATIVE_BYTES)?)?
            .bundles
            .held)
    }

    pub(crate) fn native_root(&self) -> &Path {
        &self.root.0
    }

    pub(crate) fn write_theme_file(
        &self,
        file_name: &str,
        bytes: &[u8],
        replace: bool,
    ) -> Result<(), StoreError> {
        validate_theme_file_name(file_name)?;
        if bytes.len() as u64 > 2 * 1024 * 1024 {
            return Err(StoreError::TooLarge);
        }
        let _lock = self.acquire_root_lock()?;
        let themes = self.ensure_profile_themes_dir()?;
        let path = themes.join(file_name);
        ensure_native_path(&self.root.0, &path)?;
        if replace {
            if !path.is_file() {
                return Err(StoreError::InvalidDocument);
            }
            refuse_newer_theme(&path)?;
            backup_file(&self.root.0, &self.profile, &path, BackupReason::Replace);
            atomic_replace(&path, bytes)
        } else {
            match write_new(&path, bytes) {
                Ok(()) => Ok(()),
                Err(StoreError::Io(error)) if error.kind() == io::ErrorKind::AlreadyExists => {
                    Err(StoreError::ThemeCollision)
                }
                Err(error) => Err(error),
            }
        }
    }

    /// Digest of a theme file's current bytes, used to detect edits made by another window.
    pub(crate) fn theme_file_digest(&self, file_name: &str) -> Result<[u8; 32], StoreError> {
        validate_theme_file_name(file_name)?;
        let path = self.profile_dir().join("themes").join(file_name);
        ensure_native_path(&self.root.0, &path)?;
        let bytes = fs::read(&path)?;
        Ok(Sha256::digest(&bytes).into())
    }

    /// Replaces an existing theme file only if it still matches `expected`, under the write lock.
    pub(crate) fn replace_theme_file_if_unchanged(
        &self,
        file_name: &str,
        bytes: &[u8],
        expected: [u8; 32],
    ) -> Result<(), StoreError> {
        validate_theme_file_name(file_name)?;
        if bytes.len() as u64 > 2 * 1024 * 1024 {
            return Err(StoreError::TooLarge);
        }
        let _lock = self.acquire_root_lock()?;
        let themes = self.ensure_profile_themes_dir()?;
        let path = themes.join(file_name);
        ensure_native_path(&self.root.0, &path)?;
        if !path.is_file() {
            return Err(StoreError::InvalidDocument);
        }
        let current: [u8; 32] = Sha256::digest(fs::read(&path)?).into();
        if current != expected {
            return Err(StoreError::ThemeChanged);
        }
        refuse_newer_theme(&path)?;
        backup_file(&self.root.0, &self.profile, &path, BackupReason::Replace);
        atomic_replace(&path, bytes)
    }

    pub(crate) fn delete_theme_file(&self, file_name: &str) -> Result<(), StoreError> {
        validate_theme_file_name(file_name)?;
        let _lock = self.acquire_root_lock()?;
        let themes = self.ensure_profile_themes_dir()?;
        let path = themes.join(file_name);
        ensure_native_path(&self.root.0, &path)?;
        if !path.is_file() {
            return Err(StoreError::InvalidDocument);
        }
        backup_file(&self.root.0, &self.profile, &path, BackupReason::Delete);
        fs::remove_file(path)?;
        Ok(())
    }

    pub(crate) fn profile_fonts_dir(&self) -> Result<PathBuf, StoreError> {
        let _lock = self.acquire_root_lock()?;
        self.ensure_profile_fonts_dir()
    }

    pub(crate) fn write_font_file(&self, file_name: &str, bytes: &[u8]) -> Result<(), StoreError> {
        validate_font_file_name(file_name)?;
        if bytes.len() as u64 > 32 * 1024 * 1024 {
            return Err(StoreError::TooLarge);
        }
        let _lock = self.acquire_root_lock()?;
        let fonts = self.ensure_profile_fonts_dir()?;
        let path = fonts.join(file_name);
        ensure_native_path(&self.root.0, &path)?;
        match write_new(&path, bytes) {
            Ok(()) => Ok(()),
            Err(StoreError::Io(error)) if error.kind() == io::ErrorKind::AlreadyExists => {
                Err(StoreError::FontCollision)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn prepare_bundled_installer(
        &self,
        file_name: &str,
        bytes: &[u8],
    ) -> Result<PathBuf, StoreError> {
        validate_installer_file_name(file_name)?;
        if bytes.len() > 2 * 1024 * 1024 {
            return Err(StoreError::TooLarge);
        }
        let _lock = self.acquire_root_lock()?;
        let directory = self.ensure_profile_installers_dir()?;
        let path = directory.join(file_name);
        ensure_native_path(&self.root.0, &path)?;
        match write_new(&path, bytes) {
            Ok(()) => Ok(path),
            Err(StoreError::Io(error)) if error.kind() == io::ErrorKind::AlreadyExists => {
                if fs::read(&path)? == bytes {
                    Ok(path)
                } else {
                    Err(StoreError::InstallerCollision)
                }
            }
            Err(error) => Err(error),
        }
    }

    fn acquire_root_lock(&self) -> Result<fs::File, StoreError> {
        guard_distinct_roots(&self.root.0, &self.legacy_root)?;
        fs::create_dir_all(&self.root.0)?;
        guard_distinct_roots(&self.root.0, &self.legacy_root)?;
        let lock_path = self.root.0.join(".native.lock");
        ensure_native_path(&self.root.0, &lock_path)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        lock.try_lock_exclusive().map_err(|error| {
            if error.kind() == io::ErrorKind::WouldBlock {
                StoreError::Busy
            } else {
                StoreError::Io(error)
            }
        })?;
        Ok(lock)
    }

    fn ensure_profile_themes_dir(&self) -> Result<PathBuf, StoreError> {
        let profiles = self.root.0.join("profiles");
        ensure_native_path(&self.root.0, &profiles)?;
        fs::create_dir_all(&profiles)?;
        ensure_native_path(&self.root.0, &profiles)?;
        let profile = self.profile_dir();
        ensure_native_path(&self.root.0, &profile)?;
        fs::create_dir_all(&profile)?;
        ensure_native_path(&self.root.0, &profile)?;
        let themes = profile.join("themes");
        ensure_native_path(&self.root.0, &themes)?;
        fs::create_dir_all(&themes)?;
        ensure_native_path(&self.root.0, &themes)?;
        Ok(themes)
    }

    fn ensure_profile_fonts_dir(&self) -> Result<PathBuf, StoreError> {
        let profiles = self.root.0.join("profiles");
        ensure_native_path(&self.root.0, &profiles)?;
        fs::create_dir_all(&profiles)?;
        ensure_native_path(&self.root.0, &profiles)?;
        let profile = self.profile_dir();
        ensure_native_path(&self.root.0, &profile)?;
        fs::create_dir_all(&profile)?;
        ensure_native_path(&self.root.0, &profile)?;
        let fonts = profile.join("fonts");
        ensure_native_path(&self.root.0, &fonts)?;
        fs::create_dir_all(&fonts)?;
        ensure_native_path(&self.root.0, &fonts)?;
        Ok(fonts)
    }

    fn ensure_profile_installers_dir(&self) -> Result<PathBuf, StoreError> {
        let profiles = self.root.0.join("profiles");
        ensure_native_path(&self.root.0, &profiles)?;
        fs::create_dir_all(&profiles)?;
        ensure_native_path(&self.root.0, &profiles)?;
        let profile = self.profile_dir();
        ensure_native_path(&self.root.0, &profile)?;
        fs::create_dir_all(&profile)?;
        ensure_native_path(&self.root.0, &profile)?;
        let installers = profile.join("installers");
        ensure_native_path(&self.root.0, &installers)?;
        fs::create_dir_all(&installers)?;
        ensure_native_path(&self.root.0, &installers)?;
        Ok(installers)
    }
}

fn validate_theme_file_name(file_name: &str) -> Result<(), StoreError> {
    let path = Path::new(file_name);
    if path.components().count() != 1
        || path.file_name().and_then(|name| name.to_str()) != Some(file_name)
        || !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        return Err(StoreError::InvalidDocument);
    }
    Ok(())
}

fn validate_font_file_name(file_name: &str) -> Result<(), StoreError> {
    let path = Path::new(file_name);
    if path.components().count() != 1
        || path.file_name().and_then(|name| name.to_str()) != Some(file_name)
        || !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extension.eq_ignore_ascii_case("ttf") || extension.eq_ignore_ascii_case("otf")
            })
    {
        return Err(StoreError::InvalidDocument);
    }
    Ok(())
}

fn validate_installer_file_name(file_name: &str) -> Result<(), StoreError> {
    let path = Path::new(file_name);
    if path.components().count() != 1
        || path.file_name().and_then(|name| name.to_str()) != Some(file_name)
        || !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extension.eq_ignore_ascii_case("ps1") || extension.eq_ignore_ascii_case("sh")
            })
        || !file_name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
        })
    {
        return Err(StoreError::InvalidDocument);
    }
    Ok(())
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

struct ParsedNative {
    document: NativeDocument,
    /// Bundles held back (newer or unreadable) and unknown bundle fields,
    /// which a save writes back.
    bundles: look_format::BundleSplit,
}

/// Reads native.json. Theme bundles go through `look_format` first: older
/// formats are upgraded, and a bundle this app cannot use (newer format, or
/// unreadable) is held back instead of failing the whole file.
fn parse_document(bytes: &[u8]) -> Result<ParsedNative, StoreError> {
    let mut value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| StoreError::InvalidDocument)?;
    let list = value
        .get_mut("preferences")
        .and_then(|preferences| preferences.get_mut("theme_bundles"));
    let mut bundles = look_format::BundleSplit::default();
    if let Some(list) = list {
        if let serde_json::Value::Array(items) = list.take() {
            bundles = look_format::split_bundles(items);
            *list = serde_json::Value::Array(bundles.kept.clone());
        }
    }
    let document: NativeDocument =
        serde_json::from_value(value).map_err(|_| StoreError::InvalidDocument)?;
    if document.schema_version != SCHEMA_VERSION {
        return Err(StoreError::UnsupportedVersion);
    }
    Ok(ParsedNative { document, bundles })
}

/// The bytes to save. Bundles held back at load and fields of bundles this
/// version does not know are written back as they were.
fn encode_document(
    document: &NativeDocument,
    kept: &look_format::BundleSplit,
) -> Result<Vec<u8>, StoreError> {
    if kept.held.is_empty() && kept.extras.is_empty() {
        return serde_json::to_vec_pretty(document).map_err(|_| StoreError::InvalidDocument);
    }
    let mut value = serde_json::to_value(document).map_err(|_| StoreError::InvalidDocument)?;
    if let Some(serde_json::Value::Array(bundles)) = value
        .get_mut("preferences")
        .and_then(|preferences| preferences.get_mut("theme_bundles"))
    {
        look_format::restore_bundles(bundles, &kept.held, &kept.extras);
    }
    serde_json::to_vec_pretty(&value).map_err(|_| StoreError::InvalidDocument)
}

/// A theme file made by a newer app must not be overwritten by this one.
fn refuse_newer_theme(path: &Path) -> Result<(), StoreError> {
    let Ok(bytes) = fs::read(path) else {
        return Ok(());
    };
    let Ok(document) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return Ok(());
    };
    match look_format::THEME_CHAIN.check(&document) {
        Err(look_format::FormatError::Newer { found, .. }) => Err(StoreError::NewerTheme(found)),
        _ => Ok(()),
    }
}

fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>, StoreError> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > limit {
        return Err(StoreError::TooLarge);
    }
    let bytes = fs::read(path)?;
    if bytes.len() as u64 > limit {
        return Err(StoreError::TooLarge);
    }
    Ok(bytes)
}

fn guard_distinct_roots(native: &Path, original: &Path) -> Result<(), StoreError> {
    if let (Ok(native), Ok(original)) = (native.canonicalize(), original.canonicalize()) {
        if native.starts_with(&original) || original.starts_with(&native) {
            return Err(StoreError::OriginalRootCollision);
        }
    }
    Ok(())
}

pub(crate) fn ensure_native_path(root: &Path, candidate: &Path) -> Result<(), StoreError> {
    let canonical_root = root.canonicalize()?;
    let relative = candidate
        .strip_prefix(root)
        .map_err(|_| StoreError::OutsideNativeRoot)?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(_) => {
                if !current.canonicalize()?.starts_with(&canonical_root) {
                    return Err(StoreError::OutsideNativeRoot);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

/// The startup copy of `native.json`: skipped when the newest copy already
/// holds these bytes (`backup_file` compares them).
fn backup_bytes_once(root: &Path, profile: &str, path: &Path, bytes: &[u8], reason: BackupReason) {
    if reason == BackupReason::Questionable {
        backup_bytes(root, profile, "native.json", bytes, reason);
    } else {
        backup_file(root, profile, path, reason);
    }
}

pub(crate) fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let parent = path.parent().ok_or(StoreError::InvalidProfile)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StoreError::InvalidDocument)?
        .as_nanos();
    let temp = parent.join(format!(".native-{}-{nonce}.tmp", std::process::id()));
    let result = (|| -> Result<(), StoreError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roots() -> (PathBuf, PathBuf) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!(
            "buttonscli-native-store-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&base).unwrap();
        (base.join("native"), base.join("original"))
    }

    #[test]
    fn profiles_can_be_created_and_renamed_with_their_theme_references() {
        let (native, original) = roots();
        let store = NativeStore::open(NativeDataRoot(native.clone()), original).unwrap();
        assert_eq!(store.create_profile("work").unwrap(), "work");
        assert!(matches!(
            store.create_profile("work"),
            Err(StoreError::ProfileExists)
        ));
        assert!(matches!(
            store.create_profile("../escape"),
            Err(StoreError::InvalidProfile)
        ));
        let dir = native.join("profiles").join("work");
        assert!(dir.join("themes").is_dir());
        fs::write(
            dir.join("native.json"),
            r#"{"theme_id":"personal:work:blue","other":"personal:default:x"}"#,
        )
        .unwrap();
        assert_eq!(store.rename_profile("work", "studio").unwrap(), "studio");
        let text =
            fs::read_to_string(native.join("profiles").join("studio").join("native.json")).unwrap();
        assert!(text.contains("personal:studio:blue"));
        assert!(text.contains("personal:default:x"));
        assert!(!dir.exists());
        let active = store.profile_name().to_owned();
        assert!(matches!(
            store.rename_profile(&active, "elsewhere"),
            Err(StoreError::ActiveProfile)
        ));
        fs::remove_dir_all(native.parent().unwrap()).unwrap();
    }

    #[test]
    fn saves_atomically_and_detects_stale_second_instance() {
        let (native, original) = roots();
        let first = NativeStore::open(NativeDataRoot(native.clone()), original.clone()).unwrap();
        let second = NativeStore::open(NativeDataRoot(native.clone()), original).unwrap();
        assert!(first.load().unwrap().is_none());
        let mut preferences = Preferences::default();
        preferences.presets.clear();
        assert_eq!(first.save(None, &preferences).unwrap(), 1);
        assert!(matches!(
            second.save(None, &preferences),
            Err(StoreError::StaleRevision)
        ));
        let loaded = second.load().unwrap().unwrap();
        assert!(loaded.preferences.presets.is_empty());
        assert_eq!(first.save(Some(1), &preferences).unwrap(), 2);
        assert_eq!(second.load().unwrap().unwrap().revision, 2);
        fs::remove_dir_all(native.parent().unwrap()).unwrap();
    }

    fn bundles_of(native: &Path) -> Vec<serde_json::Value> {
        let path = native.join("profiles").join("default").join("native.json");
        let value: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        value["preferences"]["theme_bundles"]
            .as_array()
            .unwrap()
            .clone()
    }

    #[test]
    fn old_bundles_load_and_save_unchanged() {
        let (native, original) = roots();
        let store = NativeStore::open(NativeDataRoot(native.clone()), original).unwrap();
        let preferences = Preferences {
            theme_bundles: vec![crate::settings::ThemeBundle {
                name: "Old".into(),
                theme_id: "basic2".into(),
                panes: vec![Default::default()],
                layout: "grid".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let first = store.save(None, &preferences).unwrap();
        let written = bundles_of(&native);
        // Format 1 is not written: older builds read the same file.
        assert!(written[0].get("format_version").is_none());
        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.preferences.theme_bundles, preferences.theme_bundles);
        store.save(Some(first), &loaded.preferences).unwrap();
        assert_eq!(bundles_of(&native), written);
        assert!(store.held_bundles().unwrap().is_empty());
        fs::remove_dir_all(native.parent().unwrap()).unwrap();
    }

    #[test]
    fn newer_and_unreadable_bundles_stay_in_native_json() {
        let (native, original) = roots();
        let store = NativeStore::open(NativeDataRoot(native.clone()), original).unwrap();
        let first = store.save(None, &Preferences::default()).unwrap();
        let path = native.join("profiles").join("default").join("native.json");
        let mut file: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let future = serde_json::json!({
            "format_version": 7, "name": "Future", "panes": {"new": "shape"}
        });
        let broken = serde_json::json!({"name": "Broken", "visible_panes": "two"});
        let extra = serde_json::json!({"name": "Extra", "notes": {"by": "a newer app"}});
        file["preferences"]["theme_bundles"] = serde_json::json!([
            {"name": "Mine"}, future.clone(), extra.clone(), broken.clone()
        ]);
        fs::write(&path, serde_json::to_vec_pretty(&file).unwrap()).unwrap();

        // The file still loads; only the bundles this app reads are in it.
        let loaded = store.load().unwrap().unwrap();
        let names: Vec<&str> = loaded
            .preferences
            .theme_bundles
            .iter()
            .map(|bundle| bundle.name.as_str())
            .collect();
        assert_eq!(names, ["Mine", "Extra"]);
        let held = store.held_bundles().unwrap();
        assert_eq!(held.len(), 2);
        assert!(
            held[0].reason.contains("made by a newer ButtonsCLI"),
            "{}",
            held[0].reason
        );

        // Saving (with a bundle removed and one added) writes them back.
        let mut preferences = loaded.preferences;
        preferences.theme_bundles.remove(0);
        preferences
            .theme_bundles
            .push(crate::settings::ThemeBundle {
                name: "New".into(),
                ..Default::default()
            });
        store.save(Some(first), &preferences).unwrap();
        let bundles = bundles_of(&native);
        let names: Vec<&str> = bundles
            .iter()
            .map(|bundle| bundle["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["Extra", "Future", "New", "Broken"]);
        assert_eq!(bundles[1], future);
        assert_eq!(bundles[3], broken);
        assert_eq!(bundles[0]["notes"], extra["notes"]);
        fs::remove_dir_all(native.parent().unwrap()).unwrap();
    }

    #[test]
    fn newer_theme_files_are_never_overwritten() {
        let (native, original) = roots();
        let store = NativeStore::open(NativeDataRoot(native.clone()), original).unwrap();
        let newer = br#"{"version": 2, "metadata": {"id": "x", "name": "X"}}"#;
        store.write_theme_file("future.json", newer, false).unwrap();
        let replaced = store.write_theme_file("future.json", b"{}", true);
        assert!(
            matches!(replaced, Err(StoreError::NewerTheme(2))),
            "{replaced:?}"
        );
        let digest = store.theme_file_digest("future.json").unwrap();
        let replaced = store.replace_theme_file_if_unchanged("future.json", b"{}", digest);
        let message = replaced.unwrap_err().to_string();
        assert!(
            message.contains("made by a newer ButtonsCLI (theme format 2"),
            "{message}"
        );
        let path = native
            .join("profiles")
            .join("default")
            .join("themes")
            .join("future.json");
        assert_eq!(fs::read(path).unwrap(), newer);
        // Current files are still replaced.
        store
            .write_theme_file("now.json", br#"{"version": 1}"#, false)
            .unwrap();
        store.write_theme_file("now.json", b"{}", true).unwrap();
        fs::remove_dir_all(native.parent().unwrap()).unwrap();
    }

    #[test]
    fn quick_secrets_ciphertext_is_profile_scoped_and_deletion_checks_revision() {
        let (native, original) = roots();
        let first = NativeStore::open(NativeDataRoot(native.clone()), original.clone()).unwrap();
        let second = NativeStore::open(NativeDataRoot(native.clone()), original).unwrap();
        assert_eq!(first.read_quick_secrets().unwrap(), None);

        let old = b"opaque encrypted payload";
        let old_digest = first.save_quick_secrets(None, old).unwrap();
        assert_eq!(
            second.read_quick_secrets().unwrap().as_deref(),
            Some(old.as_slice())
        );
        assert!(matches!(
            second.save_quick_secrets(None, b"stale write"),
            Err(StoreError::StaleRevision)
        ));

        first
            .save_quick_secrets(Some(old_digest), b"replacement ciphertext")
            .unwrap();
        assert!(matches!(
            first.delete_quick_secrets(old_digest),
            Err(StoreError::StaleRevision)
        ));
        first.forget_quick_secrets().unwrap();
        assert_eq!(second.read_quick_secrets().unwrap(), None);
        fs::remove_dir_all(native.parent().unwrap()).unwrap();
    }

    #[test]
    fn corrupt_native_document_is_never_reset() {
        let (native, original) = roots();
        let store = NativeStore::open(NativeDataRoot(native.clone()), original).unwrap();
        fs::create_dir_all(store.profile_dir()).unwrap();
        fs::write(store.native_path(), "{broken").unwrap();
        assert!(matches!(store.load(), Err(StoreError::InvalidDocument)));
        assert!(matches!(
            store.save(None, &Preferences::default()),
            Err(StoreError::InvalidDocument)
        ));
        assert_eq!(fs::read(store.native_path()).unwrap(), b"{broken");
        fs::remove_dir_all(native.parent().unwrap()).unwrap();
    }

    #[test]
    fn personal_theme_files_are_profile_scoped_atomic_and_collision_safe() {
        let (native, original) = roots();
        let store = NativeStore::open(NativeDataRoot(native.clone()), original).unwrap();
        store
            .write_theme_file("night.json", br#"{"version":1}"#, false)
            .unwrap();
        assert!(matches!(
            store.write_theme_file("night.json", b"overwrite", false),
            Err(StoreError::ThemeCollision)
        ));
        assert_eq!(
            fs::read(store.profile_dir().join("themes/night.json")).unwrap(),
            br#"{"version":1}"#
        );
        store
            .write_theme_file("night.json", b"updated", true)
            .unwrap();
        assert_eq!(
            fs::read(store.profile_dir().join("themes/night.json")).unwrap(),
            b"updated"
        );
        assert!(matches!(
            store.write_theme_file("../outside.json", b"bad", false),
            Err(StoreError::InvalidDocument)
        ));
        store.delete_theme_file("night.json").unwrap();
        assert!(!store.profile_dir().join("themes/night.json").exists());
        fs::remove_dir_all(native.parent().unwrap()).unwrap();
    }

    #[test]
    fn stale_theme_writer_cannot_overwrite_a_newer_theme_file() {
        let (native, original) = roots();
        let store = NativeStore::open(NativeDataRoot(native.clone()), original).unwrap();
        let path = store.profile_dir().join("themes/night.json");
        store
            .write_theme_file("night.json", b"loaded", false)
            .unwrap();
        let loaded = store.theme_file_digest("night.json").unwrap();
        // Another window saves its edit to the same file first.
        store
            .replace_theme_file_if_unchanged("night.json", b"other window", loaded)
            .unwrap();
        // This window still holds the older digest and must be refused.
        assert!(matches!(
            store.replace_theme_file_if_unchanged("night.json", b"stale", loaded),
            Err(StoreError::ThemeChanged)
        ));
        assert_eq!(fs::read(&path).unwrap(), b"other window");
        // Reloading the newer digest allows the save again.
        let current = store.theme_file_digest("night.json").unwrap();
        store
            .replace_theme_file_if_unchanged("night.json", b"reloaded save", current)
            .unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"reloaded save");
        fs::remove_dir_all(native.parent().unwrap()).unwrap();
    }

    #[test]
    fn bundled_installer_files_are_profile_scoped_and_never_overwritten() {
        let (native, original) = roots();
        let store = NativeStore::open(NativeDataRoot(native.clone()), original).unwrap();
        let first = store
            .prepare_bundled_installer("vibes-abc123.ps1", b"Write-Output 'ready'")
            .unwrap();
        let repeated = store
            .prepare_bundled_installer("vibes-abc123.ps1", b"Write-Output 'ready'")
            .unwrap();
        assert_eq!(first, repeated);
        assert_eq!(fs::read(&first).unwrap(), b"Write-Output 'ready'");
        assert!(matches!(
            store.prepare_bundled_installer("vibes-abc123.ps1", b"different script"),
            Err(StoreError::InstallerCollision)
        ));
        assert!(matches!(
            store.prepare_bundled_installer("../outside.ps1", b"bad"),
            Err(StoreError::InvalidDocument)
        ));
        assert!(first.starts_with(store.profile_dir().join("installers")));
        fs::remove_dir_all(native.parent().unwrap()).unwrap();
    }

    #[test]
    fn unusable_native_root_does_not_modify_original() {
        let (native, original) = roots();
        fs::write(&native, "not a directory").unwrap();
        fs::create_dir_all(&original).unwrap();
        let marker = original.join("config.json");
        fs::write(&marker, "original").unwrap();
        let store = NativeStore::open(NativeDataRoot(native.clone()), original).unwrap();
        assert!(matches!(
            store.save(None, &Preferences::default()),
            Err(StoreError::Io(_))
        ));
        assert_eq!(fs::read(marker).unwrap(), b"original");
        fs::remove_dir_all(native.parent().unwrap()).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlink_to_original_root_is_rejected() {
        let (native, original) = roots();
        fs::create_dir_all(&original).unwrap();
        std::os::unix::fs::symlink(&original, &native).unwrap();
        assert!(matches!(
            NativeStore::open(NativeDataRoot(native), original.clone()),
            Err(StoreError::OriginalRootCollision)
        ));
        fs::remove_dir_all(original.parent().unwrap()).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_profiles_directory_cannot_redirect_writes() {
        let (native, original) = roots();
        fs::create_dir_all(&native).unwrap();
        fs::create_dir_all(&original).unwrap();
        std::os::unix::fs::symlink(&original, native.join("profiles")).unwrap();
        let store = NativeStore::open(NativeDataRoot(native.clone()), original.clone()).unwrap();
        assert!(matches!(
            store.save(None, &Preferences::default()),
            Err(StoreError::OutsideNativeRoot)
        ));
        assert!(!original.join("default/native.json").exists());
        fs::remove_dir_all(native.parent().unwrap()).unwrap();
    }
}
