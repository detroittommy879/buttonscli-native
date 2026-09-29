# Native distribution and update decision

Status: the strict verifier, bounded ZIP preflight, versioned staging core,
Windows local package builder, and isolated tests are implemented. No
active-version switch, rollback launcher, production key, update UI, or release
endpoint exists. This is not a claim that ButtonsCLI Native is distributable
today.

## Product identity and release boundary

- Native product ID: `com.buttonscli.native`.
- Native artifact family: `buttonscli-native`.
- Native settings root remains `~/.buttonscli-native/`, implemented in
  `src/storage/paths.rs`.
- Keep the Rust executable name `buttonscli` for now; use distinct product IDs
  and artifact filenames to avoid breaking development scripts and helpers.
- Store native release artifacts and signing material outside the original
  Tauri release cockpit. Never update the original website's `current`
  pointer, manifest, Tauri updater channel, or private signing keys.
- Use versioned native artifacts such as
  `buttonscli-native-windows-x86_64-<version>.zip`. No mutable `latest`
  pointer is required for a manual local release.

## Signed artifact envelope

Use an immutable package plus `release.json` and a detached `release.sig`.
`release.json` has a versioned schema and identifies the product, semantic
version, target OS/architecture, artifact basename, artifact byte length,
SHA-256 digest, minimum updater version, release channel, and signing key ID.
The signer emits deterministic compact JSON. The signature covers the exact
UTF-8 bytes of `release.json`; the verifier checks the signature before
parsing or trusting any field, then checks target, product, filename, size,
and digest before extraction.

Use Ed25519 with a compiled-in public-key allowlist and strict verification.
The native app must never fetch a replacement trust key from the same release
server. Key rotation needs an old-key-signed transition or a separately
distributed application update. Tests should use deterministic throwaway
keys; a production private key must be supplied and backed up outside Git
before any real artifact can be signed. The upstream Rust API provides
`VerifyingKey::verify_strict` for strict signature checking:
[ed25519-dalek 2.2.0 documentation](https://docs.rs/ed25519-dalek/2.2.0/ed25519_dalek/struct.VerifyingKey.html#method.verify_strict).

## Installation and rollback

Do not replace a running native executable. A platform adapter stages an
update in a versioned directory, verifies the signed manifest and package
before extraction, and changes the active-version record only after the app
has closed. Retain the previous known-good version. The launcher marks a new
version healthy only after startup completes; on launch failure it restores
the previous active-version record and starts that version. Interrupted
staging leaves the current version untouched. Keep user settings and profiles
outside version directories so rollback cannot roll back user data.

For the first Windows local artifact, prefer a versioned portable ZIP. Add a
bootstrapper only with the signed manifest verifier and rollback tests in
place. Linux package-manager integration and macOS signed/notarized bundles
need separate platform adapters and host validation; they do not share the
Tauri artifact pipeline.

## Local release workflow and gates

The first workflow is manual and local: run the test/build gates, prepare a
directory containing the Windows `buttonscli.exe` and its runtime files, then
run the opt-in builder. Its key file is exactly 32 raw Ed25519 seed bytes and
must be supplied and stored outside the source directory and Git. The tool
does not generate a key or overwrite an output directory; it emits the ZIP,
`release.json`, `release.sig`, and the derived public key for review. Verify the
signature and artifact hash, back up the key, and preserve previous artifacts
for rollback. Do not add automatic GitHub desktop builds or publish to an
existing ButtonsCLI/Tauri endpoint. Native update checks remain absent until a
production public-key trust root and separate native release location are
reviewed.

Example Windows command (use paths for a prepared package and an externally
managed key):

```powershell
cargo run --no-default-features --features release-tools --bin native-release -- package `
  --source C:\release\prepared `
  --output C:\release\1.2.3 `
  --key-file C:\keys\buttonscli-native-ed25519.seed `
  --version 1.2.3 --target-os windows --target-arch x86_64 `
  --minimum-updater-version 1.0.0 --channel stable --key-id native-2026
```

The example uses placeholders; it does not create or select a real key.

Required tests before any adapter can be called complete: valid signed package
accepted; one-bit manifest/signature/artifact tampering rejected; wrong
product/target/version rejected; archive path traversal rejected; failed or
interrupted staging leaves the active version intact; failed startup restores
the prior version; and Tauri paths, channels, manifests, and release pointers
remain unchanged.

## Current limits

The product/settings namespace is already distinct. The verifier checks strict
Ed25519 signatures, canonical manifest bytes, metadata, artifact size and
digest, and ZIP structure before extraction. Preflight bounds entries and
expanded bytes, rejects traversal, symlinks, special files, case collisions,
and malformed streams, and validates CRCs. A verified Windows archive must
contain a root-level `buttonscli.exe`. Version strings are bounded before they
become directory names, and staging roots reject symlinks and Windows reparse
points. A verified archive can be extracted to a new versioned directory
without changing active-version state; duplicate versions are not overwritten.
Its compiled allowlist is intentionally empty;
throwaway keys exist only in tests. The local builder packages a prepared
directory, validates it, and signs into a new output directory without
replacing existing artifacts. P06 remains open: there is no active-version
switch, rollback launcher, update UI, production trust key, or native release
endpoint. The builder does not generate an unbacked release key or route native
artifacts through the existing Tauri release system.
