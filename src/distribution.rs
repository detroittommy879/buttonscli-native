//! Fail-closed verification primitives for the separate native release family.
//!
//! The production trust list is intentionally empty until its public key has
//! been supplied and backed up outside Git. Tests inject deterministic throwaway
//! keys; no release endpoint or installer calls these functions yet.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive};

const PRODUCT_ID: &str = "com.buttonscli.native";
const MANIFEST_SCHEMA_VERSION: u32 = 1;
const MAX_MANIFEST_BYTES: usize = 16 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_TRUSTED_KEYS: usize = 16;
const MAX_ARCHIVE_ENTRIES: usize = 10_000;
const MAX_ARCHIVE_FILE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ARCHIVE_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_ARCHIVE_PATH_BYTES: usize = 4096;
const MAX_RELEASE_VERSION_BYTES: usize = 128;

#[derive(Clone, Copy, Debug)]
pub(crate) struct TrustedSigningKey {
    key_id: &'static str,
    public_key: [u8; 32],
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct ReleaseManifest {
    schema_version: u32,
    product_id: String,
    version: String,
    target_os: String,
    target_arch: String,
    artifact_basename: String,
    artifact_size: u64,
    sha256: String,
    minimum_updater_version: String,
    channel: String,
    key_id: String,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ReleaseTarget<'a> {
    pub(crate) os: &'a str,
    pub(crate) arch: &'a str,
}

pub(crate) struct ReleaseBuildRequest<'a> {
    pub(crate) source_directory: &'a Path,
    pub(crate) output_directory: &'a Path,
    pub(crate) signing_key_file: &'a Path,
    pub(crate) version: &'a str,
    pub(crate) target: ReleaseTarget<'a>,
    pub(crate) minimum_updater_version: &'a str,
    pub(crate) channel: &'a str,
    pub(crate) key_id: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReleaseBuildResult {
    pub(crate) artifact_path: PathBuf,
    pub(crate) manifest_path: PathBuf,
    pub(crate) signature_path: PathBuf,
    pub(crate) public_key: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum VerifyError {
    ManifestTooLarge,
    SignatureLength,
    TrustStoreTooLarge,
    InvalidTrustStore,
    UntrustedSignature,
    InvalidManifest,
    NonCanonicalManifest,
    KeyIdMismatch,
    UnsupportedSchema,
    WrongProduct,
    WrongTarget,
    InvalidVersion,
    MissingPackageExecutable,
    VersionNotNewer,
    InvalidChannel,
    InvalidArtifactName,
    ArtifactTooLarge,
    EmptyArtifact,
    ArtifactSizeMismatch,
    DigestMismatch,
    UpdaterTooOld,
    InvalidArchivePath,
    InvalidArchive,
    TooManyArchiveEntries,
    ArchiveFileTooLarge,
    ArchiveTooLarge,
    UnsafeStageRoot,
    VersionAlreadyStaged,
    StageIo,
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "native release verification failed: {self:?}")
    }
}

impl std::error::Error for VerifyError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReleaseBuildError {
    InvalidMetadata,
    UnsafeSource,
    SigningKey,
    SigningKeyInsideSource,
    TooManyFiles,
    FileTooLarge,
    PackageTooLarge,
    MissingPackageExecutable,
    InvalidArchive,
    OutputAlreadyExists,
    Io,
}

impl std::fmt::Display for ReleaseBuildError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "native package build failed: {self:?}")
    }
}

impl std::error::Error for ReleaseBuildError {}

// Do not invent or check in a release identity. This empty production allowlist
// rejects every signature until a user-supplied public key is reviewed.
const COMPILED_TRUSTED_KEYS: &[TrustedSigningKey] = &[];

pub(crate) fn verify_with_compiled_keys(
    manifest_bytes: &[u8],
    signature_bytes: &[u8],
    expected_target: ReleaseTarget<'_>,
    installed_version: &str,
    updater_version: &str,
    artifact: &[u8],
) -> Result<ReleaseManifest, VerifyError> {
    verify_signed_package(
        manifest_bytes,
        signature_bytes,
        COMPILED_TRUSTED_KEYS,
        expected_target,
        installed_version,
        updater_version,
        artifact,
    )
}

/// Verify the exact manifest byte sequence before parsing or trusting fields.
/// The key ID is checked only after one allowlisted key validates the signature.
pub(crate) fn verify_signed_manifest(
    manifest_bytes: &[u8],
    signature_bytes: &[u8],
    trusted_keys: &[TrustedSigningKey],
) -> Result<ReleaseManifest, VerifyError> {
    if manifest_bytes.len() > MAX_MANIFEST_BYTES {
        return Err(VerifyError::ManifestTooLarge);
    }
    if signature_bytes.len() != 64 {
        return Err(VerifyError::SignatureLength);
    }
    if trusted_keys.len() > MAX_TRUSTED_KEYS {
        return Err(VerifyError::TrustStoreTooLarge);
    }
    if trusted_keys.iter().enumerate().any(|(index, key)| {
        key.key_id.is_empty()
            || key.key_id.len() > 64
            || trusted_keys[..index]
                .iter()
                .any(|prior| prior.key_id == key.key_id)
    }) {
        return Err(VerifyError::InvalidTrustStore);
    }

    let signature =
        Signature::from_slice(signature_bytes).map_err(|_| VerifyError::SignatureLength)?;
    let verified_key_id = trusted_keys
        .iter()
        .find_map(|trusted| {
            let key = VerifyingKey::from_bytes(&trusted.public_key).ok()?;
            key.verify_strict(manifest_bytes, &signature)
                .ok()
                .map(|()| trusted.key_id)
        })
        .ok_or(VerifyError::UntrustedSignature)?;

    let manifest: ReleaseManifest =
        serde_json::from_slice(manifest_bytes).map_err(|_| VerifyError::InvalidManifest)?;
    if manifest.key_id != verified_key_id {
        return Err(VerifyError::KeyIdMismatch);
    }
    let canonical = serde_json::to_vec(&manifest).map_err(|_| VerifyError::InvalidManifest)?;
    if canonical != manifest_bytes {
        return Err(VerifyError::NonCanonicalManifest);
    }
    Ok(manifest)
}

