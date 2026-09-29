# Reconstruction journal

## 2026-09-29 — P02 internal Quick Secrets vault

Implemented a debug-only internal vault gated through the central feature
resolver and `BUTTONSCLI_NATIVE_DEV_QUICK_SECRETS=1`. It stores a versioned,
profile-bound Argon2id/AES-256-GCM ciphertext file outside settings and legacy
imports. Create/unlock KDF work runs off the UI thread; manual, idle, window
close, profile change, and app shutdown clear the in-memory key and entries.
Forgetting the passphrase requires a second confirmation before deleting the
vault. A user must select a stable ready terminal for each single-line Paste or
Paste + Enter action. Sensitive input uses a zeroizing PTY queue and bypasses
the last-input observer; shell echo can still appear in output, including
opt-in AI Help context. No legacy vault data is imported.

All 168 Windows library tests pass, including fake-repository encryption,
wrong-passphrase, profile binding, corruption, idle lock and conflict cases;
disk revision/delete, import bounds, localized safety copy and multiline
rejection are also covered. The interactive GUI and PTY echo behavior remain
unverified. Access tier: internal; the feature remains hidden in release
builds.

## 2026-09-29 — P01 fail-closed runtime config client

Added a bounded, redirect-free runtime-config request using the existing
native HTTP transport. It runs in a background thread after the app starts,
uses the hosted runtime-config URL, refreshes every five minutes, accepts only
HTTPS remotely (loopback HTTP is reserved for tests), and defaults missing
flags or all errors to closed. A failed refresh clears previously cached
flags. Feature gates now read the current runtime flags instead of assuming
Pro is enabled. The smoke harness disables the remote read. Account sign-in,
secure session storage, entitlement refresh, and paid rollout verification
remain open; no hosted service was changed. Access tier: internal rollout
plumbing.

## 2026-09-29 — P01 native account session path

Added a native client for email-code start/verify, `/me`, entitlement resolve,
and logout. The Account Settings tab uses the existing localized source
catalog. Session token and expiry are stored under a dedicated OS credential
service with profile-scoped references; partial writes roll back. Startup
revalidates the saved session with the server, and grants refresh every five
minutes. Expiry, revocation, network failure, or a failed refresh closes the
grant. Account sign-in is free (`accountSignIn`); Pro features still require
the central rollout and their own server grant. The isolated smoke disables
both account and runtime-config requests. Live service behavior is unverified,
and native Turnstile is not implemented. No hosted service was changed.

## 2026-09-29 — P01 server feature-grant groundwork

Extended native entitlements to carry the server's explicit `activeFeatures`
list. The central access resolver now honors a grant only for its named,
active feature, after the existing kill-switch, rollout and expiry checks. A
free-plan grant test confirms that AI Help can be granted without unlocking
remote control, and that the grant expires closed. This does not activate any
Pro feature: the native auth/runtime-config adapter, secure session lifecycle,
and verified hosted rollout are still required.

## 2026-09-29 — P05 Windows isolated GUI startup smoke

Added `scripts/native-smoke.ps1`. It builds the native executable, launches
only the test-owned process with `USERPROFILE`, `HOME`, `APPDATA`, and
`LOCALAPPDATA` redirected to a unique temporary profile and its working
directory set to that profile. It locates the `ButtonsCLI` window by the
test-owned PID, verifies that the window is visible and large enough, then
closes the app. The smoke passed on Windows; an optional Appsnap capture also
confirmed a full-size first-run screen. No original ButtonsCLI process, user
profile, provider, or terminal command was used. WGPU reported that the
optional Vulkan validation layer is unavailable; this was nonfatal. This
confirms startup and first-frame rendering only, not layout interaction, PTY
behavior, detached windows, or Windows platform parity. Linux and macOS
acceptance remain separate follow-up checks.

## 2026-09-29 — V04 Shader Lab renderer decision

Compared the original WebGL image-composite contract with the native
`egui_term`/egui painter path. A custom wgpu callback can paint in egui's pass,
but cannot sample the terminal output already painted there. A real Shader Lab
port first needs an offscreen per-pane render texture and a bounded WGSL
pipeline; arbitrary GLSL conversion is not safe. Added compatibility metadata
and a localized Theme Library warning for imported themes that request a legacy
shader. The flag remains inert and shader JSON remains preserved. Shader Lab is
still partial: editor, compiler, preview, device-pipeline fallback and live GUI
checks are open. Access tier: free beta (`shaderLab`); AI shader generation
(`vibeCodeShaders`) remains Pro and waits for that renderer/compiler contract.

## 2026-09-29 — V03 AI theme generator

Added a Pro-gated AI palette generator to native Themes settings. It uses the
active configured provider and sends only the bounded style brief plus a small
palette seed; it does not include terminal contents or API keys in the request
body. Generated JSON is constrained to app/terminal colors, validates all
required six-digit colors and minimum app/terminal text contrast, and receives
at most one correction request. The resulting in-memory candidate records
provider/model provenance, preserves fonts, effects and unknown theme data,
and waits for the user to preview or open it in the editor. Saving uses a new
collision-safe filename and never replaces a saved file. Access tier: Pro
(`vibeCodeThemes`); release builds remain locked pending entitlement service,
and debug builds require `BUTTONSCLI_NATIVE_DEV_THEME_GENERATOR=1`.

Six mock-transport tests cover accepted and rejected output, field retention,
palette-only provider context, prompt bounds, cancellation, and the two-request
maximum. The full Windows library suite now passes 148 tests and `cargo check --bin
buttonscli` plus `cargo build --bin buttonscli` pass. Live provider and
candidate GUI checks remain open.

## 2026-09-29 — V02 remaining effects audit

Audited TV noise, glow, and wallpaper against the original source. TV noise has
configuration fields but no runtime renderer; glow is part of Shader Lab GLSL
presets; wallpaper has no legacy setting or renderer. Recorded those decisions
so subsequent work does not invent parity. V02 source behavior is implemented,
but 1/4/10-pane screenshots and frame-cost review remain open.

## 2026-09-29 — V02 simple noise and idle ramp

