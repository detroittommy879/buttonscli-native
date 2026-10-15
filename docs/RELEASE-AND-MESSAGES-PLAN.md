# Plan: versions, updates, Microsoft Store, macOS builds and remote messages

Status: proposal, October 15 2026. Nothing here is implemented yet except what
the "Already in the repo" notes say.

## The short version

1. **Pick a version scheme now** (step 1). Everything else depends on it.
2. **Windows Store first, as MSIX.** The Store signs MSIX packages for you and
   delivers updates itself, so you get free signing and free auto-updates. That
   is cheaper and simpler than any self-updater.
3. **Keep a self-updater only for the direct download** (zip/installer from
   buttonscli.com and GitHub Releases). The signed-manifest code for it already
   exists in `src/distribution.rs`; it needs a real key and a UI.
4. **Remote messages: a small signed JSON file**, checked at most once a day,
   verified with an Ed25519 public key compiled into the app. Same key handling
   as the updater, separate key.
5. **macOS: build on GitHub Actions** (or your MacBook as a self-hosted runner),
   ship unsigned at first, and add Apple notarization ($99/year) when Mac users
   matter.

## 1. Version numbers

- One source of truth: `version` in `Cargo.toml` (currently `0.1.0`, never bumped).
- Use semantic versions: `0.2.0` for the first public preview. Bump the minor
  for features, the patch for fixes. Go to `1.0.0` when you would recommend it
  to a stranger.
- Tag every release in git: `v0.2.0`. CI builds release assets from tags only.
- MSIX needs four numbers, `major.minor.patch.0`. The Store requires the last
  number to be 0, so map `0.2.3` to `0.2.3.0`.
- Show the version in Help > About and in `--version` (already wired to
  `CARGO_PKG_VERSION`). Add the short git commit for builds that are not tagged.
- Keep `CHANGELOG.md` per version; the Store's "What's new" text and the
  updater's release notes come from it.

## 2. Microsoft Store

What changed recently: individual developer registration is free since
September 2025 (ID scan plus selfie, no credit card) at
storedeveloper.microsoft.com.

Two ways in:

| | MSIX package | EXE/MSI installer |
|---|---|---|
| Code signing | Microsoft re-signs it after certification; no certificate to buy | You must sign with a certificate from a CA in the Microsoft Trusted Root Program (costs money; Azure Trusted Signing is the cheapest option) |
| Hosting | Store hosts it | You host it at a versioned URL that never changes |
| Updates | Store updates the app automatically | Your app must update itself |
| Install | Clean per-user install, clean uninstall | Must install silently, no web stub |

**Recommendation: MSIX.** Steps:

1. Register the free developer account and reserve the name "ButtonsCLI".
2. Add an `AppxManifest.xml` template under `packaging/msix/` with the reserved
   identity, a `runFullTrust` capability (a desktop app with ConPTY needs it),
   logos in the required sizes and the 4-part version.
3. Package in CI on `windows-latest` with `makeappx.exe` from the Windows SDK
   (or `cargo-packager`/`winapp`, which wrap it). Upload the `.msix` as a
   release artifact; you then upload it to Partner Center by hand at first.
4. Test locally: sign the test package with a throwaway self-signed cert and
   install it with `Add-AppxPackage`, then check that shells launch, ConPTY
   works, settings land in `%USERPROFILE%\.buttonscli-native`, and the control
   descriptor/helper files are still writable from inside the package.
5. Certification checklist items likely to come up: privacy policy URL (needed
   because of AI providers and the account feature), age rating questionnaire,
   screenshots (you have good ones), and a note that AI features need the user's
   own API keys.
6. **Turn the self-updater off in Store builds** (a `store` cargo feature or a
   check for package identity). As far as I know, Store policy does not allow a
   packaged app to update itself outside the Store. Verify that against the
   current Store Policies before submitting.

Things to double check before relying on them: the current Store Policies on
self-updating and in-app promotional content (section on advertising), and
whether a WSL/ConPTY desktop app needs any restricted capability beyond
`runFullTrust`.

## 3. Updates for the direct download

**Already in the repo:** `src/distribution.rs` verifies an Ed25519-signed
release manifest (product id, version, target, SHA-256 of the package), checks
the package, and stages it. Its trusted-key list is empty on purpose.

Remaining work:

1. Generate the release signing key offline (`ed25519` 32-byte seed). Keep two
   encrypted backups outside Git. Compile only the public key into the app.
2. CI release job (on tag): build, zip, write `manifest.json`, sign it in a
   protected environment, upload `manifest.json`, `manifest.sig` and the zip to
   GitHub Releases.
3. App: check `https://github.com/<you>/buttonscli-native/releases/latest/download/manifest.json`
   at most once a day (setting: "Check for updates", default on for the direct
   build, absent in the Store build). Show "Version 0.3.0 is ready, restart to
   update". Never update silently while terminals are running.
