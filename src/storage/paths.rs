use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

const MAX_METADATA_BYTES: u64 = 64 * 1024;
const MAX_CONFIG_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct NativeDataRoot(pub PathBuf);

#[derive(Clone, Debug)]
pub struct LegacyImportRoot(pub PathBuf);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyProfilePaths {
    pub name: String,
    pub config: PathBuf,
    pub themes: PathBuf,
    pub used_root_fallback: bool,
}

#[derive(Debug)]
pub enum PathError {
    MissingHome,
    MissingSource,
    MissingConfig,
    InvalidProfile,
    InvalidMetadata,
    TooLarge,
    OutsideRoot,
    Io(io::Error),
}

impl From<io::Error> for PathError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingHome => write!(f, "cannot locate the user's home directory"),
            Self::MissingSource => write!(f, "original ButtonsCLI settings folder is missing"),
            Self::MissingConfig => write!(
                f,
                "no config exists in the selected original profile or root"
            ),
            Self::InvalidProfile => write!(f, "invalid profile name"),
            Self::InvalidMetadata => write!(f, "invalid active-profile metadata"),
            Self::TooLarge => write!(f, "settings file exceeds its size limit"),
            Self::OutsideRoot => write!(f, "settings path resolves outside the selected root"),
            Self::Io(error) => write!(f, "settings path I/O: {error}"),
        }
    }
}

impl std::error::Error for PathError {}

pub fn production_roots() -> Result<(NativeDataRoot, LegacyImportRoot), PathError> {
    let home = home::home_dir().ok_or(PathError::MissingHome)?;
    Ok((
        NativeDataRoot::from_home(&home),
        LegacyImportRoot::from_home(&home),
    ))
}

impl NativeDataRoot {
    pub fn from_home(home: &Path) -> Self {
        Self(home.join(".buttonscli-native"))
    }

    pub fn profile_dir(&self, profile: &str) -> Result<PathBuf, PathError> {
        Ok(self
            .0
            .join("profiles")
            .join(sanitize_profile_name(profile)?))
    }
}

impl LegacyImportRoot {
    pub fn from_home(home: &Path) -> Self {
        Self(home.join(".buttonscli"))
    }

    pub fn resolve_active(&self) -> Result<LegacyProfilePaths, PathError> {
        let root = self.0.canonicalize().map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                PathError::MissingSource
            } else {
                PathError::Io(error)
            }
        })?;
        let metadata_path = root.join("active-profile.json");
        let name = if let Some(metadata) = read_checked(&root, &metadata_path, MAX_METADATA_BYTES)?
        {
            #[derive(Deserialize)]
            struct ActiveProfile {
                name: String,
            }
            let active: ActiveProfile =
                serde_json::from_slice(&metadata).map_err(|_| PathError::InvalidMetadata)?;
            sanitize_profile_name(&active.name)?
        } else {
            "default".to_owned()
        };
        self.resolve_at(&root, name, true)
    }

    pub fn resolve_named(&self, name: &str) -> Result<LegacyProfilePaths, PathError> {
        let root = self.0.canonicalize().map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                PathError::MissingSource
            } else {
                PathError::Io(error)
            }
        })?;
        let name = sanitize_profile_name(name)?;
        self.resolve_at(&root, name, false)
    }

    pub fn list_profile_names(&self) -> Result<Vec<String>, PathError> {
        let root = self.0.canonicalize().map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                PathError::MissingSource
            } else {
                PathError::Io(error)
            }
        })?;
        let profiles = root.join("profiles");
        if !profiles.exists() {
            return Ok(Vec::new());
        }
        ensure_inside(&root, &profiles)?;
        let mut names = Vec::new();
        for entry in fs::read_dir(&profiles)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if sanitize_profile_name(&name).is_ok_and(|sanitized| sanitized == name) {
                ensure_inside(&root, &entry.path())?;
                if entry.path().join("config.json").is_file() {
                    names.push(name);
                }
            }
        }
        names.sort();
        Ok(names)
    }

    fn resolve_at(
        &self,
        root: &Path,
        name: String,
        allow_root_fallback: bool,
    ) -> Result<LegacyProfilePaths, PathError> {
        let profile_dir = root.join("profiles").join(&name);
        let profile_config = profile_dir.join("config.json");
        let root_config = root.join("config.json");
        let (config, used_root_fallback) =
            if read_checked(root, &profile_config, MAX_CONFIG_BYTES)?.is_some() {
                (profile_config, false)
            } else if allow_root_fallback
                && read_checked(root, &root_config, MAX_CONFIG_BYTES)?.is_some()
            {
                (root_config, true)
            } else {
                return Err(PathError::MissingConfig);
            };
        let themes = profile_dir.join("themes");
        // Existing theme directories are validated before later enumeration.
        if fs::symlink_metadata(&themes).is_ok() {
            ensure_inside(root, &themes)?;
        }
        Ok(LegacyProfilePaths {
            name,
            config,
            themes,
            used_root_fallback,
        })
    }
}

/// Match original ASCII profile spelling while rejecting path syntax and empty aliases.
pub fn sanitize_profile_name(name: &str) -> Result<String, PathError> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\'])
        || name.chars().any(char::is_control)
    {
        return Err(PathError::InvalidProfile);
    }
    let sanitized: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.trim_matches('_').is_empty() {
        Err(PathError::InvalidProfile)
    } else {
        Ok(sanitized)
    }
}