Added native simple noise as a bounded pixelated grayscale mesh, with legacy
theme settings for amount, resolution, FPS, brightness range, and optional
activity-based idle ramp. The idle schedule reads the latest input/output time
from the visible PTY sessions, wakes at the configured delay, animates through
the ramp, and stops repainting once the target is reached. Master-off, calm
mode, and focused-pane policy suppress it. Each pane is capped at 1,024 noise
cells to keep work bounded. Access tier: free (`effectsMasterSwitch`).

Tests cover legacy parameter mapping and theme export, activity timing, wake and
settle behavior, master-off, deterministic noise generation, mesh bounds, and
the per-pane cell cap. The native GUI, multi-pane frame cost, and screenshots
remain unverified. The legacy `tvNoise*` fields have no runtime effect renderer
in the inspected original source, so they remain unsupported preserved data.

## 2026-09-29 — V02 HSync decision and row banding

Recorded a native-renderer no-go for pixel-faithful HSync: the original
composites xterm canvas layers and shifts every physical scanline, while the
native renderer paints cells and glyphs directly with egui shapes. A correct
port needs a separate bounded offscreen/GPU rendering spike. Implemented the
independent row-banding effect using the renderer's actual terminal cell height,
with color and opacity settings in personal themes, master-off and focused-pane
gates, and preserved JSON export. It uses an every-other-row tint, matching the
original visual cadence. Access tier: free (`effectsMasterSwitch`).

Automated coverage checks legacy parameter projection, master-off behavior,
focused-pane suppression, alpha/color resolution, theme export, exact cell-pitch
geometry, and clipping of a partial final row. Windows GUI screenshots and
frame-cost checks at 1/4/10 panes remain open. TV noise, glow, and wallpaper are
still unimplemented.

## 2026-09-29 — V01 repeating gradients and focused effects

Ported repeating linear, radial, and conic gradient modes through theme
projection, rendering, personal-theme editing, and export. Added a persisted
focused-pane-only setting that removes background effects from other panes
while preserving their theme colors. The effects master-off flag now suppresses
gradient animation as well as static and scanline effects. The tier remains
free under the existing `effectsMasterSwitch` and `calmThemeApply` entries.

Automated checks cover repeat stop sampling and mesh bounds, each repeating
geometry, master-off projection, focused/unfocused effects, and theme export.
Windows `cargo test` passes (133 library tests, 2 fixture integration tests),
the vendored renderer suite passes (14 tests), and Windows check/build plus the
WASM library check pass. The native GUI and visual/frame-budget behavior have
not had a live review.

## 2026-09-28 — U10 Cool Stuff installer and provider links

Copied the exact source Windows, Ubuntu, and macOS installer scripts into the
native assets and embedded them in the application. The Cool Stuff menu shows
the platform-specific summary, install list, recommended shell, script name,
command, notes, and next steps. It extracts a content-hash-named copy into the
active native profile, supports clipboard copy, and types the quoted command
into a new focused terminal through the stable-ID dispatcher without Enter.
The Windows path prefers PowerShell 7, then Windows PowerShell. Wrong-platform
choices disable actions. The original OpenRouter, OpenAI sharing, NVIDIA Build,
Groq, and Mistral links open the system browser. Nothing downloads or executes
until the user reviews the terminal and presses Enter. See
[`COOL-STUFF.md`](COOL-STUFF.md).

The Windows library suite passes (128 tests), including platform mapping,
command quoting, no-Enter output, PowerShell preference, wrong-host refusal,
external HTTPS links, and safe profile-scoped extraction. Formatting,
`cargo check --bin buttonscli`, and `cargo build --bin buttonscli` pass. Live
terminal/UI review remains open; the build prints the existing Rust bin/lib
PDB filename collision warning. Access tier: free (`coolStuffInstallers`).

## 2026-09-28 — U09 Windows window opacity

Added a central-registry free feature gate and persisted main-window opacity
from 25% to 100%. Windows applies the setting to the native window handle;
Linux and macOS show an explicit unsupported-capability message. The original
`window.opacity` preference now imports through the same bounds. The UI lives
under Settings → Workspace. See [`WINDOW-APPEARANCE.md`](WINDOW-APPEARANCE.md).

The Windows library suite passes (123 tests), including finite/range
normalization, imported-value projection, and feature access. `cargo check` and
`cargo build --bin buttonscli` pass; live opacity behavior and manual
monitor/DPI/compositor review remain open. Access tier: free
(`windowTransparency`).

## 2026-09-28 — U08 offline system and custom fonts

Added local system-font discovery for Windows, Linux, and macOS font folders,
plus validated `.ttf` / `.otf` import into the active native profile. Imported
files are size-bounded, checked by both the OpenType parser and egui's font
parser, stored without overwriting existing files, and registered immediately.
Missing selections fall back to bundled UI/mono families; Unicode uses the
existing bundled symbol/Noto fallback chain. Terminal settings still select
regular and bold faces independently, and cell sizing remains based on the
regular face. No download or network path was added. See [`FONTS.md`](FONTS.md).

The Windows library suite passes (120 tests), including invalid-file rejection,
filename collisions, isolated offline loading, bounded directory traversal,
missing-font fallback, separate regular/bold face selection, and Hangul glyph
fallback. `cargo fmt --all -- --check`, `cargo check --bin buttonscli`, and
`cargo build --bin buttonscli` pass. The binary build retains the existing
Rust bin/lib PDB filename collision warning. Live selector review and startup
time measurement on large font installations remain open. Access tier: free
(`customFonts`).

## 2026-09-28 — U07 personal theme library and editor

Added a free `personalThemeEditor` feature entry and a Settings editor for
creating variants, changing supported colors/effects/dividers, previewing the
current draft, saving, importing, exporting, and confirmed deletion. Theme
documents remain scoped to the active native profile. Saves and exports are
atomic/create-new respectively, filename collisions receive safe suffixes,
and unknown JSON fields survive round trips. Preview follows the existing
theme-apply scopes and cancellation restores the scoped preferences without
restarting terminal sessions. See [`PERSONAL-THEMES.md`](PERSONAL-THEMES.md).

