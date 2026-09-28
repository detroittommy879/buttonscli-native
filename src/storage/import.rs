//! Explicit, snapshot-based import. The source root is only ever read.
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::assistant::credentials::{self, CredentialStore};
use crate::assistant::provider::sanitize_endpoint;
use crate::settings::Preferences;

use super::document::LegacyDocument;
use super::paths::{LegacyImportRoot, PathError};
use super::projection::{project, scrub_secrets};
use super::store::{NativeStore, StoreError};

const MAX_CONFIG: u64 = 8 * 1024 * 1024;
const MAX_THEME: u64 = 2 * 1024 * 1024;

#[derive(Debug)]
pub(crate) enum ImportError {
    Source(PathError),
    Store(StoreError),
    InvalidConfig,
    UnsafeSource,
    UnsafeDestination,
    TooLarge,
    Io(io::Error),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(error) => write!(f, "{error}"),
            Self::Store(error) => write!(f, "{error}"),
            Self::InvalidConfig => write!(f, "original config is not a valid settings object"),
            Self::UnsafeSource => {
                write!(f, "original settings path escapes its root or changed type")
            }
            Self::UnsafeDestination => write!(f, "native import path escapes its root"),
            Self::TooLarge => write!(f, "original settings file exceeds its size limit"),
            Self::Io(error) => write!(f, "import I/O: {error}"),
        }
    }
}

impl From<PathError> for ImportError {
    fn from(value: PathError) -> Self {
        Self::Source(value)
    }
}
impl From<StoreError> for ImportError {
    fn from(value: StoreError) -> Self {
        Self::Store(value)
    }
}
impl From<io::Error> for ImportError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Serialize)]
struct ImportManifest {
    schema_version: u32,
    source_profile: String,
    fingerprint_sha256: String,
    command_presets: usize,
    ssh_presets: usize,
    themes: usize,
    exclusions: Vec<&'static str>,
}

struct Prepared {
    selection: Option<String>,
    source_profile: String,
    source_fallback: bool,
    preferences: Preferences,
    compatible_config: Value,
    themes: Vec<(String, Vec<u8>)>,
    source_fingerprint: String,
    fingerprint: String,
    warnings: Vec<String>,
    provider_metadata: Vec<String>,
    selected_locale: Option<String>,
    credential_count: usize,
}

pub(crate) struct ImportPreview {
    pub source_profile: String,
    pub destination_profile: String,
    pub already_imported: bool,
    pub source_fallback: bool,
    pub command_presets: usize,
    pub ssh_presets: usize,
    pub themes: usize,
    pub warnings: Vec<String>,
    pub provider_metadata: Vec<String>,
    pub selected_locale: Option<String>,
    pub credential_count: usize,
    prepared: Prepared,
}

pub(crate) struct CredentialTransferPlan {
    selection: Option<String>,
    source_fingerprint: String,
}

impl ImportPreview {
    pub(crate) fn credential_transfer_plan(&self) -> CredentialTransferPlan {
        CredentialTransferPlan {
            selection: self.prepared.selection.clone(),
            source_fingerprint: self.prepared.source_fingerprint.clone(),
        }
    }
}

pub(crate) struct CredentialTransferResult {
    pub(crate) revision: u64,
    pub(crate) imported: usize,
    pub(crate) failed: usize,
}