/// Verify the detached signature, package metadata, and archive contents.
pub(crate) fn verify_signed_package(
    manifest_bytes: &[u8],
    signature_bytes: &[u8],
    trusted_keys: &[TrustedSigningKey],
    expected_target: ReleaseTarget<'_>,
    installed_version: &str,
    updater_version: &str,
    artifact: &[u8],
) -> Result<ReleaseManifest, VerifyError> {
    let manifest = verify_signed_manifest(manifest_bytes, signature_bytes, trusted_keys)?;
    verify_package_bytes(
        &manifest,
        expected_target,
        installed_version,
        updater_version,
        artifact,
    )?;
    validate_package_archive(artifact)?;
    validate_target_package(artifact, expected_target.os)?;
    Ok(manifest)
}

/// Verify and extract a package into a new, versioned directory without
/// changing any active-version state. Callers must activate it separately
/// after the running application has closed.
pub(crate) fn verify_and_stage_package(
    manifest_bytes: &[u8],
    signature_bytes: &[u8],
    trusted_keys: &[TrustedSigningKey],
    expected_target: ReleaseTarget<'_>,
    installed_version: &str,
    updater_version: &str,
    artifact: &[u8],
    staging_root: &Path,
) -> Result<PathBuf, VerifyError> {
    let manifest = verify_signed_package(
        manifest_bytes,
        signature_bytes,
        trusted_keys,
        expected_target,
        installed_version,
        updater_version,
        artifact,
    )?;
    stage_verified_archive(&manifest, artifact, staging_root)
}

fn stage_verified_archive(
    manifest: &ReleaseManifest,
    artifact: &[u8],
    staging_root: &Path,
) -> Result<PathBuf, VerifyError> {
    ensure_real_directory(staging_root)?;
    let root = fs::canonicalize(staging_root).map_err(|_| VerifyError::StageIo)?;
    let versions_root = root.join("versions");
    match fs::create_dir(&versions_root) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            ensure_real_directory(&versions_root)?;
        }
        Err(_) => return Err(VerifyError::StageIo),
    }

    let destination = versions_root.join(&manifest.version);
    if fs::symlink_metadata(&destination).is_ok() {
        return Err(VerifyError::VersionAlreadyStaged);
    }

    let temporary = (0..8)
        .find_map(|_| {
            let path = versions_root.join(format!(
                ".staging-{}-{:016x}",
                manifest.version,
                fastrand::u64(..)
            ));
            if fs::create_dir(&path).is_ok() {
                Some(path)
            } else {
                None
            }
        })
        .ok_or(VerifyError::StageIo)?;

    let extraction = extract_archive(artifact, &temporary);
    if let Err(error) = extraction {
        let _ = fs::remove_dir_all(&temporary);
        return Err(error);
    }
    fs::rename(&temporary, &destination).map_err(|_| {
        let _ = fs::remove_dir_all(&temporary);
        VerifyError::StageIo
    })?;
    Ok(destination)
}

fn ensure_real_directory(path: &Path) -> Result<(), VerifyError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| VerifyError::UnsafeStageRoot)?;
    #[cfg(windows)]
    use std::os::windows::fs::MetadataExt;
    #[cfg(windows)]
    let is_reparse_point = metadata.file_attributes() & 0x400 != 0;
    #[cfg(not(windows))]
    let is_reparse_point = false;
    if metadata.file_type().is_symlink() || is_reparse_point || !metadata.is_dir() {
        return Err(VerifyError::UnsafeStageRoot);
    }
    Ok(())
}

fn extract_archive(artifact: &[u8], destination: &Path) -> Result<(), VerifyError> {
    let mut archive =
        ZipArchive::new(Cursor::new(artifact)).map_err(|_| VerifyError::InvalidArchive)?;
    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|_| VerifyError::InvalidArchive)?;
        let archive_name = file.name().to_owned();
        validate_archive_entry(&archive_name)?;
        let relative_name = archive_name.strip_suffix('/').unwrap_or(&archive_name);
        let relative_path = relative_name
            .split('/')
            .fold(PathBuf::new(), |path, component| path.join(component));
        let output_path = destination.join(relative_path);
        if file.is_dir() {
            fs::create_dir_all(&output_path).map_err(|_| VerifyError::StageIo)?;
            continue;
        }

        let parent = output_path
            .parent()
            .ok_or(VerifyError::InvalidArchivePath)?;
        fs::create_dir_all(parent).map_err(|_| VerifyError::StageIo)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output_path)
            .map_err(|_| VerifyError::StageIo)?;
        let declared_size = file.size();
        #[cfg(unix)]
        let mode = file.unix_mode();
        let copied = std::io::copy(&mut file.take(declared_size.saturating_add(1)), &mut output)
            .map_err(|_| VerifyError::InvalidArchive)?;
        if copied != declared_size {
            return Err(VerifyError::InvalidArchive);
        }
        output.sync_all().map_err(|_| VerifyError::StageIo)?;
        #[cfg(unix)]
        if let Some(mode) = mode {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&output_path, fs::Permissions::from_mode(mode & 0o755))
                .map_err(|_| VerifyError::StageIo)?;
        }
    }
    Ok(())
}

/// Validate signed package metadata and its downloaded bytes before extraction.
fn verify_package_bytes(
    manifest: &ReleaseManifest,
    expected_target: ReleaseTarget<'_>,
    installed_version: &str,
    updater_version: &str,
    artifact: &[u8],
) -> Result<(), VerifyError> {
    if manifest.schema_version != MANIFEST_SCHEMA_VERSION {
        return Err(VerifyError::UnsupportedSchema);
    }
    if manifest.product_id != PRODUCT_ID {
        return Err(VerifyError::WrongProduct);
    }
    if manifest.target_os != expected_target.os || manifest.target_arch != expected_target.arch {
        return Err(VerifyError::WrongTarget);
    }
    if manifest.version.len() > MAX_RELEASE_VERSION_BYTES {
        return Err(VerifyError::InvalidVersion);
    }

    let release_version =
        semver::Version::parse(&manifest.version).map_err(|_| VerifyError::InvalidVersion)?;
    let installed_version =
        semver::Version::parse(installed_version).map_err(|_| VerifyError::InvalidVersion)?;
    let updater_version =
        semver::Version::parse(updater_version).map_err(|_| VerifyError::InvalidVersion)?;
    let minimum_updater_version = semver::Version::parse(&manifest.minimum_updater_version)
        .map_err(|_| VerifyError::InvalidVersion)?;
    if release_version <= installed_version {
        return Err(VerifyError::VersionNotNewer);
    }
    if updater_version < minimum_updater_version {
        return Err(VerifyError::UpdaterTooOld);
    }
    if !matches!(manifest.channel.as_str(), "stable" | "beta")
        || (manifest.channel == "stable" && !release_version.pre.is_empty())
    {
        return Err(VerifyError::InvalidChannel);
    }

    let expected_basename = format!(
        "buttonscli-native-{}-{}-{}.zip",
        expected_target.os, expected_target.arch, manifest.version
    );
    if manifest.artifact_basename != expected_basename {
        return Err(VerifyError::InvalidArtifactName);
    }
    if manifest.artifact_size > MAX_ARTIFACT_BYTES {
        return Err(VerifyError::ArtifactTooLarge);
    }
    if artifact.is_empty() {
        return Err(VerifyError::EmptyArtifact);
    }
    if manifest.artifact_size != artifact.len() as u64 {
        return Err(VerifyError::ArtifactSizeMismatch);
    }
    let digest = format!("{:x}", Sha256::digest(artifact));
    if manifest.sha256 != digest {
        return Err(VerifyError::DigestMismatch);
    }
    Ok(())
}

