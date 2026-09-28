use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use fs2::FileExt;
use serde::{Deserialize, Serialize};

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

    fn profile_dir(&self) -> PathBuf {
        self.root.0.join("profiles").join(&self.profile)
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
        let document = parse_document(&read_limited(&path, MAX_NATIVE_BYTES)?)?;
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
        let document = NativeDocument {
            schema_version: SCHEMA_VERSION,
            revision,
            preferences: preferences.clone(),
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