The full Windows library suite passes (116 tests), including starter-document
projection, unknown-field round-trip, import/export, filename collision races,
storage isolation, preview/cancel and save/reload. `cargo fmt --all -- --check`,
`cargo check --bin buttonscli`, `cargo build --bin buttonscli`, and
`git diff --check` pass. The build still reports the existing bin/lib PDB
filename collision. Windows GUI visual acceptance remains open. Access tier:
free (`personalThemeEditor`).

## 2026-09-28 — U06 Windows shell discovery and profiles

Expanded the free `shellProfiles` feature with Windows WSL detection. When WSL
is present, the shell menu keeps the generic launcher and adds one profile per
installed distribution, passing each distribution name as a distinct argument.
The WSL list parser accepts UTF-8 and UTF-16 output, removes duplicate names,
and handles names with spaces. Windows shell discovery now checks System32 as
well as `PATH`; existing detected-profile IDs stay stable.

Improved custom command-line parsing for quoted paths, escaped embedded quotes,
empty arguments, and Windows backslashes. Shell launches continue to report the
underlying error rather than falling back to another profile. Updated the
Windows user guide, README, migration tracker and changelog. Focused tests and
The full Windows library suite passes (107 tests), as do `cargo check --bin buttonscli`
and `cargo build --bin buttonscli`, `cargo fmt --all`, and `git diff --check`.
The build retains the existing bin/lib PDB filename collision warning. Linux,
macOS, and live Windows GUI/PTY behavior remain unverified; Fedora is not a gate.

## 2026-09-28 — U05 language selector and catalog

Bundled the original 21-locale catalog (776 unique English source strings) and
routed existing native message keys and static app labels through it with
English fallback for unmatched strings. Added a first-run system/manual
language chooser, a scrollable Settings → Language & Region page, Windows user
locale detection, import projection for legacy language settings, and a free
`localizationSettings` catalog entry. Existing native settings default to
confirmed English, so an upgrade does not show the chooser again. See
[`LOCALIZATION.md`](LOCALIZATION.md).

Added an OFL-licensed, offline Noto Sans KR fallback subset with all 11,172
modern Hangul syllables; attribution and the license are in `assets/fonts/`.
`cargo fmt --all`, Windows `cargo check --bin buttonscli` and build, five
focused locale, font-catalog, settings, import-projection and onboarding tests,
and `git diff --check` passed.
The test build reports the existing bin/lib PDB filename collision. Static font
cmap inspection confirms the intended glyph ranges; actual rendering,
long-label focus, RTL/IME behavior, and live Windows UI remain unverified.

## 2026-09-28 — U03 dock and status controls

Added persistent dock width, compact SSH buttons, an auto-hide rail/overlay,
and workspace settings for delay, opacity and peek distance. The overlay does
not resize the central terminal area when opening or closing and does not
change the focused session. Added status-bar terminal zoom with a percentage
reset and a quick calm-effects toggle. Original-profile import now maps the
supported dock settings with native bounds and warnings. The free
`workspaceControls` feature entry gates discovery and actions.

`cargo fmt --all`, Windows `cargo check --bin buttonscli` and
`cargo build --bin buttonscli`, 5 dock tests, 1 settings normalization test,
1 import projection test, 1 feature-access test, 3 i18n tests, and
`git diff --check` passed. The build emits the existing bin/lib PDB filename
collision warning. Fake-time coverage checks reveal, delayed close, reopening,
bounded delay and no repaint once closed. Windows GUI interaction and
cross-platform checks remain pending.

## 2026-09-28 — U04 detached Settings and preview rollback

Moved the existing Settings surface into its own eframe viewport, with an
embedded egui window when the backend does not support another native window.
Theme, font, and per-terminal choices continue to apply to the live workspace.
Added Keep Changes and Revert and Close; rollback restores the preference
snapshot and per-terminal theme overrides without replacing or retargeting
terminal sessions. A completed profile import establishes a fresh rollback
point, while OS credential-store writes remain outside rollback.

`cargo fmt --all`, Windows `cargo check --bin buttonscli` and
`cargo build --bin buttonscli`, two focused app tests, three i18n tests, and
`git diff --check` passed. The tests cover restoring preferences/theme
overrides without changing focused session state and rendering the embedded
viewport fallback. The build emits the existing bin/lib PDB filename collision
warning. Monitor/DPI movement, focus, keyboard interaction, and live GUI/PTY
behavior still require manual review.

## 2026-09-28 — U02 terminal search and buffer actions

Added focused-terminal regex search against Alacritty's live grid, with wrapped
and wide-Unicode matches, scrollback navigation, wraparound next/previous, and
visible match highlights. Added Terminal-menu actions for Find, Select all,
and Clear screen. Clear screen clears the active viewport in place, keeps the
shell running, and leaves cleared lines in scrollback. Search targets the
focused session even when its pane is not rendered. Added the free
`terminalSearch` feature-catalog entry and native UI strings. See
[`TERMINAL-SEARCH.md`](TERMINAL-SEARCH.md). Access tier: free.

`cargo check --bin buttonscli` and five focused `egui_term` backend tests passed
on Windows, including wrapped wide-Unicode search, a synthetic scrollback
match, navigation wraparound, select-all, and clear-screen state. The desktop
GUI, real PTY clear behavior, and Fedora interaction have not been manually
verified. The separate vendored-crate test run created an untracked lockfile;
it is cleanup-only and will not be retained.

## 2026-09-28 — U01 editable native shortcuts

Added a persisted shortcut map and Settings recorder for new tab, close,
reopen, copy, paste, open Settings, and quit. It keeps the existing
Primary+Shift defaults, supports platform-aware Primary/Ctrl/Alt/Shift
matching, rejects duplicate bindings and Ctrl+C/Primary+C without Shift, and
lets users clear one binding or restore defaults. Unknown future shortcut
entries survive round trips and reserve their chords. Added the free
`keyboardShortcuts` catalog entry. Original custom keyboard preferences stay
preserved in compatibility data but are not activated because several of the
legacy actions do not exist in this native build.