/// Reject archive entry names that could escape a portable package root.
pub(crate) fn validate_archive_entry(name: &str) -> Result<(), VerifyError> {
    if name.is_empty()
        || name.len() > MAX_ARCHIVE_PATH_BYTES
        || name.contains('\0')
        || name.contains('\\')
        || name.starts_with('/')
        || name.ends_with(':')
        || name.contains(':')
    {
        return Err(VerifyError::InvalidArchivePath);
    }
    let entry = name.strip_suffix('/').unwrap_or(name);
    if entry.is_empty() {
        return Err(VerifyError::InvalidArchivePath);
    }
    for component in entry.split('/') {
        if component.is_empty()
            || component == "."
            || component == ".."
            || component.ends_with('.')
            || component.ends_with(' ')
            || component.encode_utf16().count() > 255
            || component.chars().any(|character| {
                character.is_control() || matches!(character, '<' | '>' | '"' | '|' | '?' | '*')
            })
            || is_windows_reserved_name(component)
        {
            return Err(VerifyError::InvalidArchivePath);
        }
    }
    Ok(())
}

/// Preflight a portable ZIP before any updater is allowed to extract it.
///
/// This reads every entry to validate decompression and CRC data while keeping
/// expanded sizes bounded. Extraction itself remains a separate staged step.
fn validate_package_archive(artifact: &[u8]) -> Result<(), VerifyError> {
    let mut archive =
        ZipArchive::new(Cursor::new(artifact)).map_err(|_| VerifyError::InvalidArchive)?;
    if archive.is_empty() {
        return Err(VerifyError::InvalidArchive);
    }
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(VerifyError::TooManyArchiveEntries);
    }

    let mut paths = HashMap::<String, bool>::with_capacity(archive.len());
    let mut implicit_directories = HashSet::<String>::with_capacity(archive.len());
    let mut total_size = 0_u64;
    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|_| VerifyError::InvalidArchive)?;
        let name = file.name();
        validate_archive_entry(name)?;
        if file.is_symlink() {
            return Err(VerifyError::InvalidArchivePath);
        }

        let is_directory = file.is_dir();
        if let Some(mode) = file.unix_mode() {
            let kind = mode & 0o170000;
            let expected_kind = if is_directory { 0o040000 } else { 0o100000 };
            if kind != 0 && kind != expected_kind {
                return Err(VerifyError::InvalidArchivePath);
            }
        }

        let normalized = name.strip_suffix('/').unwrap_or(name).to_lowercase();
        if paths.contains_key(&normalized) {
            return Err(VerifyError::InvalidArchivePath);
        }
        let mut parent = normalized.as_str();
        while let Some((ancestor, _)) = parent.rsplit_once('/') {
            if paths.get(ancestor) == Some(&false) {
                return Err(VerifyError::InvalidArchivePath);
            }
            implicit_directories.insert(ancestor.to_owned());
            parent = ancestor;
        }
        if !is_directory && implicit_directories.contains(&normalized) {
            return Err(VerifyError::InvalidArchivePath);
        }
        paths.insert(normalized, is_directory);

        let declared_size = file.size();
        if is_directory && declared_size != 0 {
            return Err(VerifyError::InvalidArchive);
        }
        if declared_size > MAX_ARCHIVE_FILE_BYTES {
            return Err(VerifyError::ArchiveFileTooLarge);
        }
        total_size = total_size
            .checked_add(declared_size)
            .ok_or(VerifyError::ArchiveTooLarge)?;
        if total_size > MAX_ARCHIVE_TOTAL_BYTES {
            return Err(VerifyError::ArchiveTooLarge);
        }
    }

    // Check the complete metadata budget before decompressing any content,
    // then read every entry to force CRC and stream validation.
    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|_| VerifyError::InvalidArchive)?;
        let declared_size = file.size();
        let bytes_read = std::io::copy(
            &mut file.take(declared_size.saturating_add(1)),
            &mut std::io::sink(),
        )
        .map_err(|_| VerifyError::InvalidArchive)?;
        if bytes_read != declared_size {
            return Err(VerifyError::InvalidArchive);
        }
    }
    Ok(())
}

/// The first native release target is the Windows portable ZIP. Require its
/// executable at the package root so a signed but inert ZIP cannot be staged.
fn validate_target_package(artifact: &[u8], target_os: &str) -> Result<(), VerifyError> {
    if target_os != "windows" {
        return Ok(());
    }
    let mut archive =
        ZipArchive::new(Cursor::new(artifact)).map_err(|_| VerifyError::InvalidArchive)?;
    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|_| VerifyError::InvalidArchive)?;
        if file.name().eq_ignore_ascii_case("buttonscli.exe") && !file.is_dir() {
            return Ok(());
        }
    }
    Err(VerifyError::MissingPackageExecutable)
}

struct ReleaseFile {
    source: PathBuf,
    archive_name: String,
    size: u64,
    mode: u32,
}