4. Apply on restart: a tiny updater step swaps the folder and relaunches;
   keep the previous version for one-click rollback.
5. Rules that prevent the known updater attacks: the version is inside the
   signed manifest (a Tauri updater CVE this year came from an unsigned version
   field allowing downgrades), never install a lower version than the running
   one, and reject manifests for another product id or platform.

## 4. Remote messages (notices, status-bar text)

What most apps do: fetch a small JSON document over HTTPS on a schedule, and
trust it only if a signature made with a key compiled into the app checks out.
HTTPS alone protects the transport but not a hacked web server, which is why
the signature matters. Same pattern as the update manifest.

Proposed format (`messages.json` plus `messages.json.sig`, detached Ed25519):

```json
{
  "schema": 1,
  "product": "com.buttonscli.native",
  "serial": 42,
  "issuedAt": "2026-10-15T00:00:00Z",
  "expiresAt": "2026-11-15T00:00:00Z",
  "messages": [
    {
      "id": "v030-released",
      "kind": "status",
      "text": "## New: VHS and CRT shaders\nTry **Themes > Shaders**.",
      "link": "https://buttonscli.com/news/0-3-0",
      "minVersion": "0.2.0",
      "maxVersion": null,
      "platforms": ["windows", "macos", "linux"],
      "showFrom": "2026-10-15T00:00:00Z",
      "showUntil": "2026-10-30T00:00:00Z",
      "priority": 1
    }
  ]
}
```

Rules in the app:

- Verify the signature over the exact bytes before parsing. Check `product`,
  `expiresAt`, and that `serial` is not lower than the last one accepted
  (stops an attacker replaying an old file).
- Render a tiny safe subset: headings, paragraphs, bold/italic, lists. No HTML,
  no images, no scripts. Use the active theme's UI colors and fonts.
- Links: only `https://` on domains you list in the app (buttonscli.com,
  wuu73.org). Always shown as text the user clicks; never opened automatically.
- `kind`: `status` (one line in the status bar, rotates rarely), `notice` (a
  dismissible card once), `critical` (security notice; still dismissible).
- Remember dismissed ids locally. Cap how often anything shows (for example one
  new item per day).
- Fetch at most daily, with a short timeout, after startup has finished; never
  block the UI. Send no identifiers: a plain GET with no query string. Cache
  the last good file for offline use.
- Settings > Privacy: "Show news and tips from ButtonsCLI" (on by default in
  the direct build). If you ever show third-party ads, label them and check the
  Store's advertising policy first.
- Use a separate signing key from the updater key, so a leaked message key
  cannot ship code.

Host it as a static file (GitHub Pages, your server, or a Cloudflare R2 bucket).
Sign it on your machine with a small script: `cargo run --bin sign-message
messages.json` (to be written, reusing the Ed25519 code).

## 5. macOS builds

- GitHub-hosted macOS runners are free for public repositories. For private
  repositories, macOS minutes count 10x against the monthly free allowance
  (2,000 minutes on the Free plan), so each build costs a lot of quota.
- Cheapest option for a private repo: register your MacBook as a
  **self-hosted runner** (free, no minute limits) and let the tag workflow use
  it for macOS jobs.
- Build `aarch64-apple-darwin` and `x86_64-apple-darwin`, merge with `lipo`
  into a universal binary, wrap in a `.app` bundle (`cargo-bundle` or
  `cargo-packager`), and ship a `.dmg` or zip.
- Without the Apple Developer Program ($99/year) the app cannot be notarized,
  and Gatekeeper warns on first open (right-click > Open). Fine for early
  testers; get notarization before a public Mac launch.
- The repo has no Mac interaction testing yet (see `LIMITATIONS.md`); expect a
  round of fixes for keyboard shortcuts (Cmd), fonts and the PTY.

## 6. Order of work

1. Version bump to `0.2.0`, CHANGELOG entry, tag-driven CI release job that
   uploads the Windows zip to GitHub Releases.
2. MSIX template and CI packaging; local install test; free Store account and
   name reservation; first submission.
3. Release key, signed manifest in CI, update check and restart-to-update UI
   for the direct build (Store build excluded).
4. Messages: signer script, fetcher, renderer, status-bar slot, settings toggle.
5. macOS self-hosted runner, universal build, unsigned test release.

Sources used for the Store and signing details:
[MSI/EXE package requirements](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msi/app-package-requirements),
[Distribute a Win32 app through the Store](https://learn.microsoft.com/en-us/windows/apps/distribute-through-store/how-to-distribute-your-win32-app-through-microsoft-store),
[Free registration for individual developers](https://blogs.windows.com/windowsdeveloper/2025/09/10/free-developer-registration-for-individual-developers-on-microsoft-store/),
[GitHub Actions billing](https://docs.github.com/en/enterprise-cloud@latest/billing/concepts/product-billing/github-actions),
[Tauri updater downgrade CVE](https://www.sentinelone.com/vulnerability-database/cve-2026-95625/).