`cargo fmt --all`, Windows `cargo check --bin buttonscli` and
`cargo build --bin buttonscli` passed. Four focused shortcut tests and three
feature-access tests passed. The Settings recorder has not had manual GUI
acceptance. The wasm target is not installed in this Windows toolchain, so this
shared settings UI has not been cross-compiled there.

## 2026-09-28 — C05 optional MCP adapter source and protocol smoke

Adapted the read-only original stdio helper template into a native MCP server
that reads only `~/.buttonscli-native/control/`, validates descriptors, and
connects only when one live instance is found. The app embeds and installs a
content-hashed copy; **Agent Inst.** now includes a ready-to-paste MCP config
that points at the stable control directory, never at a rotating token. Added
bounded JSONL request lines, graceful draining for in-flight tool calls,
actionable disconnect errors, bracketed delivery metadata, and a fake-API
initialize/list/status smoke script. The helper remains optional and the API's
Pro gate applies to every route. Access tier: Pro
(`automationRemoteControl`).

`cargo fmt --all`, Windows `cargo check --bin buttonscli` and
`cargo build --bin buttonscli`, both Node syntax checks, and
`node scripts/test-mcp-smoke.mjs` passed. The smoke used a temporary fake
loopback API and confirmed authenticated discovery and status, exposed tools,
MCP initialization, and no token in stdout. A real MCP client, full route
matrix, desktop handoff click, and cross-platform behavior remain unverified.

## 2026-09-28 — R02–R04 and C01–C04 native control source pass

Vendored Alacritty's optional observer seam at its existing single PTY reader and added a 200,000-character raw output tail with input/output timestamps and sequence metadata. Added bounded raw, bracketed and paced UTF-8 delivery; paced requests stop on cancellation/closed targets, cap at 512 Unicode characters per request, and cap at 30 seconds. Implemented authenticated loopback `/v1` status, tabs, read/send/key/run, create/rename, layout, and preset routes through the stable-ID app-thread dispatcher. Added a per-instance descriptor, version-hashed native Node helper with no fallback to the original app, and an **Agent Inst.** clipboard handoff containing an exact descriptor path but no token. Updated API, migration, decision and changelog docs. Remote control tier: Pro (`automationRemoteControl`); debug-only override is `BUTTONSCLI_NATIVE_DEV_REMOTE_CONTROL=1`, release remains unavailable pending entitlements.

`cargo fmt --all`, Windows `cargo check --bin buttonscli`, `node --check scripts/buttonsclictl.mjs`, and the helper's `--help` path passed. No tests, live GUI/API/PTY interaction, provider request, or output benchmark was run. Therefore the route behavior, handoff clipboard, paced input cancellation, output transcript fixtures and throughput remain runtime-unverified; fixed-size responsive grid behavior honors requested columns only within minimum-pane bounds. At this entry, the optional MCP adapter had not yet been implemented; see the later C05 entry. Full locale translation and platform acceptance remain open.

## 2026-09-28 — A03/A04/A05/A06 AI Help source implementation, Windows build pass

Added bounded SSE framing and response parsing for OpenAI-compatible chat completions, including non-streaming JSON fallback, partial deltas, a 64 Ki-character answer cap, at most two validated command/control suggestions, and a 1 MiB serialized request cap. AI Help now has a separate deferred viewport with an embedded fallback, streamed in-session turns, cancellation, retry, optional terminal preview, and explicit Insert / Insert + Enter / reviewed-key delivery. Follow-up requests include recent successful conversation history. Preview reads the already-owned terminal grid snapshot, masks the configured provider key and obvious secret patterns, and requires a successful credential lookup before enabling send. Actions are pinned to stable session IDs and revalidated through the app-thread dispatcher. Access tier: pro (`aiHelp`); release builds remain locked pending entitlements.

`cargo fmt --all` and Windows `cargo check --bin buttonscli` passed after the source edits. No tests or interactive GUI/provider checks were run in this batch. Therefore the viewport and action flow are source/build complete but not manually accepted. Context is a best-effort screen/scrollback rendering, not raw PTY output; the full R03 capture service is still open. Input supports bounded literal command bytes and a fixed set of key sequences; bracketed paste and paced delivery remain open under R04. See `docs/migration/OUTPUT-CAPTURE-DECISION.md` and `docs/AI-HELP.md`.

## 2026-09-28 — A02 provider HTTP and model discovery, Windows source pass

Added an app-owned blocking Reqwest transport used only on worker threads, with TLS verification, no redirects, 5-second connect/20-second total timeouts, and a 1 MiB response cap. The provider editor now tests its selected model using a short chat request and can discover/sort compatible model IDs; empty discovery leaves manual entry intact. HTTP errors report status only, and transport errors do not echo URLs, response bodies, or keys. The Pro gate remains closed without entitlements; debug builds require the explicit `BUTTONSCLI_NATIVE_DEV_AI_HELP=1` override. Fake-transport unit cases are present but were not run in this pass; Windows binary `cargo check` passed. Live local fake-server behavior and provider UI review remain. Access tier: `aiHelp` is pro.

## 2026-09-28 — A01 provider metadata and credentials, Windows source pass

Added named provider metadata in native preferences, an AI providers Settings tab, editable HTTP(S) endpoints and model IDs, OS credential storage through `keyring` 3.6.3, and explicit session-only keys. Credential calls run on workers; UI shows only generic errors and never persists key values. Legacy import now offers a separate unchecked key-transfer choice. Only keys corresponding to imported providers transfer; old files are read only, and older native imports recover provider metadata from their sanitized compatibility file. Added fake-store tests for success/failure, key redaction, source preservation and recovery. Windows library tests and strict Clippy pass. A live credential-store/GUI pass and AI request pipeline remain. Access tier: `aiHelp` is pro; provider configuration itself does not execute AI or imply entitlement.

## 2026-09-28 — R01 stable session action dispatcher