/// Called only after an explicit key-transfer choice and a successful profile import.
/// The source is reopened read-only and checked against the preview before any key is saved.
pub(crate) fn transfer_credentials(
    source: &LegacyImportRoot,
    plan: &CredentialTransferPlan,
    store: &NativeStore,
    preferences: &mut Preferences,
    credentials: &dyn CredentialStore,
) -> Result<CredentialTransferResult, ImportError> {
    let current = scan(source, plan.selection.as_deref())?;
    if current.source_fingerprint != plan.source_fingerprint {
        return Err(ImportError::Store(StoreError::SourceChanged));
    }
    let paths = match plan.selection.as_deref() {
        Some(name) => source.resolve_named(name)?,
        None => source.resolve_active()?,
    };
    let raw = read_source(&source.0.canonicalize()?, &paths.config, MAX_CONFIG)?;
    let document = LegacyDocument::parse(&raw).map_err(|_| ImportError::InvalidConfig)?;
    let assistant = document.get("assistant").unwrap_or(&Value::Null);
    let entries = assistant
        .get("namedProviders")
        .and_then(Value::as_array)
        .or_else(|| assistant.get("savedProfiles").and_then(Value::as_array));
    let mut candidates = Vec::new();
    if let Some(entries) = entries {
        for entry in entries {
            if entry["endpoint"]
                .as_str()
                .and_then(sanitize_endpoint)
                .is_none()
            {
                continue;
            }
            candidates.push(entry["apiKey"].as_str().filter(|key| !key.is_empty()));
        }
    } else if assistant["endpoint"]
        .as_str()
        .and_then(sanitize_endpoint)
        .is_some()
    {
        candidates.push(assistant["apiKey"].as_str().filter(|key| !key.is_empty()));
    }
    let mut imported = 0;
    let mut failed = 0;
    for (provider, key) in preferences
        .provider_settings
        .providers
        .iter_mut()
        .zip(candidates)
    {
        let Some(key) = key else {
            continue;
        };
        let reference = credentials::reference(store.profile_name(), &provider.id);
        match credentials.put(&reference, key) {
            Ok(()) => {
                provider.credential_ref = Some(reference);
                imported += 1;
            }
            Err(_) => failed += 1,
        }
    }
    let revision = if imported > 0 {
        store.save(Some(1), preferences)?
    } else {
        1
    };
    Ok(CredentialTransferResult {
        revision,
        imported,
        failed,
    })
}

pub(crate) enum ImportCommit {
    AlreadyImported,
    Imported {
        store: NativeStore,
        preferences: Box<Preferences>,
    },
}

pub(crate) fn preview(
    source: &LegacyImportRoot,
    store: &NativeStore,
    selection: Option<&str>,
) -> Result<ImportPreview, ImportError> {
    let prepared = scan(source, selection)?;
    let (destination_profile, already_imported) = destination(store, &prepared)?;
    Ok(ImportPreview {
        source_profile: prepared.source_profile.clone(),
        destination_profile,
        already_imported,
        source_fallback: prepared.source_fallback,
        command_presets: prepared.preferences.presets.len(),
        ssh_presets: prepared.preferences.ssh_presets.len(),
        themes: prepared.themes.len(),
        warnings: prepared.warnings.clone(),
        provider_metadata: prepared.provider_metadata.clone(),
        selected_locale: prepared.selected_locale.clone(),
        credential_count: prepared.credential_count,
        prepared,
    })
}

pub(crate) fn commit(
    source: &LegacyImportRoot,
    store: &NativeStore,
    preview: ImportPreview,
) -> Result<ImportCommit, ImportError> {
    let current = scan(source, preview.prepared.selection.as_deref())?;
    if current.source_fingerprint != preview.prepared.source_fingerprint
        || current.source_profile != preview.prepared.source_profile
    {
        return Err(ImportError::Store(StoreError::SourceChanged));
    }
    let (destination_now, already_imported) = destination(store, &preview.prepared)?;
    if destination_now != preview.destination_profile
        || already_imported != preview.already_imported
    {
        return Err(ImportError::Store(StoreError::ImportCollision));
    }
    if already_imported {
        return Ok(ImportCommit::AlreadyImported);
    }
    let prepared = preview.prepared;
    let config = serde_json::to_vec_pretty(&prepared.compatible_config)
        .map_err(|_| ImportError::InvalidConfig)?;
    let source_profile = prepared.source_profile.clone();
    let source_fingerprint = prepared.source_fingerprint.clone();
    let selection = prepared.selection.clone();
    let manifest = ImportManifest {
        schema_version: 1,
        source_profile: prepared.source_profile,
        fingerprint_sha256: prepared.fingerprint.clone(),
        command_presets: prepared.preferences.presets.len(),
        ssh_presets: prepared.preferences.ssh_presets.len(),
        themes: prepared.themes.len(),
        exclusions: vec![
            "API keys",
            "runtime/auth files",
            "session history",
            "unknown top-level fields",
        ],
    };
    let manifest = serde_json::to_vec_pretty(&manifest).map_err(|_| ImportError::InvalidConfig)?;
    let new_store = store.import_profile(
        &destination_now,
        &prepared.preferences,
        &config,
        &prepared.themes,
        &manifest,
        || {
            scan(source, selection.as_deref()).is_ok_and(|now| {
                now.source_fingerprint == source_fingerprint && now.source_profile == source_profile
            })
        },
    )?;
    Ok(ImportCommit::Imported {
        store: new_store,
        preferences: Box::new(prepared.preferences),
    })
}