/// Build a signed package from a prepared release directory. The caller must
/// provide the key file; this function never creates or persists signing keys.
pub(crate) fn build_signed_release_package(
    request: ReleaseBuildRequest<'_>,
) -> Result<ReleaseBuildResult, ReleaseBuildError> {
    validate_release_build_metadata(&request)?;
    let source_metadata = fs::symlink_metadata(request.source_directory)
        .map_err(|_| ReleaseBuildError::UnsafeSource)?;
    if is_link_or_reparse(&source_metadata) || !source_metadata.is_dir() {
        return Err(ReleaseBuildError::UnsafeSource);
    }
    let source_root =
        fs::canonicalize(request.source_directory).map_err(|_| ReleaseBuildError::UnsafeSource)?;
    let output_directory = resolve_new_output_directory(request.output_directory)?;
    if output_directory.starts_with(&source_root) {
        return Err(ReleaseBuildError::UnsafeSource);
    }

    let key_metadata = fs::symlink_metadata(request.signing_key_file)
        .map_err(|_| ReleaseBuildError::SigningKey)?;
    if is_link_or_reparse(&key_metadata) || !key_metadata.is_file() || key_metadata.len() != 32 {
        return Err(ReleaseBuildError::SigningKey);
    }
    let key_path =
        fs::canonicalize(request.signing_key_file).map_err(|_| ReleaseBuildError::SigningKey)?;
    if key_path.starts_with(&source_root) {
        return Err(ReleaseBuildError::SigningKeyInsideSource);
    }

    let files = collect_release_files(&source_root)?;
    if files.is_empty() {
        return Err(ReleaseBuildError::UnsafeSource);
    }
    if request.target.os == "windows"
        && !files.iter().any(|file| {
            file.archive_name.eq_ignore_ascii_case("buttonscli.exe")
                && !file.archive_name.contains('/')
        })
    {
        return Err(ReleaseBuildError::MissingPackageExecutable);
    }

    let mut seed = zeroize::Zeroizing::new([0_u8; 32]);
    File::open(&key_path)
        .and_then(|mut file| file.read_exact(&mut seed[..]))
        .map_err(|_| ReleaseBuildError::SigningKey)?;
    let signing_key = SigningKey::from_bytes(&seed);
    let public_key = signing_key.verifying_key().to_bytes();
    let artifact = create_release_archive(&files)?;
    validate_package_archive(&artifact).map_err(|_| ReleaseBuildError::InvalidArchive)?;
    validate_target_package(&artifact, request.target.os)
        .map_err(|_| ReleaseBuildError::MissingPackageExecutable)?;

    let manifest = ReleaseManifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        product_id: PRODUCT_ID.into(),
        version: request.version.into(),
        target_os: request.target.os.into(),
        target_arch: request.target.arch.into(),
        artifact_basename: format!(
            "buttonscli-native-{}-{}-{}.zip",
            request.target.os, request.target.arch, request.version
        ),
        artifact_size: artifact.len() as u64,
        sha256: format!("{:x}", Sha256::digest(&artifact)),
        minimum_updater_version: request.minimum_updater_version.into(),
        channel: request.channel.into(),
        key_id: request.key_id.into(),
    };
    let manifest_bytes =
        serde_json::to_vec(&manifest).map_err(|_| ReleaseBuildError::InvalidMetadata)?;
    if manifest_bytes.len() > MAX_MANIFEST_BYTES {
        return Err(ReleaseBuildError::InvalidMetadata);
    }
    let signature_bytes = signing_key.sign(&manifest_bytes).to_bytes();

    let staging_directory = create_release_staging_directory(&output_directory)?;
    let result = (|| {
        let artifact_path = staging_directory.join(&manifest.artifact_basename);
        let manifest_path = staging_directory.join("release.json");
        let signature_path = staging_directory.join("release.sig");
        write_new_file(&artifact_path, &artifact)?;
        write_new_file(&manifest_path, &manifest_bytes)?;
        write_new_file(&signature_path, &signature_bytes)?;
        fs::rename(&staging_directory, &output_directory).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                ReleaseBuildError::OutputAlreadyExists
            } else {
                ReleaseBuildError::Io
            }
        })?;
        Ok(ReleaseBuildResult {
            artifact_path: output_directory.join(&manifest.artifact_basename),
            manifest_path: output_directory.join("release.json"),
            signature_path: output_directory.join("release.sig"),
            public_key,
        })
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging_directory);
    }
    result
}

fn validate_release_build_metadata(
    request: &ReleaseBuildRequest<'_>,
) -> Result<(), ReleaseBuildError> {
    if request.version.len() > MAX_RELEASE_VERSION_BYTES
        || request.minimum_updater_version.len() > MAX_RELEASE_VERSION_BYTES
    {
        return Err(ReleaseBuildError::InvalidMetadata);
    }
    let version =
        semver::Version::parse(request.version).map_err(|_| ReleaseBuildError::InvalidMetadata)?;
    semver::Version::parse(request.minimum_updater_version)
        .map_err(|_| ReleaseBuildError::InvalidMetadata)?;
    let valid_target = [&request.target.os, &request.target.arch]
        .iter()
        .all(|value| {
            !value.is_empty()
                && value.len() <= 32
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        });
    if !valid_target
        || request.key_id.is_empty()
        || request.key_id.len() > 64
        || !request
            .key_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        || !matches!(request.channel, "stable" | "beta")
        || (request.channel == "stable" && !version.pre.is_empty())
    {
        return Err(ReleaseBuildError::InvalidMetadata);
    }
    Ok(())
}

fn collect_release_files(root: &Path) -> Result<Vec<ReleaseFile>, ReleaseBuildError> {
    let mut directories = vec![root.to_owned()];
    let mut files = Vec::new();
    let mut total_size = 0_u64;
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory).map_err(|_| ReleaseBuildError::UnsafeSource)? {
            let entry = entry.map_err(|_| ReleaseBuildError::UnsafeSource)?;
            let path = entry.path();
            let metadata =
                fs::symlink_metadata(&path).map_err(|_| ReleaseBuildError::UnsafeSource)?;
            if is_link_or_reparse(&metadata) {
                return Err(ReleaseBuildError::UnsafeSource);
            }
            if metadata.is_dir() {
                directories.push(path);
                continue;
            }
            if !metadata.is_file() {
                return Err(ReleaseBuildError::UnsafeSource);
            }

            let relative = path
                .strip_prefix(root)
                .map_err(|_| ReleaseBuildError::UnsafeSource)?;
            let archive_name = relative
                .components()
                .map(|component| component.as_os_str().to_str())
                .collect::<Option<Vec<_>>>()
                .ok_or(ReleaseBuildError::UnsafeSource)?
                .join("/");
            validate_archive_entry(&archive_name).map_err(|_| ReleaseBuildError::UnsafeSource)?;
            if metadata.len() > MAX_ARCHIVE_FILE_BYTES {
                return Err(ReleaseBuildError::FileTooLarge);
            }
            total_size = total_size
                .checked_add(metadata.len())
                .ok_or(ReleaseBuildError::PackageTooLarge)?;
            if total_size > MAX_ARTIFACT_BYTES {
                return Err(ReleaseBuildError::PackageTooLarge);
            }
            if files.len() == MAX_ARCHIVE_ENTRIES {
                return Err(ReleaseBuildError::TooManyFiles);
            }
            let mode = release_file_mode(&metadata, relative);
            files.push(ReleaseFile {
                source: path,
                archive_name,
                size: metadata.len(),
                mode,
            });
        }
    }
    files.sort_by(|left, right| left.archive_name.cmp(&right.archive_name));
    Ok(files)
}