Added `session/actions.rs` with original-style active/ID/tab-ID/title resolution, ambiguity errors, a 64-request bounded reply queue, deadlines and explicit access/readiness/closed/unsupported errors. Submission pins the active terminal ID; execution rechecks that ID on the app thread, including hidden sessions. Native create/reopen/focus/rename/move/close/layout/count UI mutations now pass through this action path. Rename dialogs pin IDs rather than vector positions. Tests cover queued focus changes, reorder/close, duplicate titles, hidden IDs, saturation, deadlines and failed shell launch; existing pane remap tests continue to pass. This is free core infrastructure. The future control server must request repaint after background enqueue and apply its own feature-access decision before submission. `Send` remains deliberately unsupported until R04.

## 2026-09-28 — L08 per-terminal and random themes, source pass

Added stable-session-ID terminal theme overrides and Theme Settings actions for This terminal, Theme all, Random current/all and Use global. Theme all updates the persisted default and clears overrides even for hidden tabs; new tabs inherit it. Random selection excludes each terminal's current theme when alternatives exist. Terminal palettes and supported gradient/effect settings change through the existing live TerminalView path; app chrome remains global. Recent-close recovery carries an override to the new session, while restart clears overrides because sessions are not restored. Tests cover ID-scoped presentation, theme-all reset and non-repeating random choices. Windows source tests and Clippy pass; Fedora live PTY, 1/4/10-pane, personal-theme and contrast checks remain. Access tier: free (`themeSelection`).

## 2026-09-28 — L09 saved chrome corner radius, source pass

Added one 0–16 point radius preference for tab/control widgets, cards, menus and Settings, with a Theme Settings slider and native settings round-trip. Old settings get the restrained 6-point default, and oversized saved values clamp on load. Terminal cell clipping stays unchanged. Windows library tests passed; Fedora/Windows GUI DPI, focus and hit-target review plus a WASM build are still pending. Access tier: free (`settingsAppearance`).

## 2026-09-28 — L07 visible saved pane dividers, source pass

Kept the 10-point drag gap and now paint a centered divider line at idle and during hover. Added theme defaults from optional personal theme `theme.app.shell.paneDivider` data plus native preference overrides for color and 1–6-point painted width. Settings exposes theme inheritance and custom controls; old settings inherit automatically. Tests cover serialization, theme parsing and override clamping. Native theme editor/export is not implemented, and Fedora/Windows contrast and drag interaction remain unverified, so L07 is partial. Access tier: free (`paneDivider`).

## 2026-09-28 — L05 real-grid scrollbar source implementation

Exposed retained history, viewport lines, display offset and alternate-screen/mouse-reporting mode from the vendored Alacritty adapter without a new PTY reader or outer ScrollArea. The native pane reserves a narrow track; wheel and thumb drag both use the backend's `Scroll` command. Geometry tests cover fresh output, resize/truncation, top/bottom dragging and hidden modes; a vendored-grid test exercises actual scroll-up, top and history truncation. Windows `cargo test` passed 68 library and 2 fixture tests; vendored `egui_term` passed 7 tests; strict Clippy passed with the known unrelated test-module lint exempted. Fedora/Windows GUI selection, PTY-size and alternate-screen checks remain open before L05 acceptance. Access tier: free (`terminalScrollbar`).

## 2026-09-28 — L03 responsive pane layout source implementation

Added a pure, stable-session-ID layout reducer for COL, ROW and GRID. It plans from the available workspace size using 320 × 160 logical-point review bounds plus a font-size estimate for 40 columns and 8 rows, keeps the focused session in view, and hides overflow without closing PTYs; the tab strip and status count expose the effective view. The render tree wraps COL across/down and ROW down/across, with shape-specific divider keys so ratios return after resizing back. Wide/narrow/focus, large-font and new-tab-at-capacity tests were added. Windows release suite passed 64 library and 2 fixture tests; the final source additions passed 65 debug library tests and strict Clippy with the known unrelated lint exempted. A live Fedora X11/Wayland GUI and `stty size` pass are still needed before L03 acceptance. Access tier: free (`paneLayout`).

## 2026-09-28 — S05/S06 previewed snapshot import

Added a Settings preview for the original app's active or a selected additional profile, with destination, counts, warnings, provider metadata, detected-key count and explicit exclusions. Import runs on a worker, rereads source before and during a locked staged commit, publishes a new native profile, and rolls it back if the final metadata update fails. A sanitized content fingerprint makes identical repeats idempotent without overwriting native edits; changed content gets a new profile. Source byte hashes remain transient, so manifests identify only sanitized content. Known API-key fields, endpoint URL user info/query data, runtime/auth files and session history are excluded. The original folder is never written. Strict Clippy first rejected oversized import event variants; boxing the preview/preferences fixed it. Windows release tests passed 64 library tests and 2 fixture tests, and later focused changes passed 65 debug library tests and strict Clippy with only the pre-existing test-module lint exempted. UI interaction, Fedora behavior, credential transfer and general profile switching remain open. Access tier: free (`originalSettingsImport`).

## 2026-09-28 — Migration documentation sync

Updated the planning baseline and parity backlog to distinguish implemented native storage, theme loading and wrapped tabs from pending original-app import, pane reflow and Fedora GUI acceptance. This corrects stale source-audit statements without changing runtime behavior.

## 2026-09-28 — S04 native personal theme loading

The active native profile now loads version 1 personal theme JSON documents beside the 559 embedded choices. Internal IDs include profile and filename, so duplicate legacy metadata IDs cannot replace embedded themes or each other. Raw documents retain unsupported effect fields; bad/oversized/escaped files produce isolated warnings. The first focused test incorrectly classified embedded `basic2` as a native theme; source shows it is a legacy bundle, and the assertion was corrected. Temporary-profile tests cover collisions, a bad file, preserved data, profile isolation and the full embedded catalog. Original profile themes are not imported yet. GUI theme switching without PTY respawn remains a manual check. Access tier: free (`themeSelection`).

## 2026-09-28 — S03 native settings store

