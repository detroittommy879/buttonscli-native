# Compatibility contracts

See [plan](README.md) for source roots O and N. These decisions constrain implementation; proposed interfaces are not current APIs.

## 1. Separate native settings with optional import

### Canonical paths

Use injectable `NativeDataRoot` and `LegacyImportRoot`; production defaults to the OS-resolved user's home plus `.buttonscli-native` and `.buttonscli`, respectively. Only the native root is writable. Tests always use temporary directories; WASM uses its existing browser persistence and cannot touch these files. This implements the user's clarification, superseding the initially considered shared-writer design.

```text
~/.buttonscli/                        # original; read-only import source
  active-profile.json                 # original metadata { "name": "..." }
  config.json                         # legacy fallback
  profiles/<profile>/config.json
  profiles/<profile>/themes/*.json
  profiles/<profile>/shaders/
~/.buttonscli-native/                 # proposed native-owned root
  active-profile.json
  profiles/<profile>/config.json      # compatible data with sensitive fields excluded
  profiles/<profile>/native.json      # versioned native-only settings
  profiles/<profile>/themes/*.json
  profiles/<profile>/shaders/         # optional preserved data, not yet executable
  control/<instance-id>.json          # fresh native runtime credentials
  helpers/                           # pinned native CLI/MCP helpers
  imports/                           # manifests without secrets; native rollback metadata
```

Original path and sanitization behavior: O `src-tauri/src/main.rs::{get_app_data_dir,get_active_profile_path,sanitize_profile_name,get_config_path}`. Characterize it in fixtures before translating. Reject traversal, symlink/reparse escapes, invalid filenames, oversized files, and profile paths outside the resolved data root. Avoid silently choosing a different profile after a malformed metadata file; show recovery state without overwriting anything.

Normal startup reads the native root, then existing eframe preferences for a one-time native migration, then defaults. It may offer import on first run, but must not silently replace existing native data. Settings exposes Import from original ButtonsCLI, with a source picker and preview. Within the chosen legacy source, prefer active profile config; use root config only if that profile config is absent. Import active profile by default, with optional selected additional profiles. Missing fields can default; an intentionally empty preset list must stay empty. A corrupt profile is not equivalent to missing and must not trigger autosave of defaults.

Parse the original JSON in memory alongside typed projections. Do not reduce it through the smaller native `Preferences` struct. Preserve compatible unknown nested objects, ordering, and unsupported visual settings in the native copy, subject to the credential/runtime exclusions below. Never persist a raw whole-config backup containing keys as an incidental import artifact. Compatibility projection must not trim preset command whitespace merely because the current native editor's `normalized()` does.

### Field ownership

| Original field | Native mapping / rule |
|---|---|
| `presets[]`, `sshPresets[]` | `label`, `command`, optional `sendEnter`; characterize original absent-value default; explicit false must survive |
| `window.defaultShell`, `window.customShellProfiles[]` | Translate command-based original selection to native shell IDs; preserve unmapped profile fields; native working-directory extensions stay in sidecar |
| `theme.app`, `theme.terminal`, `theme.typography`, `effects` | Project actual saved values, not only a built-in theme ID; keep unsupported values round-trippable |
| `keyboard`, `layout`, `window.opacity` | Preserve until individual features are implemented; only patch owned fields |
| `assistant.namedProviders`, `activeProviderId` and deprecated provider forms | Import metadata and normalize using original migration behavior; keys follow credential policy below |
| `localization` | Respect mode/manual locale and prior confirmation; do not force onboarding again |
| `features`, `plugins`, future keys | Retain; local config is not proof of paid access |
| Native pane ratios / viewport positions / theme source IDs | Versioned `native.json`; never add incompatible meanings to legacy keys |

Theme library: load current profile files as well as embedded themes. Preserve raw v1 documents and metadata. Use source-qualified internal identities so a personal theme ID collision cannot silently replace an embedded entry. Keep selected profile and original metadata ID for export. Bad files generate per-file warnings and do not empty the library. Unsupported effects remain data, with a clear “not rendered” indication.

### Import, save and re-import protocol

Preview source profile(s), category counts, unsupported fields, excluded credentials/runtime files, and destination collisions before import. Select settings, command presets, SSH presets, themes, and optionally inert shader documents. User confirms the concrete import preview. Default repeated import to a new named profile or skip conflicting items; explicit replace is a separate choice. Themes can deduplicate by source identity plus content hash; preset arrays need explicit keep/replace because legacy entries lack IDs.

