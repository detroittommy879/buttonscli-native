use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use fs2::FileExt;
use serde::{Deserialize, Serialize};

use crate::assistant::provider::ProviderSettings;
use crate::settings::Preferences;

use super::paths::{sanitize_profile_name, NativeDataRoot};

const MAX_NATIVE_BYTES: u64 = 8 * 1024 * 1024;
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
    SourceChanged,
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
            Self::SourceChanged => write!(f, "original settings changed; refresh the preview"),
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

    pub(crate) fn root_dir(&self) -> &Path {
        &self.root.0
    }
    fn native_path(&self) -> PathBuf {
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
        let has_provider_settings = serde_json::from_slice::<serde_json::Value>(&bytes)
            .ok()
            .and_then(|value| value.get("preferences")?.get("provider_settings").cloned())
            .is_some();
        let mut document = parse_document(&bytes)?;
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
        let actual_revision = if path.exists() {
            Some(parse_document(&read_limited(&path, MAX_NATIVE_BYTES)?)?.revision)
        } else {
            None
        };
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
        let encoded =
            serde_json::to_vec_pretty(&document).map_err(|_| StoreError::InvalidDocument)?;
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

    pub(crate) fn delete_theme_file(&self, file_name: &str) -> Result<(), StoreError> {
        validate_theme_file_name(file_name)?;
        let _lock = self.acquire_root_lock()?;
        let themes = self.ensure_profile_themes_dir()?;
        let path = themes.join(file_name);
        ensure_native_path(&self.root.0, &path)?;
        if !path.is_file() {
            return Err(StoreError::InvalidDocument);
        }
        fs::remove_file(path)?;
        Ok(())
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

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn parse_document(bytes: &[u8]) -> Result<NativeDocument, StoreError> {
    let document: NativeDocument =
        serde_json::from_slice(bytes).map_err(|_| StoreError::InvalidDocument)?;
    if document.schema_version != SCHEMA_VERSION {
        return Err(StoreError::UnsupportedVersion);
    }
    Ok(document)
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

fn ensure_native_path(root: &Path, candidate: &Path) -> Result<(), StoreError> {
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

fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
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