Added versioned `native.json` in `~/.buttonscli-native/profiles/<profile>/`, native active-profile metadata, same-directory temp replacement, file locking and revision checks. Desktop startup prefers native data and reads prior eframe preferences only when no native document exists; the old eframe data is retained. A malformed native file or stale second instance blocks overwrite and surfaces an error. Guard checks prevent the native root or redirected child paths from writing into the original settings folder. Tests cover Windows replacement/restart, stale instances, corrupt document and unusable destination. The first lock implementation used Rust 1.89's API and failed the crate's Rust 1.85 MSRV lint; replaced it with `fs2` 0.4.3. No original app settings were read or written. Import remains S05, and Fedora/Windows GUI restart validation is pending. Access tier: free.

## 2026-09-28 — S02 pure legacy config projection

Added JSON document parsing and a UI-free projection for command/SSH presets, shell profiles/default selection, bundled font-compatible typography, locale, provider endpoint/model metadata and retained visual/settings objects. Explicit empty preset lists stay empty; absent `sendEnter` follows the original trailing-newline rule, while explicit false and command whitespace survive. The serialized compatible copy removes known credential fields recursively and omits unknown top-level blocks pending an import choice. The first focused test had an invalid expectation that the fixture contained an unknown top-level key; corrected it and added a separate unknown-key case. No real settings were read or written. Import preview, persistence and revalidation remain S03–S05. Access tier: free.

## 2026-09-28 — L04 wrapped native tab strip

Replaced the fixed 40-point, sideways-scrolling tab row with egui 0.31's `horizontal_wrapped` layout in a content-sized top panel. Existing tab activation, rename, menus, add/reopen and reorder actions remain on the same controls. README records the visible behavior; access tier: free (`tabWrapping`). Windows full release tests (44 library, 2 fixture) and Clippy with the known unrelated lint exempted passed. Narrow-window/Fedora X11 and Wayland interaction, long-name clipping and DPI behavior still need GUI review; source/API checks are not those manual checks.

## 2026-09-28 — S01 separate data roots and read-only legacy resolver

Added native `~/.buttonscli-native` and legacy `~/.buttonscli` root types with injectable home paths. The resolver reads active-profile metadata, prefers that profile's config, uses the old root config only if the profile file is absent, rejects malformed/oversized metadata and out-of-root canonical paths, and writes nothing. Tests use disposable roots with spaces/Unicode; they cover fallback, invalid names, source preservation and containment. Windows full release suite passed (44 library and 2 fixture tests); Clippy passed with only the pre-existing test-module lint exempted. WASM target is not installed here; the module is native-gated. S02/S03 still need projection and persistence, and every later import read must revalidate source paths and content before commit. Access tier: free.

## 2026-09-28 — L02 stable `termN` tab names

New PTYs receive `term1`, `term2`, etc. from a run-local monotonic title counter, skipping an already open matching custom title. Shell name, working directory and terminal-reported title remain separate metadata; terminal title escape sequences no longer replace the visible default or explicit rename. Reopened tabs get a new number unless they had an explicit custom name. README explains close/restart behavior. Access tier: free (`tabNaming`). Windows release tests and Clippy were run; Fedora GUI and future CLI selector acceptance remain open.

## 2026-09-28 — L01 layout source decision

Recorded current COL/ROW/GRID behavior for 1–10 panes and target wrap/minimum-size rules in `docs/migration/LAYOUT-DECISION.md`. Source confirms both pane and tab strip need independent wrapping. Fedora X11/Wayland interaction and screenshots are still pending, so L01's manual acceptance is open. No runtime code changed. Access tier: free.

## 2026-09-28 — F04 native message lookup

Added a typed lookup for import, control and plain AI Help strings with named interpolation, the original app's 21 supported locale codes, and English fallback for partial locale entries. Spanish, French, Japanese and German have a few native entries; the remaining migration strings currently fall back to English. The full existing UI translation and locale selector remain U05. Three focused release tests and Clippy with the known unrelated test-module lint exempted passed. Access tier: free infrastructure.

## 2026-09-28 — F03 feature access foundation

Added a native catalog and pure access resolver with stable keys, tiers, rollout state, owner, runtime flag metadata, injected entitlement source, expiring cached grants, development override and kill-switch precedence. Layout/theme/scrollbar/divider/settings appearance are free; AI Help and remote control remain pro and unreleased; internal features stay hidden. Three focused Windows release tests passed. Strict Clippy found the pre-existing `items_after_test_module` lint in `src/terminal.rs`; Clippy passed with only that lint exempted. This is a foundation only: UI and future action dispatchers must both call the resolver when their features are wired. No paid entitlement is inferred from local settings. Access tier: free infrastructure.

## 2026-09-28 — F02 native settings model extraction

Moved preferences, shell profiles, command/SSH presets and theme-scope serialization into `src/settings.rs` without changing the app's eframe storage path or editor behavior. Added tests for explicit empty preset lists and command whitespace: JSON decoding keeps whitespace, while the existing editor normalization still trims it. Focused Windows release tests passed (2 new; prior full extraction build passed 29 existing plus 2 fixture tests), and `cargo fmt --all -- --check` passed. Native root/import persistence is still S01–S05. Access tier: free.

## 2026-09-28 — F01 synthetic migration fixtures

Added invented legacy profile, root fallback, theme collision, malformed metadata, preset, and fake provider-key fixtures with a source matrix. Verified the original preset newline rule in `PresetBar.tsx`; no real settings were read. Windows Rust 1.96: baseline `cargo test --release` passed 29 tests, and `cargo test --release --test legacy_fixtures` passed 2. This freezes examples only; import behavior remains unimplemented. Access tier: free infrastructure.

## 2026-09-27 — Plan refined for native daily use and Fedora

Updated `docs/migration/` after the user narrowed AI Help to explanations and
reviewed command suggestions and reported layout issues. Removed Agent Mode,
full-permission and Stall Recovery implementation tasks. Added bounded tasks for
COL/ROW/GRID repair, wrapped pane and tab rows, `termN` names, visible
scrollbars, random/per-terminal/theme-all controls, saved divider colors,
rounded chrome and detached Settings. Existing Tauri provider endpoints/models
are in the import preview; selected API keys can be imported to a native OS
credential store after explicit confirmation, without copying control/auth
files. Fedora X11/Wayland is the first manual validation target.