Read selected source files consistently: detect changed content between preview and commit and require refreshed preview. Import into staging under the native root, validate, then commit the selected profile and manifest; on failure roll back native changes. Keep the original directory byte-for-byte untouched. Never execute imported commands or start shells merely because a preset was imported.

Never bulk-copy `control-api.json`, helper executables/scripts, logs, terminal/session history, auth sessions, paid markers, lock/PID files, or secret-vault data. Provider metadata can import; API keys require a separate explicit credential import into the OS store. Remove known key fields from all current and deprecated provider shapes before writing native config. Unknown top-level extension blocks need a preview/allowlist decision rather than blindly copying potential credentials. User may keep the original folder as their existing backup; no second raw secret-bearing copy is required.

Native saves use a common store with revision/lock coordination across native instances, atomic same-directory replacement and error recovery. On Windows implement/test replacement semantics rather than assuming Unix rename behavior. Re-import does not imply bidirectional sync. Revert & Close reverts only that Settings session's changes, not concurrent changes. Do not invent index-based preset merging.

Native rollback copies must be sanitized and restrictive, outside Git, and absent from diagnostics/uploads. Do not delete old native eframe preferences until the user has verified migration. No companion modification to O's settings writer is needed for this design.

## 2. Terminal service contract

UI, CLI, and assistant all use one action dispatcher for create/rename/focus/layout, input/control keys, and preset execution. Stable tab/session IDs survive rename/reorder and are invalidated on close. Distinguish PTY-ready, exited, and closed. Requests against an unready or closed terminal return an explicit error, not success.

Resolve `active` once at action submission, then pin the ID. Reject ambiguous exact titles; preserve original ID/title/PTY selector precedence through contract tests. Layout count (ten visible panes) must not become a ten-tab automation limit.

Input service separates literal text, Enter, bracketed paste, and slow-typed delivery. Preserve UTF-8 character boundaries, CR/LF semantics, cancellation, and bounded payload sizes. Do not evaluate received text outside the chosen PTY. Close/restart during slow delivery must cancel it rather than redirect to a new focused terminal.

Output needs two explicit surfaces:

1. CLI compatibility tail: characterize O `control_api.rs` raw PTY capture/read behavior, including ANSI/control sequences and truncation. Preserve observable `/v1` behavior; do not silently replace it with a current viewport screenshot/text dump.
2. AI snapshot: normalized bounded terminal text plus target ID, capture time, last input/output activity and sequence/revision, truncation, and alternate-screen status. No clipboard or arbitrary files.

N vendored backend owns Alacritty's PTY event loop and its terminal mutex. R02 is a mandatory spike: determine a bounded output-observer seam without adding a second reader to the PTY. If dependency internals must change, isolate the patch, document upstream provenance and test it. Screen hashes cannot reliably substitute for raw output activity (identical redraws, hidden tabs, and alternate screens matter).

Quiet means no observed new output for an interval, not command completion or exit code zero. Track input and output timestamps separately. Use a monotonic clock for elapsed waits; wall time is metadata. Waits must terminate on deadlines, cancellation, closed tabs, or shutdown.

## 3. `buttonsclictl` compatibility and discovery

User's “clictl” refers to the existing `buttonsclictl` helper. Keep its established command surface first. Reuse a pinned, attributed copy of O `scripts/buttonsclictl.mjs` plus tests; Node is optional for this helper, not required to launch the GUI. A standalone Rust helper is a later replacement, tested against the same fixtures.

| Existing HTTP contract | Required behavior |
|---|---|
| `GET /v1/status` | Instance identity, connection/status response field casing exactly as existing client expects |
| `GET/POST /v1/tabs` | List/create, stable IDs, shell validation, PTY readiness/error |
| `GET /v1/tabs/:selector/read` | Bounded tail and tab summary, query defaults/clamps compatible |
| `POST .../send`, `.../key` | Direct/slow input, control key allowlist, byte accounting |
| `POST .../run` | Send plus bounded observation; `stable`, `timedOut`, `completionReason`, text/match fields |
| `POST .../rename` | Deterministic selectors and rename behavior |
| `POST /v1/layout/open` | Horizontal/vertical/grid mapped to native topology, returned visible/active IDs |
| `GET /v1/presets`, `POST /v1/presets/run` | Current profile presets, exact label ambiguity handling, type-only behavior |

CLI `wait-for-text` and `wait-for-quiet` are client-side command families; test them end-to-end too. Preserve stdin/file/base64 payload support and machine-readable output/exit behavior. Capture response/error fixtures from source-defined cases, never from live private terminals.