fn release_file_mode(metadata: &fs::Metadata, relative: &Path) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = relative;
        metadata.permissions().mode() & 0o755
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        if relative
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("buttonscli.exe"))
        {
            0o755
        } else {
            0o644
        }
    }
}

fn create_release_archive(files: &[ReleaseFile]) -> Result<Vec<u8>, ReleaseBuildError> {
    let cursor = Cursor::new(Vec::new());
    let mut archive = zip::ZipWriter::new(cursor);
    for entry in files {
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(entry.mode);
        archive
            .start_file(&entry.archive_name, options)
            .map_err(|_| ReleaseBuildError::InvalidArchive)?;
        let metadata =
            fs::symlink_metadata(&entry.source).map_err(|_| ReleaseBuildError::UnsafeSource)?;
        if is_link_or_reparse(&metadata) || !metadata.is_file() || metadata.len() != entry.size {
            return Err(ReleaseBuildError::UnsafeSource);
        }
        let source = File::open(&entry.source).map_err(|_| ReleaseBuildError::UnsafeSource)?;
        let copied = std::io::copy(&mut source.take(entry.size.saturating_add(1)), &mut archive)
            .map_err(|_| ReleaseBuildError::UnsafeSource)?;
        if copied != entry.size {
            return Err(ReleaseBuildError::UnsafeSource);
        }
    }
    let artifact = archive
        .finish()
        .map_err(|_| ReleaseBuildError::InvalidArchive)?
        .into_inner();
    if artifact.len() as u64 > MAX_ARTIFACT_BYTES {
        return Err(ReleaseBuildError::PackageTooLarge);
    }
    Ok(artifact)
}

fn is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    use std::os::windows::fs::MetadataExt;
    #[cfg(windows)]
    let is_reparse_point = metadata.file_attributes() & 0x400 != 0;
    #[cfg(not(windows))]
    let is_reparse_point = false;
    metadata.file_type().is_symlink() || is_reparse_point
}

fn resolve_new_output_directory(path: &Path) -> Result<PathBuf, ReleaseBuildError> {
    let leaf = path.file_name().ok_or(ReleaseBuildError::InvalidMetadata)?;
    if leaf == "." || leaf == ".." {
        return Err(ReleaseBuildError::InvalidMetadata);
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let metadata = fs::symlink_metadata(parent).map_err(|_| ReleaseBuildError::Io)?;
    if is_link_or_reparse(&metadata) || !metadata.is_dir() {
        return Err(ReleaseBuildError::Io);
    }
    let parent = fs::canonicalize(parent).map_err(|_| ReleaseBuildError::Io)?;
    let output = parent.join(leaf);
    match fs::symlink_metadata(&output) {
        Ok(_) => return Err(ReleaseBuildError::OutputAlreadyExists),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(ReleaseBuildError::Io),
    }
    Ok(output)
}

fn create_release_staging_directory(output_directory: &Path) -> Result<PathBuf, ReleaseBuildError> {
    let parent = output_directory
        .parent()
        .ok_or(ReleaseBuildError::InvalidMetadata)?;
    for _ in 0..8 {
        let staging = parent.join(format!(".native-release-{:016x}", fastrand::u64(..)));
        match fs::create_dir(&staging) {
            Ok(()) => return Ok(staging),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(ReleaseBuildError::Io),
        }
    }
    Err(ReleaseBuildError::Io)
}

fn write_new_file(path: &Path, contents: &[u8]) -> Result<(), ReleaseBuildError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| ReleaseBuildError::Io)?;
    file.write_all(contents)
        .map_err(|_| ReleaseBuildError::Io)?;
    file.sync_all().map_err(|_| ReleaseBuildError::Io)?;
    Ok(())
}

