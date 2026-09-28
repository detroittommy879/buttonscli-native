# Reconstruction journal

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