fn ensure_inside(root: &Path, candidate: &Path) -> Result<(), PathError> {
    let resolved = candidate.canonicalize()?;
    if resolved.starts_with(root) {
        Ok(())
    } else {
        Err(PathError::OutsideRoot)
    }
}

fn read_checked(
    root: &Path,
    candidate: &Path,
    max_bytes: u64,
) -> Result<Option<Vec<u8>>, PathError> {
    let metadata = match fs::symlink_metadata(candidate) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    ensure_inside(root, candidate)?;
    if !metadata.is_file() {
        return Err(PathError::InvalidMetadata);
    }
    if metadata.len() > max_bytes {
        return Err(PathError::TooLarge);
    }
    let bytes = fs::read(candidate)?;
    if bytes.len() as u64 > max_bytes {
        return Err(PathError::TooLarge);
    }
    Ok(Some(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestRoot(PathBuf);
    impl TestRoot {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "buttonscli-native-paths-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for TestRoot {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn roots_are_independent_and_profile_name_matches_original() {
        let temp = TestRoot::new();
        assert_ne!(
            NativeDataRoot::from_home(&temp.0).0,
            LegacyImportRoot::from_home(&temp.0).0
        );
        assert_eq!(sanitize_profile_name("Work Space").unwrap(), "Work_Space");
        assert_eq!(sanitize_profile_name("café").unwrap(), "caf_");
        assert!(matches!(
            sanitize_profile_name("../escape"),
            Err(PathError::InvalidProfile)
        ));
        assert!(matches!(
            sanitize_profile_name("日本語"),
            Err(PathError::InvalidProfile)
        ));
    }

    #[test]
    fn active_profile_beats_root_fallback_and_never_changes_source() {
        let temp = TestRoot::new();
        let root = temp.0.join("café home").join(".buttonscli");
        fs::create_dir_all(root.join("profiles/Work_Space")).unwrap();
        fs::write(root.join("active-profile.json"), r#"{"name":"Work Space"}"#).unwrap();
        fs::write(root.join("config.json"), "root").unwrap();
        fs::write(root.join("profiles/Work_Space/config.json"), "profile").unwrap();
        let selected = LegacyImportRoot(root.clone()).resolve_active().unwrap();
        assert_eq!(selected.name, "Work_Space");
        assert!(!selected.used_root_fallback);
        assert_eq!(fs::read(&selected.config).unwrap(), b"profile");
        assert_eq!(fs::read(root.join("config.json")).unwrap(), b"root");
    }

    #[test]
    fn missing_metadata_uses_default_then_root_fallback() {
        let temp = TestRoot::new();
        fs::write(temp.0.join("config.json"), "root").unwrap();
        let selected = LegacyImportRoot(temp.0.clone()).resolve_active().unwrap();
        assert_eq!(selected.name, "default");
        assert!(selected.used_root_fallback);
    }

    #[test]
    fn missing_source_and_oversized_metadata_report_errors() {
        let temp = TestRoot::new();
        assert!(matches!(
            LegacyImportRoot(temp.0.join("missing")).resolve_active(),
            Err(PathError::MissingSource)
        ));
        fs::write(
            temp.0.join("active-profile.json"),
            vec![b'x'; MAX_METADATA_BYTES as usize + 1],
        )
        .unwrap();
        assert!(matches!(
            LegacyImportRoot(temp.0.clone()).resolve_active(),
            Err(PathError::TooLarge)
        ));
    }

    #[test]
    fn named_profile_remains_selectable_when_active_metadata_is_broken() {
        let temp = TestRoot::new();
        fs::create_dir_all(temp.0.join("profiles/Other")).unwrap();
        fs::write(temp.0.join("profiles/Other/config.json"), "{}").unwrap();
        fs::write(temp.0.join("active-profile.json"), "{broken").unwrap();
        let source = LegacyImportRoot(temp.0.clone());
        assert!(matches!(
            source.resolve_active(),
            Err(PathError::InvalidMetadata)
        ));
        assert_eq!(source.list_profile_names().unwrap(), vec!["Other"]);
        assert_eq!(source.resolve_named("Other").unwrap().name, "Other");
    }

    #[test]
    fn canonical_containment_rejects_outside_path() {
        let root = TestRoot::new();
        let outside = TestRoot::new();
        assert!(matches!(
            ensure_inside(&root.0.canonicalize().unwrap(), &outside.0),
            Err(PathError::OutsideRoot)
        ));
    }

    #[test]
    fn malformed_metadata_never_falls_back_or_rewrites() {
        let temp = TestRoot::new();
        let metadata = temp.0.join("active-profile.json");
        fs::write(&metadata, "{bad").unwrap();
        fs::write(temp.0.join("config.json"), "root").unwrap();
        assert!(matches!(
            LegacyImportRoot(temp.0.clone()).resolve_active(),
            Err(PathError::InvalidMetadata)
        ));
        assert_eq!(fs::read(metadata).unwrap(), b"{bad");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_config_cannot_escape_import_root() {
        let temp = TestRoot::new();
        let outside = TestRoot::new();
        fs::write(outside.0.join("config.json"), "private").unwrap();
        std::os::unix::fs::symlink(outside.0.join("config.json"), temp.0.join("config.json"))
            .unwrap();
        assert!(matches!(
            LegacyImportRoot(temp.0.clone()).resolve_active(),
            Err(PathError::OutsideRoot)
        ));
    }
}