fn is_windows_reserved_name(component: &str) -> bool {
    let stem = component
        .split_once('.')
        .map_or(component, |(stem, _)| stem)
        .to_ascii_uppercase();
    matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        stem.strip_prefix(prefix).is_some_and(|suffix| {
            matches!(
                suffix,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    const TEST_KEY_ID: &str = "throwaway-tests-only";
    const TARGET: ReleaseTarget<'static> = ReleaseTarget {
        os: "windows",
        arch: "x86_64",
    };
    const ARTIFACT: &[u8] = b"deterministic native zip bytes";

    fn signing_key() -> SigningKey {
        SigningKey::from_bytes(&[0x5a; 32])
    }

    fn trusted_key(signing_key: &SigningKey) -> TrustedSigningKey {
        TrustedSigningKey {
            key_id: TEST_KEY_ID,
            public_key: signing_key.verifying_key().to_bytes(),
        }
    }

    fn valid_manifest() -> ReleaseManifest {
        ReleaseManifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            product_id: PRODUCT_ID.into(),
            version: "1.2.3".into(),
            target_os: TARGET.os.into(),
            target_arch: TARGET.arch.into(),
            artifact_basename: format!("buttonscli-native-{}-{}-1.2.3.zip", TARGET.os, TARGET.arch),
            artifact_size: ARTIFACT.len() as u64,
            sha256: format!("{:x}", Sha256::digest(ARTIFACT)),
            minimum_updater_version: "1.0.0".into(),
            channel: "stable".into(),
            key_id: TEST_KEY_ID.into(),
        }
    }

    fn sign(manifest: &ReleaseManifest, key: &SigningKey) -> (Vec<u8>, Vec<u8>) {
        let bytes = serde_json::to_vec(manifest).unwrap();
        let signature = key.sign(&bytes).to_bytes().to_vec();
        (bytes, signature)
    }

    fn test_archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut archive = zip::ZipWriter::new(cursor);
        for (name, contents) in entries {
            archive
                .start_file(
                    *name,
                    SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
                )
                .unwrap();
            archive.write_all(contents).unwrap();
        }
        archive.finish().unwrap().into_inner()
    }

    fn manifest_for_artifact(artifact: &[u8]) -> ReleaseManifest {
        let mut manifest = valid_manifest();
        manifest.artifact_size = artifact.len() as u64;
        manifest.sha256 = format!("{:x}", Sha256::digest(artifact));
        manifest
    }

    fn temporary_stage_root() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "buttonscli-distribution-test-{}-{:016x}",
            std::process::id(),
            fastrand::u64(..)
        ));
        fs::create_dir(&path).unwrap();
        path
    }

    fn verify_valid_package() -> Result<(), VerifyError> {
        let key = signing_key();
        let manifest = valid_manifest();
        let (bytes, signature) = sign(&manifest, &key);
        let verified = verify_signed_manifest(&bytes, &signature, &[trusted_key(&key)])?;
        verify_package_bytes(&verified, TARGET, "1.2.2", "1.0.0", ARTIFACT)
    }

    #[test]
    fn accepts_canonical_signed_package_with_throwaway_key() {
        assert_eq!(verify_valid_package(), Ok(()));
    }

    #[test]
    fn production_verifier_fails_closed_without_a_trust_root() {
        let key = signing_key();
        let (bytes, signature) = sign(&valid_manifest(), &key);
        assert_eq!(
            verify_with_compiled_keys(&bytes, &signature, TARGET, "1.2.2", "1.0.0", ARTIFACT,),
            Err(VerifyError::UntrustedSignature)
        );
    }

    #[test]
    fn rejects_manifest_and_signature_tampering() {
        let key = signing_key();
        let (bytes, signature) = sign(&valid_manifest(), &key);
        let mut changed_manifest = bytes.clone();
        changed_manifest[0] ^= 1;
        assert_eq!(
            verify_signed_manifest(&changed_manifest, &signature, &[trusted_key(&key)]),
            Err(VerifyError::UntrustedSignature)
        );
        let mut changed_signature = signature;
        changed_signature[0] ^= 1;
        assert_eq!(
            verify_signed_manifest(&bytes, &changed_signature, &[trusted_key(&key)]),
            Err(VerifyError::UntrustedSignature)
        );
    }

    #[test]
    fn verifies_signature_before_parsing_and_requires_canonical_json() {
        let key = signing_key();
        let (bytes, _) = sign(&valid_manifest(), &key);
        let malformed = b"{not json}";
        let malformed_signature = key.sign(malformed).to_bytes();
        assert_eq!(
            verify_signed_manifest(malformed, &malformed_signature, &[trusted_key(&key)]),
            Err(VerifyError::InvalidManifest)
        );

        let spaced = [bytes.as_slice(), b" "].concat();
        let spaced_signature = key.sign(&spaced).to_bytes();
        assert_eq!(
            verify_signed_manifest(&spaced, &spaced_signature, &[trusted_key(&key)]),
            Err(VerifyError::NonCanonicalManifest)
        );
    }

    #[test]
    fn rejects_untrusted_keys_and_mismatched_key_ids() {
        let key = signing_key();
        let (bytes, signature) = sign(&valid_manifest(), &key);
        assert_eq!(
            verify_signed_manifest(&bytes, &signature, &[]),
            Err(VerifyError::UntrustedSignature)
        );
        let mismatch = TrustedSigningKey {
            key_id: "different-id",
            public_key: key.verifying_key().to_bytes(),
        };
        assert_eq!(
            verify_signed_manifest(&bytes, &signature, &[mismatch]),
            Err(VerifyError::KeyIdMismatch)
        );
    }

    #[test]
    fn rejects_oversized_manifest_malformed_signature_and_oversized_trust_store() {
        let key = signing_key();
        let (bytes, signature) = sign(&valid_manifest(), &key);
        assert_eq!(
            verify_signed_manifest(&vec![b'x'; MAX_MANIFEST_BYTES + 1], &signature, &[]),
            Err(VerifyError::ManifestTooLarge)
        );
        assert_eq!(
            verify_signed_manifest(&bytes, &[0; 63], &[trusted_key(&key)]),
            Err(VerifyError::SignatureLength)
        );
        let oversized = vec![trusted_key(&key); MAX_TRUSTED_KEYS + 1];
        assert_eq!(
            verify_signed_manifest(&bytes, &signature, &oversized),
            Err(VerifyError::TrustStoreTooLarge)
        );
    }

    #[test]
    fn rejects_wrong_product_target_version_channel_and_artifact_name() {
        let key = signing_key();
        let cases = [
            ("product", VerifyError::WrongProduct),
            ("target", VerifyError::WrongTarget),
            ("version", VerifyError::InvalidVersion),
            ("channel", VerifyError::InvalidChannel),
            ("artifact", VerifyError::InvalidArtifactName),
        ];
        for (field, expected) in cases {
            let mut manifest = valid_manifest();
            match field {
                "product" => manifest.product_id = "com.buttonscli.tauri".into(),
                "target" => manifest.target_arch = "aarch64".into(),
                "version" => manifest.version = "not-semver".into(),
                "channel" => manifest.channel = "nightly".into(),
                "artifact" => manifest.artifact_basename = "../buttonscli.zip".into(),
                _ => unreachable!(),
            }
            let (bytes, signature) = sign(&manifest, &key);
            let verified =
                verify_signed_manifest(&bytes, &signature, &[trusted_key(&key)]).unwrap();
            assert_eq!(
                verify_package_bytes(&verified, TARGET, "1.2.2", "1.0.0", ARTIFACT),
                Err(expected),
                "case {field}"
            );
        }

        let mut manifest = valid_manifest();
        manifest.version = format!("1.2.3+{}", "x".repeat(MAX_RELEASE_VERSION_BYTES));
        let (bytes, signature) = sign(&manifest, &key);
        let verified = verify_signed_manifest(&bytes, &signature, &[trusted_key(&key)]).unwrap();
        assert_eq!(
            verify_package_bytes(&verified, TARGET, "1.2.2", "1.0.0", ARTIFACT),
            Err(VerifyError::InvalidVersion)
        );
    }

    #[test]
    fn rejects_downgrade_old_updater_size_and_digest_mismatch() {
        let key = signing_key();
        let cases = [
            ("downgrade", VerifyError::VersionNotNewer),
            ("updater", VerifyError::UpdaterTooOld),
            ("size", VerifyError::ArtifactSizeMismatch),
            ("digest", VerifyError::DigestMismatch),
        ];
        for (field, expected) in cases {
            let mut manifest = valid_manifest();
            let installed = if field == "downgrade" {
                "1.2.3"
            } else {
                "1.2.2"
            };
            let updater = if field == "updater" { "0.9.0" } else { "1.0.0" };
            let artifact = if field == "digest" {
                b"deterministic native zip byteS".as_slice()
            } else {
                ARTIFACT
            };
            if field == "size" {
                manifest.artifact_size += 1;
            }
            if field == "digest" {
                manifest.sha256 = format!("{:x}", Sha256::digest(ARTIFACT));
            }
            let (bytes, signature) = sign(&manifest, &key);
            let verified =
                verify_signed_manifest(&bytes, &signature, &[trusted_key(&key)]).unwrap();
            assert_eq!(
                verify_package_bytes(&verified, TARGET, installed, updater, artifact),
                Err(expected),
                "case {field}"
            );
        }
    }

    #[test]
    fn rejects_unsupported_schema_and_artifacts_over_the_size_bound() {
        let key = signing_key();
        for (change_schema, expected) in [
            (true, VerifyError::UnsupportedSchema),
            (false, VerifyError::ArtifactTooLarge),
        ] {
            let mut manifest = valid_manifest();
            if change_schema {
                manifest.schema_version += 1;
            } else {
                manifest.artifact_size = MAX_ARTIFACT_BYTES + 1;
            }
            let (bytes, signature) = sign(&manifest, &key);
            let verified =
                verify_signed_manifest(&bytes, &signature, &[trusted_key(&key)]).unwrap();
            assert_eq!(
                verify_package_bytes(&verified, TARGET, "1.2.2", "1.0.0", ARTIFACT),
                Err(expected)
            );
        }

        let manifest = valid_manifest();
        let (bytes, signature) = sign(&manifest, &key);
        let verified = verify_signed_manifest(&bytes, &signature, &[trusted_key(&key)]).unwrap();
        assert_eq!(
            verify_package_bytes(&verified, TARGET, "1.2.2", "1.0.0", &[]),
            Err(VerifyError::EmptyArtifact)
        );
    }

    #[test]
    fn rejects_archive_traversal_and_windows_special_paths() {
        for path in [
            "../escape.txt",
            "safe/../../escape.txt",
            "/absolute.txt",
            "C:/drive.txt",
            "\\\\server\\share\\file.txt",
            "safe\\..\\escape.txt",
            "CON",
            "folder/NUL.txt",
            "CONIN$",
            "folder/COM¹.txt",
            "trailing.",
            "trailing ",
            "folder/unsafe?.txt",
            "folder/unsafe|name.txt",
            "folder/control\u{001f}.txt",
        ] {
            assert_eq!(
                validate_archive_entry(path),
                Err(VerifyError::InvalidArchivePath),
                "path {path:?}"
            );
        }
        for path in ["buttonscli.exe", "assets/", "assets/icon.png"] {
            assert_eq!(validate_archive_entry(path), Ok(()), "path {path:?}");
        }
        let too_long_component = format!("{}.txt", "x".repeat(256));
        assert_eq!(
            validate_archive_entry(&too_long_component),
            Err(VerifyError::InvalidArchivePath)
        );
        assert_eq!(
            validate_archive_entry(&"x".repeat(MAX_ARCHIVE_PATH_BYTES + 1)),
            Err(VerifyError::InvalidArchivePath)
        );
    }

    #[test]
    fn signed_package_preflights_real_zip_entries_before_accepting() {
        let key = signing_key();
        let valid = test_archive(&[
            ("buttonscli.exe", b"test executable"),
            ("assets/icon.png", b"icon"),
        ]);
        let (manifest_bytes, signature) = sign(&manifest_for_artifact(&valid), &key);
        assert!(verify_signed_package(
            &manifest_bytes,
            &signature,
            &[trusted_key(&key)],
            TARGET,
            "1.2.2",
            "1.0.0",
            &valid,
        )
        .is_ok());

        let traversal = test_archive(&[("../escape.txt", b"outside")]);
        let (manifest_bytes, signature) = sign(&manifest_for_artifact(&traversal), &key);
        assert_eq!(
            verify_signed_package(
                &manifest_bytes,
                &signature,
                &[trusted_key(&key)],
                TARGET,
                "1.2.2",
                "1.0.0",
                &traversal,
            ),
            Err(VerifyError::InvalidArchivePath)
        );

        let missing_executable = test_archive(&[("assets/readme.txt", b"not an app")]);
        let (manifest_bytes, signature) = sign(&manifest_for_artifact(&missing_executable), &key);
        assert_eq!(
            verify_signed_package(
                &manifest_bytes,
                &signature,
                &[trusted_key(&key)],
                TARGET,
                "1.2.2",
                "1.0.0",
                &missing_executable,
            ),
            Err(VerifyError::MissingPackageExecutable)
        );
    }

    #[test]
    fn archive_preflight_rejects_corrupt_symlink_and_case_colliding_entries() {
        assert_eq!(
            validate_package_archive(b"not a zip"),
            Err(VerifyError::InvalidArchive)
        );

        let mut bad_crc = test_archive(&[("buttonscli.exe", b"stored payload")]);
        let file_name_size = u16::from_le_bytes([bad_crc[26], bad_crc[27]]) as usize;
        let extra_size = u16::from_le_bytes([bad_crc[28], bad_crc[29]]) as usize;
        let payload_start = 30 + file_name_size + extra_size;
        bad_crc[payload_start] ^= 0x01;
        assert_eq!(
            validate_package_archive(&bad_crc),
            Err(VerifyError::InvalidArchive)
        );

        let cursor = Cursor::new(Vec::new());
        let mut symlink = zip::ZipWriter::new(cursor);
        symlink
            .add_symlink(
                "linked-file",
                "../outside.txt",
                SimpleFileOptions::default(),
            )
            .unwrap();
        let symlink = symlink.finish().unwrap().into_inner();
        assert_eq!(
            validate_package_archive(&symlink),
            Err(VerifyError::InvalidArchivePath)
        );

        let colliding = test_archive(&[
            ("bin/buttonscli.exe", b"one"),
            ("BIN/BUTTONSCLI.EXE", b"two"),
        ]);
        assert_eq!(
            validate_package_archive(&colliding),
            Err(VerifyError::InvalidArchivePath)
        );
        let parent_collision = test_archive(&[("assets", b"file"), ("assets/icon.png", b"child")]);
        assert_eq!(
            validate_package_archive(&parent_collision),
            Err(VerifyError::InvalidArchivePath)
        );

        let mut oversized_entry = test_archive(&[("buttonscli.exe", b"x")]);
        let central_header = oversized_entry
            .windows(4)
            .position(|window| window == b"PK\x01\x02")
            .unwrap();
        let uncompressed_size_offset = central_header + 24;
        oversized_entry[uncompressed_size_offset..uncompressed_size_offset + 4]
            .copy_from_slice(&((MAX_ARCHIVE_FILE_BYTES + 1) as u32).to_le_bytes());
        assert_eq!(
            validate_package_archive(&oversized_entry),
            Err(VerifyError::ArchiveFileTooLarge)
        );

        let mut oversized_total = test_archive(&[
            ("one.bin", b"x"),
            ("two.bin", b"x"),
            ("three.bin", b"x"),
            ("four.bin", b"x"),
            ("five.bin", b"x"),
        ]);
        let central_headers = oversized_total
            .windows(4)
            .enumerate()
            .filter_map(|(offset, window)| (window == b"PK\x01\x02").then_some(offset))
            .collect::<Vec<_>>();
        assert_eq!(central_headers.len(), 5);
        for central_header in central_headers {
            let offset = central_header + 24;
            oversized_total[offset..offset + 4]
                .copy_from_slice(&(MAX_ARCHIVE_FILE_BYTES as u32).to_le_bytes());
        }
        assert_eq!(
            validate_package_archive(&oversized_total),
            Err(VerifyError::ArchiveTooLarge)
        );
    }

    #[test]
    fn stages_verified_archive_without_switching_active_version_or_overwriting() {
        let root = temporary_stage_root();
        fs::write(root.join("active-version"), "1.2.2").unwrap();
        let artifact = test_archive(&[
            ("buttonscli.exe", b"new build"),
            ("assets/help.md", b"help"),
        ]);
        let key = signing_key();
        let manifest = manifest_for_artifact(&artifact);
        let (manifest_bytes, signature) = sign(&manifest, &key);

        let staged = verify_and_stage_package(
            &manifest_bytes,
            &signature,
            &[trusted_key(&key)],
            TARGET,
            "1.2.2",
            "1.0.0",
            &artifact,
            &root,
        )
        .unwrap();
        assert_eq!(
            staged,
            fs::canonicalize(root.join("versions"))
                .unwrap()
                .join("1.2.3")
        );
        assert_eq!(
            fs::read(staged.join("buttonscli.exe")).unwrap(),
            b"new build"
        );
        assert_eq!(fs::read(staged.join("assets/help.md")).unwrap(), b"help");
        assert_eq!(
            fs::read_to_string(root.join("active-version")).unwrap(),
            "1.2.2"
        );
        assert_eq!(
            verify_and_stage_package(
                &manifest_bytes,
                &signature,
                &[trusted_key(&key)],
                TARGET,
                "1.2.2",
                "1.0.0",
                &artifact,
                &root,
            ),
            Err(VerifyError::VersionAlreadyStaged)
        );
        assert_eq!(
            fs::read(staged.join("buttonscli.exe")).unwrap(),
            b"new build"
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_package_cannot_create_a_staging_version() {
        let root = temporary_stage_root();
        fs::write(root.join("active-version"), "1.2.2").unwrap();
        let artifact = test_archive(&[("../escape.txt", b"outside")]);
        let key = signing_key();
        let (manifest_bytes, signature) = sign(&manifest_for_artifact(&artifact), &key);

        assert_eq!(
            verify_and_stage_package(
                &manifest_bytes,
                &signature,
                &[trusted_key(&key)],
                TARGET,
                "1.2.2",
                "1.0.0",
                &artifact,
                &root,
            ),
            Err(VerifyError::InvalidArchivePath)
        );
        assert_eq!(
            fs::read_to_string(root.join("active-version")).unwrap(),
            "1.2.2"
        );
        assert!(!root.join("versions").exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn release_builder_creates_verifiable_package_without_overwriting() {
        let root = temporary_stage_root();
        let source = root.join("input");
        fs::create_dir(&source).unwrap();
        fs::create_dir(source.join("assets")).unwrap();
        fs::write(source.join("buttonscli.exe"), b"test executable").unwrap();
        fs::write(source.join("assets/help.md"), b"help").unwrap();
        let key_path = root.join("throwaway-signing-key.bin");
        fs::write(&key_path, [0x5a; 32]).unwrap();
        let output = root.join("release");

        let built = build_signed_release_package(ReleaseBuildRequest {
            source_directory: &source,
            output_directory: &output,
            signing_key_file: &key_path,
            version: "1.2.3",
            target: TARGET,
            minimum_updater_version: "1.0.0",
            channel: "stable",
            key_id: TEST_KEY_ID,
        })
        .unwrap();

        assert_eq!(built.public_key, signing_key().verifying_key().to_bytes());
        let artifact = fs::read(&built.artifact_path).unwrap();
        let manifest = fs::read(&built.manifest_path).unwrap();
        let signature = fs::read(&built.signature_path).unwrap();
        let trusted_key = TrustedSigningKey {
            key_id: TEST_KEY_ID,
            public_key: built.public_key,
        };
        assert_eq!(
            verify_signed_package(
                &manifest,
                &signature,
                &[trusted_key],
                TARGET,
                "1.2.2",
                "1.0.0",
                &artifact,
            )
            .unwrap()
            .version,
            "1.2.3"
        );
        assert_eq!(
            build_signed_release_package(ReleaseBuildRequest {
                source_directory: &source,
                output_directory: &output,
                signing_key_file: &key_path,
                version: "1.2.3",
                target: TARGET,
                minimum_updater_version: "1.0.0",
                channel: "stable",
                key_id: TEST_KEY_ID,
            }),
            Err(ReleaseBuildError::OutputAlreadyExists)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn release_builder_rejects_missing_windows_executable_and_embedded_key() {
        let root = temporary_stage_root();
        let source = root.join("input");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("readme.txt"), b"not an app").unwrap();
        let key_path = root.join("throwaway-signing-key.bin");
        fs::write(&key_path, [0x5a; 32]).unwrap();
        let output = root.join("release");

        assert_eq!(
            build_signed_release_package(ReleaseBuildRequest {
                source_directory: &source,
                output_directory: &output,
                signing_key_file: &key_path,
                version: "1.2.3",
                target: TARGET,
                minimum_updater_version: "1.0.0",
                channel: "stable",
                key_id: TEST_KEY_ID,
            }),
            Err(ReleaseBuildError::MissingPackageExecutable)
        );
        assert!(!output.exists());

        let embedded_key = source.join("signing-key.bin");
        fs::write(&embedded_key, [0x5a; 32]).unwrap();
        assert_eq!(
            build_signed_release_package(ReleaseBuildRequest {
                source_directory: &source,
                output_directory: &output,
                signing_key_file: &embedded_key,
                version: "1.2.3",
                target: TARGET,
                minimum_updater_version: "1.0.0",
                channel: "stable",
                key_id: TEST_KEY_ID,
            }),
            Err(ReleaseBuildError::SigningKeyInsideSource)
        );
        assert!(!output.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
