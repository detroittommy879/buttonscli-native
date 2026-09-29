# Native distribution and update decision

Status: the strict verifier, bounded ZIP preflight, versioned staging core, and
isolated tests are implemented on Windows. No package builder, active-version
switch, rollback launcher, production key, or release endpoint exists. This is
still a source and architecture decision for P06, not a claim that ButtonsCLI
Native is distributable today.

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

The first workflow is manual and local: run the test/build gates, package into
an external native release staging directory, sign with an external key path,
verify the produced signature and artifact hash, and preserve previous
artifacts for rollback. Do not add automatic GitHub desktop builds or publish
to an existing ButtonsCLI/Tauri endpoint. Native update checks remain absent
until the user supplies a production public-key trust root and a separate
native release location.

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
throwaway keys exist only in tests. P06 remains open: there is no package
builder, active-version switch, rollback launcher, update UI, production trust
key, or native release endpoint. This avoids generating an unbacked release
key or routing native artifacts through the existing Tauri release system.