fn scan(source: &LegacyImportRoot, selection: Option<&str>) -> Result<Prepared, ImportError> {
    let paths = match selection {
        Some(name) => source.resolve_named(name)?,
        None => source.resolve_active()?,
    };
    let root = source.0.canonicalize()?;
    let config = read_source(&root, &paths.config, MAX_CONFIG)?;
    let document = LegacyDocument::parse(&config).map_err(|_| ImportError::InvalidConfig)?;
    let credential_count = count_provider_keys(&document);
    let projection = project(&document);
    let mut source_digest = Sha256::new();
    source_digest.update(paths.name.as_bytes());
    source_digest.update([u8::from(paths.used_root_fallback)]);
    source_digest.update((config.len() as u64).to_le_bytes());
    source_digest.update(&config);
    let mut themes = Vec::new();
    let mut warnings = projection.warnings;
    if paths.themes.exists() {
        let theme_root = paths.themes.canonicalize()?;
        if !theme_root.starts_with(&root) {
            return Err(ImportError::UnsafeSource);
        }
        let mut files: Vec<PathBuf> = fs::read_dir(&paths.themes)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<_, _>>()?;
        if files.len() > 256 {
            return Err(ImportError::TooLarge);
        }
        files.sort();
        let mut total_theme_bytes = 0_u64;
        for path in files {
            if path
                .extension()
                .and_then(|s| s.to_str())
                .is_none_or(|s| !s.eq_ignore_ascii_case("json"))
            {
                continue;
            }
            let filename = path
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or(ImportError::UnsafeSource)?
                .to_owned();
            let bytes = read_source(&theme_root, &path, MAX_THEME)?;
            total_theme_bytes += bytes.len() as u64;
            if total_theme_bytes > 32 * 1024 * 1024 {
                return Err(ImportError::TooLarge);
            }
            source_digest.update((filename.len() as u64).to_le_bytes());
            source_digest.update(filename.as_bytes());
            source_digest.update((bytes.len() as u64).to_le_bytes());
            source_digest.update(&bytes);
            let parsed: Value = match serde_json::from_slice(&bytes) {
                Ok(value) => value,
                Err(_) => {
                    warnings.push(format!("{filename}: invalid theme JSON, skipped"));
                    continue;
                }
            };
            if parsed["version"] != 1
                || !parsed["metadata"]["id"].is_string()
                || !parsed["metadata"]["name"].is_string()
                || !parsed["theme"].is_object()
                || !parsed["effects"].is_object()
            {
                warnings.push(format!("{filename}: unsupported theme document, skipped"));
                continue;
            }
            let safe = scrub_secrets(&parsed);
            themes.push((
                filename,
                serde_json::to_vec_pretty(&safe).map_err(|_| ImportError::InvalidConfig)?,
            ));
        }
    }
    let provider_metadata = provider_summary(&projection.safe_config);
    let mut sanitized_digest = Sha256::new();
    sanitized_digest.update(paths.name.as_bytes());
    sanitized_digest.update(
        serde_json::to_vec(&projection.safe_config).map_err(|_| ImportError::InvalidConfig)?,
    );
    for (name, bytes) in &themes {
        sanitized_digest.update(name.as_bytes());
        sanitized_digest.update((bytes.len() as u64).to_le_bytes());
        sanitized_digest.update(bytes);
    }
    Ok(Prepared {
        selection: selection.map(str::to_owned),
        source_profile: paths.name,
        source_fallback: paths.used_root_fallback,
        preferences: projection.preferences,
        compatible_config: projection.safe_config,
        themes,
        source_fingerprint: format!("{:x}", source_digest.finalize()),
        fingerprint: format!("{:x}", sanitized_digest.finalize()),
        warnings,
        provider_metadata,
        selected_locale: projection.selected_locale,
        credential_count,
    })
}