Reviewed upstream `egui_dock`: current 0.21 targets egui 0.36; 0.16 matches the
app's pinned egui 0.31. Its docking features may help, but automatic responsive
grid behavior still needs a product layout reducer. Added a version-pinned
Fedora prototype task and `docs/migration/DOCKING-RESEARCH.md`. Confirmed the
locally installed egui 0.31.1 source exposes secondary viewport APIs; actual
window behavior remains for the Fedora probe. No runtime code or live settings
were changed. Documentation references and Git diff checks were run; a GUI
smoke, dependency build and OS credential-store import are not yet verified.

## 2026-09-27 — Source-audited migration specification

Compared the native models/backend/theme loader with the current local Tauri
config, CLI, assistant, feature-access, docs and test contracts. Added
`docs/migration/` with priorities, compatibility rules, 51 bounded tasks,
acceptance tests, platform/performance gates and a Luna handoff prompt. Updated
the backlog execution order and README links. User clarified that native should
use a separate settings folder: planned `~/.buttonscli-native/` with optional
previewed import from `~/.buttonscli/`, not live shared writes.

Fresh Windows `cargo test --release`: 29 passed, zero failed. No runtime code,
personal settings, original repo files, services or live terminals changed.
Document links/task references were checked. An initial combined documentation
patch failed on the journal heading; reapplied with the actual heading. GUI,
performance and cross-platform acceptance remain future work; test success
does not certify those behaviors.

## 2026-08-14 — Investigation and foundation

The legacy application was inspected as a product rather than treated as a
code port. Its most valuable ideas are the terminal-first layout, compact tab
strip, quick command dock, extensive theme controls, and a status surface that
keeps shell state visible. The new implementation will preserve those ideas
without preserving the React/Tauri split or its IPC cost.

The architecture review compared GPUI, eframe/egui, Warp's native stack,
Alacritty, WezTerm PTY components, and the existing `egui_term` adapter.
Eframe/egui won because it offers a credible native GPU path on all three
desktop platforms and a supported WASM renderer. Alacritty remains responsible
for the difficult terminal semantics. This keeps the first milestone small
enough to test honestly while leaving clean seams for a custom renderer later.

The first checkpoint is deliberately boring in the best way: a clean local Git
repository, an open-source-ready license and README, architecture notes, and a
private ignored directory for anything that must not ship.

## 2026-08-14 — The first real terminal

The native window now owns a real login shell through Alacritty's PTY/event-loop
stack. Input, output, live grid resize, scrollback, selection, tab creation,
theme changes, font scaling, command presets, settings persistence, and clean
window exit all work in one process. A manual X11 smoke test resized the window,
generated 220 output lines, scrolled backward through the terminal grid, and
copied a multiline selection back into the shell.

That clipboard test caught a useful upstream adapter flaw: its declared
Ctrl+Shift+C/V bindings did not dispatch the actions, and pasted text could be
mistaken for Ctrl+V when the asynchronous clipboard response arrived after the
modifier keys were released. The small MIT adapter is now vendored with a
focused fix and its original license retained. JetBrains Mono is bundled under
the SIL Open Font License so cell metrics and the reference look are stable.

## 2026-08-14 — One UI, safe browser demo

The application was split into a reusable Rust library and a desktop launcher.
On desktop the terminal surface owns an actual PTY. On `wasm32` the same egui
chrome instead renders an interactive, deterministic command sandbox. This is a
deliberate product boundary: the website can offer a convincing trial without
claiming or attempting access to a visitor's machine. A release-mode WASM build
now succeeds; the raw module is 4.4 MiB before `wasm-opt`/gzip or Brotli.

## 2026-08-14 — Pane layouts and lifecycle proof

The workspace now supports the three most useful reference layouts: one pane,
two side-by-side panes, and two stacked panes. Switching into a split creates a
second real shell when necessary. Each pane remains independently interactive,
resizes its own PTY grid, and visibly marks its backing tab. Clicking a hidden
tab swaps it into the focused pane. Closing either side normalizes indices and
collapses back to one pane when only one session remains.

The manual smoke test exercised both split directions with distinct commands in
each shell, then closed one tab and confirmed its bash child disappeared while
the surviving shell stayed usable. Finally the window was closed through the
desktop window manager (not by killing the test harness): the application and
remaining child shell both exited within the polling window.

## 2026-08-14 — Release audit

The release audit rebuilt every target from the current main branch, reran
formatting, tests, native and WASM clippy with warnings denied, generated the
browser package, and linked the optimized Linux executable. The final stripped
binary is 13.36 MB. It opened an X11 window in 572 ms on this software-rendered
VM, then shut down its application and Bash PIDs cleanly through Alt+F4.

Browser-driven local testing also influenced the shipped page: the connected VM
browser has WebGL disabled, which originally left a blank canvas after eframe
reported the missing capability. The host page now catches that startup error
and presents a styled compatibility explanation. The canvas path remains marked
unverified until it is exercised on a WebGL-enabled browser.

## 2026-08-14 — Font and theme parity pass

The reduced four-theme/one-font prototype was replaced with the legacy visual
asset system. The repository now carries the original 127 theme documents and
26 font binaries with their shipped notices. A migration tool evaluated the
legacy TypeScript catalog and froze its 428 generated presets, bringing the
native searchable browser to all 555 original choices. Theme application now
updates distinct shell surfaces, the terminal foreground/background, every
normal and bright ANSI color, and bundled typography without restarting or
clearing a shell.

Settings now has dedicated Themes, Fonts, and Workspace pages. Fonts are live
and independently persisted for the same seven zones as the legacy app, with
real packaged weights, file provenance, sample rendering, and whole-app sync
actions. Manual X11 inspection confirmed that the 27 MB font pack initializes
without a rasterizer failure and that `basic2` changes the native chrome and
terminal metrics on first launch. That close test also exposed and fixed a
vendored event-forwarder panic when the app event channel disappears during
shutdown.

