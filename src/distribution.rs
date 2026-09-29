//! Fail-closed verification primitives for the separate native release family.
//!
//! The production trust list is intentionally empty until its public key has
//! been supplied and backed up outside Git. Tests inject deterministic throwaway
//! keys; no release endpoint or installer calls these functions yet.

use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const PRODUCT_ID: &str = "com.buttonscli.native";
const MANIFEST_SCHEMA_VERSION: u32 = 1;
const MAX_MANIFEST_BYTES: usize = 16 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_TRUSTED_KEYS: usize = 16;

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
    VersionNotNewer,
    InvalidChannel,
    InvalidArtifactName,
    ArtifactTooLarge,
    EmptyArtifact,
    ArtifactSizeMismatch,
    DigestMismatch,
    UpdaterTooOld,
    InvalidArchivePath,
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "native release verification failed: {self:?}")
    }
}

impl std::error::Error for VerifyError {}

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

/// Verify the detached signature and package metadata as one ordered operation.
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
    Ok(manifest)
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
            || is_windows_reserved_name(component)
        {
            return Err(VerifyError::InvalidArchivePath);
        }
    }
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
    }
}