fn count_provider_keys(document: &LegacyDocument) -> usize {
    fn count(value: &Value) -> usize {
        match value {
            Value::Object(fields) => fields
                .iter()
                .map(|(key, value)| {
                    let normalized: String = key
                        .chars()
                        .filter(|ch| ch.is_ascii_alphanumeric())
                        .map(|ch| ch.to_ascii_lowercase())
                        .collect();
                    if normalized == "apikey"
                        && value.as_str().is_some_and(|value| !value.is_empty())
                    {
                        1
                    } else {
                        count(value)
                    }
                })
                .sum(),
            Value::Array(entries) => entries.iter().map(count).sum(),
            _ => 0,
        }
    }
    document.get("assistant").map(count).unwrap_or(0)
}

fn read_source(root: &Path, path: &Path, limit: u64) -> Result<Vec<u8>, ImportError> {
    let resolved = path.canonicalize()?;
    if !resolved.starts_with(root) {
        return Err(ImportError::UnsafeSource);
    }
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(ImportError::UnsafeSource);
    }
    if metadata.len() > limit {
        return Err(ImportError::TooLarge);
    }
    let bytes = fs::read(path)?;
    if bytes.len() as u64 > limit {
        return Err(ImportError::TooLarge);
    }
    Ok(bytes)
}

fn provider_summary(config: &Value) -> Vec<String> {
    let assistant = &config["assistant"];
    let mut names = Vec::new();
    for key in ["namedProviders", "savedProfiles"] {
        if let Some(entries) = assistant[key].as_array() {
            for entry in entries {
                let name = entry["name"]
                    .as_str()
                    .or_else(|| entry["label"].as_str())
                    .unwrap_or("Unnamed provider");
                let endpoint = entry["endpoint"].as_str().unwrap_or("No endpoint");
                let model = entry["model"].as_str().unwrap_or("No model");
                names.push(format!("{name} · {endpoint} · {model}"));
            }
        }
    }
    names
}

fn destination(store: &NativeStore, prepared: &Prepared) -> Result<(String, bool), ImportError> {
    let prefix: String = prepared.source_profile.chars().take(24).collect();
    let base = format!("imported-{prefix}-{}", &prepared.fingerprint[..12]);
    for suffix in 0..100 {
        let name = if suffix == 0 {
            base.clone()
        } else {
            format!("{base}-{suffix}")
        };
        let path = store.native_root().join("profiles").join(&name);
        if !path.exists() {
            return Ok((name, false));
        }
        let resolved = path.canonicalize()?;
        let root = store.native_root().canonicalize()?;
        if !resolved.starts_with(&root) {
            return Err(ImportError::UnsafeDestination);
        }
        let manifest = path.join("import-manifest.json");
        if manifest.is_file() {
            if !manifest.canonicalize()?.starts_with(&root) {
                return Err(ImportError::UnsafeDestination);
            }
            if fs::metadata(&manifest)?.len() > 64 * 1024 {
                continue;
            }
            let bytes = fs::read(&manifest)?;
            if bytes.len() <= 64 * 1024 {
                if let Ok(previous) = serde_json::from_slice::<Value>(&bytes) {
                    if previous["fingerprint_sha256"] == prepared.fingerprint
                        && previous["source_profile"] == prepared.source_profile
                    {
                        return Ok((name, true));
                    }
                }
            }
        }
    }
    Err(ImportError::Store(StoreError::ImportCollision))
}

#[cfg(test)]
mod tests {
    use super::super::paths::NativeDataRoot;
    use super::*;
    use crate::assistant::credentials::{CredentialError, SessionCredentialStore};
    use std::time::{SystemTime, UNIX_EPOCH};
    use zeroize::Zeroizing;

