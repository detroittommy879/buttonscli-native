# Signed developer notices

The native app can display two optional, authenticated developer messages: a short
status-bar label and a small closeable window. A third `reserved` slot accepts JSON
for future use but does nothing today. No HTML, images, scripts, terminal input,
downloads or automatic updates are supported. The existing GitHub update check is
independent; this feature does not sign release binaries.

Users opt in during first-run setup or Settings → Workspace → Developer notices.
Security notices and product news share that switch; promotions require a second,
separate opt-in. All are off initially, including for existing profiles. Disabling
the channel clears its UI and stops future checks. `BUTTONSCLI_NO_ANNOUNCEMENTS=1`
also disables it. An already-running HTTP request may finish after switching off;
its result is discarded.

## Files and trust

- App endpoint: `https://buttonscli.com/app-notices/messages.json`.
- AMP directory: `/var/www/buttonscli.com/html/app-notices/`.
- `messages.json.txt` is a staged signed envelope; **only** `messages.json` is fetched.
- `payload.example.json.txt` is the readable, unsigned sample; edit a local copy.
- Embedded public keys: `assets/announcement-keys.json`; rebuild after changing it.
- Maintainer-only tool: `announcement-tool` (feature `release-tools`). It is not an
  app runtime dependency and needs no GPG installation.

The envelope holds `key_id`, a UTF-8 `payload` string, and a base64 Ed25519 signature.
The signature covers `ButtonsCLI announcements v1\0` followed by the exact payload
bytes. Reformatting the payload after signing invalidates it. Envelope whitespace
can change. Both trusted keys can sign; keys are never learned from the server.
Strict verification uses the already-pinned `ed25519-dalek` 2.2.0 dependency.

## Author and stage (PowerShell, from the checkout)

The first command builds the offline signing utility. Output files must be new:
the tool refuses to overwrite them. Use a fresh filename/directory each time.

1. `cargo build --locked --features release-tools --bin announcement-tool`
2. `./target/debug/announcement-tool.exe draft 2 ./notice-payload.json`
3. Edit `notice-payload.json`. Use a sequence greater than **every feed ever
   published**, including recovery-key feeds. Give each new message a new ID;
   keep its ID when preserving a user's dismissal. Use `null` to empty a slot.
4. `./target/debug/announcement-tool.exe sign "$env:USERPROFILE/.buttonscli-signing/announcements-2026-11-11/primary-2026.key" primary-2026 ./notice-payload.json ./notice-signed.json.txt`
5. `./target/debug/announcement-tool.exe verify ./notice-signed.json.txt`
6. `scp ./notice-signed.json.txt 193.122.157.24:/var/www/buttonscli.com/html/app-notices/messages.json.txt`

Keep drafts outside Git (or under `.private/`). The two relative filenames above
are examples for a single publishing session; remove/move those public drafts
afterwards. Never upload any `.key` file, native profile, or provider settings.
The sample staged during initial setup uses sequence 1 and expires after seven
days. Generate a fresh draft before publishing if it has expired.

## Activate and stop

Activate only after checking the staged content/signature:

`ssh 193.122.157.24 'mv -f /var/www/buttonscli.com/html/app-notices/messages.json.txt /var/www/buttonscli.com/html/app-notices/messages.json'`

This is one rename on the same filesystem; readers see the old or new signed
envelope, never mismatched payload/signature files. Verify the public download with
the tool too. HTTPS, no redirects, a 15-second total timeout, and a 64 KiB read cap
apply. The directory's `.htaccess` disables caching/index listing; no server reload
is needed. Offline, 404, malformed or invalid answers silently display nothing new.

Clients check after opt-in at startup, then every six hours while open. They do
not cache messages across restarts, but keep replay/dismissal state per profile.
To withdraw messages from running clients, publish a higher-sequence feed with
`status`, `popup`, and `reserved` all `null`. Removing/renaming the active file
stops future delivery but cannot recall an already-displayed message; it remains
until dismissal, restart or expiry. There is no push transport.

