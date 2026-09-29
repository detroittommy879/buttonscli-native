# Quick Secrets security design

Quick Secrets is an internal native feature (`quickSecrets`). It stays hidden
in release builds and is available only in development builds when
`BUTTONSCLI_NATIVE_DEV_QUICK_SECRETS=1` is set. The development override is
resolved through the central feature catalog and has no release-build effect.

## Storage and cryptography

- Store one versioned encrypted vault in the active native profile. Never put
  vault material in preferences, imports, logs, crash messages, or telemetry.
- Do not import the legacy browser vault. The native format is independent and
  begins empty. Legacy vault data remains untouched.
- Derive a 256-bit key from the exact passphrase bytes and a random 16-byte salt
  with Argon2id v19. Use fixed, versioned parameters (64 MiB, three passes, one
  lane) so untrusted file data cannot request unbounded KDF work.
- Encrypt each complete vault snapshot with AES-256-GCM, a fresh random
  12-byte nonce, and associated data that binds the format version and native
  profile. Reject unknown versions, invalid lengths, and vault files larger
  than 1 MiB before deriving a key.
- Write only ciphertext through the native store's same-directory atomic
  replacement path. Keep a fake in-memory repository for tests.
- Keep the derived key, decrypted entries, passphrase drafts, and plaintext
  serialization in zeroizing containers where supported. Redact `Debug` and
  never log their values.

## Unlock, lock, and recovery

Require a passphrase with at least 12 Unicode scalar values; preserve it
exactly, including leading and trailing spaces. Verify the confirmation before
creating the vault. Unlock failure must leave the stored bytes unchanged.
Lock clears the in-memory key and entry list. A configurable idle timeout
defaults to 15 minutes; zero means manual lock. App shutdown and profile change
also lock the vault.

There is no password reset or recovery key. A lost passphrase means the
ciphertext cannot be recovered. The internal UI offers an explicit forget and
recreate path after a separate confirmation; that deletes all stored entries.
Corrupt or unsupported files are reported without replacement.

## Terminal delivery

Require an explicit stable terminal ID selection for every delivery. Show its
current title and reject a closed, unready, exited, or changed target. A secret
is sent only after the user clicks a clearly labeled Paste or Paste + Enter
action. Never use the current focus as an implicit fallback. Quick Secrets has
no auto-import, presets integration, clipboard access, or background watcher.

Sensitive paste bypasses the native last-input observer, but it cannot prevent
a shell or child program from echoing the text into PTY output. AI Help context
is opt-in and may include such echoed output; the UI warns about this before
any paste.

This is a short-snippet feature, not a password manager. A secret necessarily
appears in process memory while unlocked and in the selected terminal after
paste. The operating-system account and device protections remain part of the
security boundary.