Bind ephemeral `127.0.0.1` only; authenticate reads as well as writes. Strong per-process tokens; restrictive discovery permissions/ACLs; reject browser origins unless explicitly supported. Bound bodies, concurrency and queue depth; redact Authorization headers and request bodies in logs. Gate execution with `automationRemoteControl`, not merely its Settings button.

Native writes `~/.buttonscli-native/control/<instance-id>.json` with compatible `baseUrl`, `authToken`, `updatedAtMs`, plus additive identity/capability metadata. “Agent Inst.” copies instructions setting `BUTTONSCLI_CONTROL_INFO_PATH` to this exact file, which the existing helper already supports. Do not replace `~/.buttonscli/control-api.json` or overwrite the original helper at startup. Install any native-pinned helper under `~/.buttonscli-native/helpers/`. Tokens remain in discovery files, not clipboard setup text or committed MCP config.

MCP setup must pass the native discovery path explicitly; inspect the template's discovery behavior before reuse. Shutdown removes only a descriptor proven to belong to the exiting instance. Restart rotates token; stale descriptors produce actionable errors. No automatic fallback to another running app when an explicit instance is unavailable.

## 4. AI Help and credentials

Port the current behavior from O `assistantService.ts`, `assistantActions.ts`, `ai/prompts`, `useAIAssistantChat.ts`, and Rust request/stream/model helpers. The original also contains `aiSdkService.ts`; trace the active call path before reusing code. Avoid importing an obsolete parallel transport.

Provider metadata supports named endpoints, manual model entry, optional discovery and connection test. Use direct Rust HTTP calls through an application-owned transport trait. Validate URL composition, endpoint suffix handling, timeouts, redirects and authorization forwarding; support deliberately configured local HTTP providers without globally disabling TLS verification. Test SSE split across arbitrary bytes, UTF-8 fragments, malformed/error responses, non-streaming fallback, cancellation, and bounded answer buffers.

Credential policy: OS credential-store adapter for newly entered native keys; plain config stores only references in a native-owned schema. On first import, offer to use/import existing legacy keys without logging or copying them into sidecar JSON. Do not erase legacy keys (the original still needs them), and explain that later provider-key changes are not bidirectionally shared until both apps support credential references. If the key store is unavailable/locked, allow session-only credentials; no silent plaintext fallback. Avoid tests against real accounts.

AI Help must be an independently movable native window with one app-owned controller. Closing it must not close shells or implicitly approve an action. Reopening reuses state without duplicate workers. Multi-viewport support on the pinned eframe version is a focused prototype/acceptance task, not an assumed cross-platform guarantee; an overlay can be an interim development surface but does not meet M3.

Freeze target ID and bounded context at submission. Show included context/target, allow context off, redact known configured credentials, and document that heuristics cannot guarantee removal of all secrets. Treat output as data, not authorization. Parse structured suggested actions with a strict control-key allowlist. Review actions with visible target/text; executing an old response after tab close/profile change must fail safely. Default to review; no startup restoration of full-permission authorization from imported config.

Agent mode is a later state machine: propose → review → execute → observe fresh output → continue/stop. Stop on no fresh output, timeout, stale target, max steps, access denial, cancellation, or repeated failures. Full permissions requires an explicit current-session choice with a warning. Text Stall Recovery is per-tab, debounced, bounded, and opt-in; vision adds separately opt-in captures of the intended terminal region only.

## 5. Access, localization, privacy

Create a native catalog with the original stable keys, tier, default, owner, rollout and override metadata. Preserve original decisions: `aiHelp`, `automationRemoteControl`, `agenticMode`, `assistantIdleAutomation`, `vibeCodeThemes`, `vibeCodeShaders` are pro; `quickSecrets` and `profileManagement` internal; ordinary new capabilities default free. Distinguish local theme import (free) from hosted `themeSharing` (planned pro).

A named explicit development/all-free mode may enable local testing; do not treat a local config flag or migrated paid marker as canonical purchased entitlement. Inject access decisions in tests. Server entitlements remain authoritative for paid shipping, with any offline grant bounded and expiring. Gate action execution and discovery through the same resolver; do not replicate scattered UI checks.

Externalize new native UI text immediately; reuse original message keys/locales where practical, with tested fallback/interpolation and font coverage. Do not promise RTL or IME correctness based only on translated labels.

Preserve analytics preferences but do not activate network telemetry merely by importing config. If telemetry is implemented, use original stable feature keys and update producer/receiver/schema/docs together. Avoid raw terminal output, commands, paths, provider prompts and secrets in analytics. Runtime failures must not destabilize the terminal.