## Payload fields and limits

```json
{
  "schema": 1,
  "product": "com.buttonscli.native.notices",
  "sequence": 2,
  "issued_at": 1794430800,
  "expires_at": 1794517200,
  "slots": {
    "status": {
      "id": "update-status-2",
      "category": "security",
      "title": "A security update is available",
      "url": "https://github.com/detroittommy879/buttonscli-native/releases/latest"
    },
    "popup": null,
    "reserved": null
  }
}
```

Use `draft` for current Unix-second timestamps; the dates above illustrate the
shape only. `issued_at` permits five minutes of future clock skew. Expiry must be
after issuance and now, with at most 30 days' lifetime. Invalid feeds are rejected
as a whole. There are no mandatory/noncloseable messages.

| Field | Limit / behavior |
| --- | --- |
| Signed envelope / payload | 64 KiB / 24 KiB UTF-8 bytes; JSON nesting limit applies |
| Message ID | 1–64 ASCII letters, digits, `.`, `_`, `-`; unique per slot |
| Status title | 1–160 Unicode characters; visually truncated with full hover text |
| Status body | Empty; omit it |
| Popup title / body | 1–96 / 0–4096 Unicode characters; wrapped, body scrolls |
| Category | `security`, `news`, `promotion` |
| Link | Optional HTTPS URL, ≤2048 bytes, no credentials; explicit click only |
| Targeting | Optional `min_version` / `max_version` (inclusive SemVer), `platforms` array of `windows`, `linux`, `macos`; omitted/empty = all |
| Text | Plain text; reject control characters and bidi overrides/isolates; body allows newline/tab |
| Reserved slot | Arbitrary signed JSON within payload/nesting limits, never rendered |

The profile keeps the highest sequence and exact payload hash: lower sequences,
or different content reusing a sequence, are rejected. Identical re-fetches are
allowed. Dismissals retain the last 128 IDs; old IDs can reappear after eviction.
Revert and Close restores notice switches but preserves replay/dismissal history;
switching profiles clears pending deliveries from the previous profile.
Resetting/deleting profile state also resets replay/dismissal history. A fresh
installation can accept an old but still-valid signed feed. A wrong system clock
can suppress notices. A compromised host can withhold messages or replay a still
valid feed within these constraints; it cannot forge them without a private key.

## Private keys and recovery

Initial private keys are in the local, ACL-restricted directory
`%USERPROFILE%\.buttonscli-signing\announcements-2026-11-11\`:
`primary-2026.key` and `recovery-2026.key` (32 raw bytes each).
They are unencrypted files, protected by the current Windows account and SYSTEM
ACL, not hardware-backed keys. **Back up both offline; move the recovery key
offline after checking the backup.** Do not store either on AMP or in Git.

For a new identity, first create a private directory with a restrictive Windows
ACL (Unix: mode 0700), then run
`announcement-tool keygen <private-directory> <new-public-keys.json>`.
Install/review the public map in `assets/announcement-keys.json`, then rebuild.
Unix key files are created mode 0600; Windows files inherit the directory ACL.

If the primary key is lost, sign a higher-sequence feed with the recovery key ID.
If a key is compromised, publish a new app release removing that public key.
The backup key does **not** revoke a compromised primary key in old binaries;
there is deliberately no remotely writable trust store. Keep the main private
key offline between publishing sessions when practical.

## Validation

`cargo test --locked --lib announcements` checks signatures, cross-protocol
rejection, tampering, replay, expiry, text/URL limits, opt-in, filtering and popup
dismissal. The optional Windows `native_signed_announcements_probe` captures
wide/narrow layouts without the normal profile or network; set
`BUTTONSCLI_ANNOUNCEMENT_CAPTURE_DIR` to an owned output directory and run with
`--ignored --test-threads=1`. This does not certify Linux/macOS interaction.