## 2026-08-14 — Native gradient and CRT effects

The terminal renderer now accepts a four-corner gradient mesh instead of
flattening every legacy theme to one background color. Default terminal cells
leave that mesh visible while applications that deliberately paint ANSI
background cells remain correct. Animated themes drift their gradient colors;
static and scanline values are normalized from the legacy percentage scale and
painted as bounded overlays. A screenshot check of `basic2` confirmed its dark
red/black terminal gradient behind live Bash output.

## 2026-08-14 — Scoped and calm theme application

Theme cards now expose the legacy five-part apply scope: app chrome, terminal
colors, fonts, gradients, and special effects. Each part keeps its own persisted
source theme, so mixing a favorite terminal palette with another theme's chrome
is real state rather than a temporary preview. Calm apply preserves the selected
colors while suppressing motion, static, and scanlines. Migration and partial
apply behavior have focused unit coverage.

## 2026-08-14 — Real terminal bold typography

Terminal themes no longer discard `fontWeightBold` or
`drawBoldTextInBrightColors`. The renderer receives distinct regular and bold
font faces, selects them per Alacritty cell flag, and promotes normal ANSI colors
to the theme's exact bright values when requested. Settings shows both the
requested bold weight and the actual nearest bundled face when a family lacks
that weight.

## 2026-08-14 — Editable command presets

The hard-coded native buttons were replaced with one persisted preset library
shared by the top bar, resizable command dock, and a dedicated Commands settings
page. Presets can be added, edited, deleted, or restored to the same
platform-aware starter set as the legacy app. Each button also preserves the
important `sendEnter` distinction: repetitive commands can run immediately,
while templates such as SSH addresses can be typed into the focused terminal
for review without being submitted.

The migration path gives existing native preferences the starter collection,
and focused tests cover migration, validation, CRUD state, and exact terminal
payloads. A living `PARITY_BACKLOG.md` now tracks the rest of the reconstruction
with source references and acceptance criteria rather than relying on a short
limitations summary.

## 2026-08-14 — Complete tab lifecycle

Native tabs now expose per-tab actions for rename, move left/right, and close;
double-click also opens rename. Manual titles are protected from later shell
title events. Closing a tab terminates its PTY and keeps bounded recovery
metadata, so reopening creates a clean shell and restores the user-assigned
name without implying that a terminated process resumed.

Moving a tab remaps focused, primary-pane, and secondary-pane indices as one
operation. Forward and backward remapping have focused tests, and an X11 pass
created two Bash sessions, renamed and reordered one, closed and reopened it,
then confirmed normal child cleanup on window close.

## 2026-08-14 — Ten live terminal panes

The primary/secondary special case was replaced by an ordered visible-pane
model capped at ten sessions. Status controls now add or remove panes and switch
between column, row, and balanced-grid geometry. Selecting a hidden tab replaces
the focused pane, while close and reorder remap every affected index without
changing which terminal occupies the other cells.

Grid sizing and close-state normalization have focused tests. The X11 pass tiled
ten real Bash PTYs in a 4-by-3 grid, wrote distinct output into separate panes,
confirmed ten child processes, and then verified that window close removed all
of them. Nested and individually resizable split trees remain tracked rather
than being hidden by this milestone.

## 2026-08-14 — Detected and custom shell profiles

Terminal launch is no longer tied to one implicit environment shell. The native
app discovers available shell executables, identifies each choice by its full
path, and offers automatic, detected, and user-defined profiles both as a
persisted workspace default and from the new-tab menu. Custom profiles accept a
quoted command line plus an optional working-directory override; the workspace
default directory is independently configurable. Launches execute the selected
binary directly instead of interpolating user text through another shell.

Closed-tab recovery remembers the selected profile while still starting a clean
PTY. Invalid commands, unmatched quotes, missing executables, and missing
directories surface as launch errors instead of silently falling back. Focused
tests cover parsing, discovery, migration, persistence state, and error paths.
The X11 smoke pass ran Bash and Dash simultaneously, confirmed both login-shell
processes, checked the responsive profile editor, and verified clean shutdown.

## 2026-08-14 — Separate SSH preset collection

The left dock now matches the legacy data model instead of mirroring the general
command buttons. It owns an independently persisted SSH collection with its own
empty state, add/edit/delete actions, type-versus-run choice, and focused-pane
delivery. The top bar remains the general command collection. Commands settings
switches between the two libraries without mixing their mutations.

Automated coverage proves old preferences migrate with an empty SSH library and
that editing or deleting an SSH entry cannot modify command presets. The X11
pass created a type-only entry, inserted it at a live Bash cursor without
submitting, restarted the app, confirmed persistence, and verified clean parent
and child shutdown.

## 2026-08-14 — Nested, persistent pane dividers

Pane rendering now builds a recursive tree for column, row, and balanced-grid
topologies instead of calculating one fixed rectangle size for every terminal.
Each branch owns a draggable divider and a topology-specific persisted ratio;
resetting from the status bar returns every branch to its balanced default. The
divider gap is kept outside terminal widgets so resizing does not begin a text
selection.

Structural tests prove every visible terminal appears exactly once in trees up
to ten leaves, while preference round-trip coverage protects saved ratios. In
the X11 smoke pass, moving only the top row divider of a four-PTY grid left the
bottom row unchanged. The asymmetric layout returned after a normal restart,
and both shutdown passes removed all four Bash children.

## 2026-08-14 — Legacy gradient geometry

Terminal gradients now preserve their source geometry. Linear themes honor the
stored angle, radial themes render concentric multi-stop rings around the named
position, and conic themes render a smooth angular fan with the configured
rotation. These meshes remain behind default terminal cells, so applications
that deliberately paint an ANSI background still win.

Focused coverage identifies representative radial and conic themes from the
embedded legacy catalog. The X11 pass applied `Prismatic Stage`, confirmed its
centered 90-degree conic sweep in a real Bash terminal, and verified clean child
shutdown. Repeating geometry and user-facing gradient editing remain explicit
items in the visual backlog.