    fn fixture() -> (PathBuf, LegacyImportRoot, NativeStore) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base =
            std::env::temp_dir().join(format!("buttonscli-import-{}-{nonce}", std::process::id()));
        let original = base.join("original");
        let profile = original.join("profiles/Work_Space");
        fs::create_dir_all(profile.join("themes")).unwrap();
        fs::write(
            original.join("active-profile.json"),
            include_bytes!("../../tests/fixtures/legacy/active-profile.json"),
        )
        .unwrap();
        fs::write(
            profile.join("config.json"),
            include_bytes!("../../tests/fixtures/legacy/profiles/Work_Space/config.json"),
        )
        .unwrap();
        fs::write(
            profile.join("themes/duplicate-a.json"),
            include_bytes!(
                "../../tests/fixtures/legacy/profiles/Work_Space/themes/duplicate-a.json"
            ),
        )
        .unwrap();
        fs::write(
            profile.join("themes/duplicate-b.json"),
            include_bytes!(
                "../../tests/fixtures/legacy/profiles/Work_Space/themes/duplicate-b.json"
            ),
        )
        .unwrap();
        fs::write(
            original.join("control-api.json"),
            r#"{"token":"FAKE-RUNTIME-KEY"}"#,
        )
        .unwrap();
        fs::write(
            original.join("session-history.json"),
            r#"{"password":"FAKE-HISTORY-KEY"}"#,
        )
        .unwrap();
        let native = base.join("native");
        let store = NativeStore::open(NativeDataRoot(native), original.clone()).unwrap();
        (base, LegacyImportRoot(original), store)
    }

    fn source_tree_digest(root: &Path) -> String {
        fn visit(root: &Path, current: &Path, digest: &mut Sha256) {
            let mut entries: Vec<_> = fs::read_dir(current)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect();
            entries.sort();
            for path in entries {
                if path.is_dir() {
                    visit(root, &path, digest);
                } else {
                    digest.update(
                        path.strip_prefix(root)
                            .unwrap()
                            .to_string_lossy()
                            .as_bytes(),
                    );
                    digest.update(fs::read(path).unwrap());
                }
            }
        }
        let mut digest = Sha256::new();
        visit(root, root, &mut digest);
        format!("{:x}", digest.finalize())
    }

    struct UnavailableCredentials;

    impl CredentialStore for UnavailableCredentials {
        fn put(&self, _: &str, _: &str) -> Result<(), CredentialError> {
            Err(CredentialError::Unavailable)
        }
        fn get(&self, _: &str) -> Result<Zeroizing<String>, CredentialError> {
            Err(CredentialError::Unavailable)
        }
        fn delete(&self, _: &str) -> Result<(), CredentialError> {
            Err(CredentialError::Unavailable)
        }
    }

    #[test]
    fn selected_key_transfer_uses_credential_store_without_serializing_values() {
        let (base, source, store) = fixture();
        let original_digest = source_tree_digest(&source.0);
        let preview = preview(&source, &store, None).unwrap();
        let plan = preview.credential_transfer_plan();
        let ImportCommit::Imported {
            store,
            mut preferences,
        } = commit(&source, &store, preview).unwrap()
        else {
            panic!("fresh import skipped");
        };
        let credentials = SessionCredentialStore::default();
        let result =
            transfer_credentials(&source, &plan, &store, &mut preferences, &credentials).unwrap();
        assert_eq!((result.imported, result.failed, result.revision), (1, 0, 2));
        let provider = &preferences.provider_settings.providers[0];
        assert_eq!(
            credentials
                .get(provider.credential_ref.as_deref().unwrap())
                .unwrap()
                .as_str(),
            "FAKE-KEY-NAMED-NOT-REAL"
        );
        for name in [
            "native.json",
            "legacy-compatible.json",
            "import-manifest.json",
        ] {
            let content = fs::read_to_string(store.profile_dir().join(name)).unwrap();
            assert!(!content.contains("FAKE-KEY"));
        }
        assert_eq!(source_tree_digest(&source.0), original_digest);
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn failed_key_transfer_does_not_claim_saved_credentials() {
        let (base, source, store) = fixture();
        let preview = preview(&source, &store, None).unwrap();
        let plan = preview.credential_transfer_plan();
        let ImportCommit::Imported {
            store,
            mut preferences,
        } = commit(&source, &store, preview).unwrap()
        else {
            panic!("fresh import skipped");
        };
        let result = transfer_credentials(
            &source,
            &plan,
            &store,
            &mut preferences,
            &UnavailableCredentials,
        )
        .unwrap();
        assert_eq!((result.imported, result.failed, result.revision), (0, 1, 1));
        assert!(preferences.provider_settings.providers[0]
            .credential_ref
            .is_none());
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn prior_import_loads_provider_metadata_from_safe_compatibility_file() {
        let (base, source, store) = fixture();
        let preview = preview(&source, &store, None).unwrap();
        let ImportCommit::Imported { store, .. } = commit(&source, &store, preview).unwrap() else {
            panic!("fresh import skipped");
        };
        let path = store.profile_dir().join("native.json");
        let mut native: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        native["preferences"]
            .as_object_mut()
            .unwrap()
            .remove("provider_settings");
        fs::write(&path, serde_json::to_vec(&native).unwrap()).unwrap();
        let loaded = store.load().unwrap().unwrap();
        assert_eq!(
            loaded.preferences.provider_settings.providers[0].id,
            "fixture-local"
        );
        assert!(loaded.preferences.provider_settings.providers[0]
            .credential_ref
            .is_none());
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn preview_and_commit_preserve_original_and_exclude_secrets_and_runtime_files() {
        let (base, source, store) = fixture();
        let theme_path = source.0.join("profiles/Work_Space/themes/duplicate-a.json");
        let mut theme: Value = serde_json::from_slice(&fs::read(&theme_path).unwrap()).unwrap();
        theme["metadata"]["apiKey"] = Value::String("FAKE-THEME-KEY".into());
        fs::write(&theme_path, serde_json::to_vec(&theme).unwrap()).unwrap();
        let source_digest = source_tree_digest(&source.0);
        let source_config = fs::read(source.0.join("profiles/Work_Space/config.json")).unwrap();
        let source_theme = fs::read(&theme_path).unwrap();
        let preview = preview(&source, &store, None).unwrap();
        assert_eq!(preview.source_profile, "Work_Space");
        assert_eq!(preview.command_presets, 2);
        assert_eq!(preview.ssh_presets, 0);
        assert_eq!(preview.themes, 2);
        assert!(!preview.already_imported);
        assert_eq!(preview.provider_metadata.len(), 2);
        assert_eq!(preview.credential_count, 3);
        let destination = preview.destination_profile.clone();
        let ImportCommit::Imported {
            store: imported,
            preferences,
        } = commit(&source, &store, preview).unwrap()
        else {
            panic!("fresh import skipped");
        };
        assert_eq!(imported.profile_name(), destination);
        assert_eq!(preferences.presets[0].command, "  echo café  ");
        let imported_dir = imported.profile_dir();
        let native = fs::read_to_string(imported_dir.join("native.json")).unwrap();
        let compatible = fs::read_to_string(imported_dir.join("legacy-compatible.json")).unwrap();
        let manifest = fs::read_to_string(imported_dir.join("import-manifest.json")).unwrap();
        for data in [&native, &compatible, &manifest] {
            assert!(!data.contains("FAKE-KEY"));
            assert!(!data.contains("FAKE-RUNTIME-KEY"));
            assert!(!data.contains("FAKE-HISTORY-KEY"));
        }
        assert!(
            !fs::read_to_string(imported_dir.join("themes/duplicate-a.json"))
                .unwrap()
                .contains("FAKE-THEME-KEY")
        );
        assert!(!imported_dir.join("control-api.json").exists());
        assert!(!imported_dir.join("session-history.json").exists());
        assert_eq!(
            fs::read(source.0.join("profiles/Work_Space/config.json")).unwrap(),
            source_config
        );
        assert_eq!(
            fs::read(source.0.join("profiles/Work_Space/themes/duplicate-a.json")).unwrap(),
            source_theme
        );
        assert_eq!(
            fs::read_to_string(source.0.join("control-api.json")).unwrap(),
            r#"{"token":"FAKE-RUNTIME-KEY"}"#
        );
        assert_eq!(source_tree_digest(&source.0), source_digest);
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn repeated_snapshot_is_idempotent_and_changed_theme_gets_new_profile() {
        let (base, source, store) = fixture();
        let first = preview(&source, &store, None).unwrap();
        let original_destination = first.destination_profile.clone();
        let ImportCommit::Imported {
            store: imported,
            mut preferences,
        } = commit(&source, &store, first).unwrap()
        else {
            panic!("fresh import skipped");
        };
        preferences.presets[0].label = "native edit".into();
        imported.save(Some(1), &preferences).unwrap();
        let repeat = preview(&source, &imported, None).unwrap();
        assert!(repeat.already_imported);
        assert!(matches!(
            commit(&source, &imported, repeat).unwrap(),
            ImportCommit::AlreadyImported
        ));
        assert_eq!(
            imported.load().unwrap().unwrap().preferences.presets[0].label,
            "native edit"
        );
        let config_path = source.0.join("profiles/Work_Space/config.json");
        let config = fs::read_to_string(&config_path).unwrap();
        fs::write(
            &config_path,
            config.replace("FAKE-KEY-NAMED-NOT-REAL", "FAKE-KEY-ROTATED-NOT-REAL"),
        )
        .unwrap();
        let key_only_change = preview(&source, &imported, None).unwrap();
        assert!(key_only_change.already_imported);
        let theme_path = source.0.join("profiles/Work_Space/themes/duplicate-a.json");
        let mut theme: Value = serde_json::from_slice(&fs::read(&theme_path).unwrap()).unwrap();
        theme["metadata"]["name"] = Value::String("Updated theme".into());
        fs::write(&theme_path, serde_json::to_vec(&theme).unwrap()).unwrap();
        let changed = preview(&source, &imported, None).unwrap();
        assert!(!changed.already_imported);
        assert_ne!(changed.destination_profile, original_destination);
        assert_eq!(
            imported.load().unwrap().unwrap().preferences.presets[0].label,
            "native edit"
        );
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn changed_source_and_failed_publication_leave_no_new_profile() {
        let (base, source, store) = fixture();
        let stale = preview(&source, &store, None).unwrap();
        let config = source.0.join("profiles/Work_Space/config.json");
        let original = fs::read(&config).unwrap();
        fs::write(&config, b"{}").unwrap();
        assert!(matches!(
            commit(&source, &store, stale),
            Err(ImportError::Store(StoreError::SourceChanged))
        ));
        fs::write(&config, &original).unwrap();
        let fresh = preview(&source, &store, None).unwrap();
        let destination = fresh.destination_profile.clone();
        // Force the final metadata replacement to fail after the staged directory is published.
        fs::create_dir_all(store.native_root().join("active-profile.json")).unwrap();
        assert!(commit(&source, &store, fresh).is_err());
        assert!(!store
            .native_root()
            .join("profiles")
            .join(destination)
            .exists());
        assert_eq!(fs::read(&config).unwrap(), original);
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn named_additional_profile_import_does_not_use_root_fallback() {
        let (base, source, store) = fixture();
        let other = source.0.join("profiles/Other");
        fs::create_dir_all(&other).unwrap();
        fs::write(
            other.join("config.json"),
            r#"{"presets":[{"label":"Other","command":"true","sendEnter":false}]}"#,
        )
        .unwrap();
        assert_eq!(
            source.list_profile_names().unwrap(),
            vec!["Other", "Work_Space"]
        );
        let selected = preview(&source, &store, Some("Other")).unwrap();
        assert_eq!(selected.source_profile, "Other");
        assert_eq!(selected.command_presets, 1);
        assert!(!selected.source_fallback);
        let ImportCommit::Imported { preferences, .. } = commit(&source, &store, selected).unwrap()
        else {
            panic!("named profile skipped");
        };
        assert_eq!(preferences.presets[0].label, "Other");
        fs::remove_file(other.join("config.json")).unwrap();
        assert!(matches!(
            source.resolve_named("Other"),
            Err(PathError::MissingConfig)
        ));
        fs::remove_dir_all(base).unwrap();
    }
}
