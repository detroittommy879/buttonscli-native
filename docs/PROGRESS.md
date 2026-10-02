# Reconstruction journal

## 2026-10-02 — GitHub README and integration follow-through

Rebuilt the README around the usable terminal workspace: centered title/badges,
feature table, source-build quick start, four user-supplied October screenshots,
and explicit AI Help/CLI/MCP access and remaining-work sections. Originals are
unchanged; the import screenshot with a private provider endpoint was excluded.
Public documentation images are now distinguished from diagnostic captures in
CONTRIBUTING.md. The first README patch failed because delete/add targeted the
same file; writing the complete UTF-8 document succeeded.

PR #1's Linux CI fails on five platform-specific warnings in `terminal.rs`.
Scoped Windows-only helpers/imports to Windows, retained WSL decoding under
tests on other hosts, and made the shell collection mutable only where it is
extended. Local Rust 1.96 also caught a boolean simplification in theme metadata;
the equivalent `is_none_or` form passes strict native Clippy. No CI gate was
relaxed. Incremental-cache finalization reported Windows access errors on the
first check; a process-local `CARGO_INCREMENTAL=0` avoids that cache issue.
Five CLI deadline tests and the 14-tool MCP fake-API contract pass. Full Windows
tests pass 235 library tests plus two fixtures, with five optional tests ignored;
formatting, strict native Clippy and the desktop build pass. The isolated live
CLI/startup matrix with `-WithMcpSdk` passes, including official-client discovery
of 14 tools and live status/tabs calls, output targeting, paced cancellation and
child-shell cleanup. Shutdown emitted existing ended-pipe diagnostics; cleanup
assertions passed. No personal terminals or settings were used.
README local links resolve, copied images match their originals, and the public
GitHub render was visually checked. GitHub CI is rerunning before integration.

## 2026-10-01 — Theme favorites, pane controls and migration gate repairs

Free `themeFavorites`, `terminalContextMenu` and `paneHoverLabel` features are
registered centrally. Stars on theme cards save profile-local IDs; a dedicated
section near the top of Themes and the status menu provide quick access.
The status random action shares the existing random-selection implementation.
Pane menus bind actions to session IDs, including after reorder or close.
They expose favorites/random/global, copy, select-all, clear, rename,
auto-tile inclusion and close. Hover labels paint the tab name without adding
a hit target; Workspace saves independent font/size/weight and opacity.

The native and WASM strict Clippy gates now pass without new lint suppressions.
Repairs group related arguments, remove unnecessary closure dropping, relocate
helpers before tests, simplify parser/option operations, replace a post-1.85
Windows API usage, and omit desktop-only code from the browser target.
Full Windows tests pass 235 library tests and two import fixtures; five optional
tests are ignored in the default run. The explicit auto-tile lifecycle test
also verifies favorite/random/global targeting, closed-target rejection, hover
visibility/disable, and unchanged session IDs. Pointer tests cover menu actions.
Native and optimized WASM builds pass, with the existing native PDB naming
warning. Five Node CLI deadline regressions and the MCP contract pass.

Windows ConPTY creation now assigns each shell atomically to its own
kill-on-close job. Closing a pane ends its shell and descendants, including
when queued `exit` has not been processed. Child-watcher callbacks retain a
valid context until synchronous unregister completes; duplicated process
handles, initial-thread handles and redundant pipe ends now close explicitly.
Failed watcher setup closes ConPTY while its reader can still drain output.
The explicit rapid-close test passes eight immediately dropped CMD shells and
a busy PowerShell shell with a CMD descendant. This fixes the prior orphan-shell
finding rather than hiding it behind a finite fixture.

The updated live Windows control/startup matrix passes after these changes.
A seeded isolated startup capture shows readable Favorites/Random controls at
1296×859 without the language chooser covering the workspace. A separate
abrupt-exit smoke kills only the host and verifies its held shell process
handle becomes signaled. This checks OS job cleanup independently of queued
graceful `exit` input or a recursive process kill in the harness.

References: Microsoft's [atomic job assignment example](https://devblogs.microsoft.com/oldnewthing/20230209-00/?p=107812/)
and [ConPTY lifetime contract](https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session).
Broad hands-on focus/clipboard/DPI, real-provider/hosted-entitlement acceptance,
Linux/macOS checks, production signing roots and activation/rollback integration
remain open. Optional Shader Lab/HSync still need the separately documented
offscreen-renderer work; no shader parity is claimed.

## 2026-10-01 — Auto-tile membership, context menus and AI Help discovery

Tabs now offer right-click menus with session-local **Include in auto-tile**
membership. Stable IDs preserve that choice across moves; reopening a closed
tab restores its membership. Excluded sessions remain alive and can be selected
alone. COL/ROW/GRID preserve the requested pane count across single view and
orientation changes without spawning replacements for excluded tabs. All
gathers existing included sessions up to the ten-pane limit.

Command and SSH presets share their action content between right-click menus
and optional dots. The persisted Workspace toggle defaults to hiding dots.
AI Help has a status-bar entry and an enabled Help-menu entry; the window can
show its access explanation/provider-settings link while requests retain the
existing Pro/development gates. New labels use the native literal catalog seam
with English fallback where the source catalog has no entry.

Per user direction, the old metrics-hosted runtime-config fetch and feedback
sender are disabled in production entry points. Feedback is hidden. Mock
adapter tests retain the old contract without contacting its host. The separate
auth and provider paths remain available. No new metrics sender or container
is introduced. `docs/migration/WORKSPACE-FOLLOWUPS.md` records later isolated
experiments for movable chrome and detached terminals, plus the new service.

Windows full tests pass 232 library tests plus two import fixtures, with four
optional tests ignored. The new ignored ConPTY lifecycle test passes when run
explicitly: four test-owned shells, remembered counts, stable-ID exclusion,
reordering, solo selection, orientation changes, close/reopen and re-inclusion.
Headless egui pointer tests cover tab/preset context menus, optional dots and
the AI Help status button. Desktop build, formatting and diff checks pass;
five CLI deadline regressions and the 14-tool MCP contract pass. Strict native
and WASM Clippy still fail on the existing 18/52 findings; gates stay intact.
The startup screenshot shows the new controls with the first-run language
chooser still open. Broader DPI/focus, provider/Vibe and cross-platform GUI
acceptance remain open.

The revised live Windows CLI/startup matrix passes, including child-shell
cleanup. Its prompt-aware readiness and short markers avoid sending into a
starting shell and ConPTY-split marker failures. Cleanup now polls exit for up
to five seconds instead of assuming shutdown completes within 300 ms. The
rapid-close CMD unit-fixture finding and bounded replacement are recorded in
`docs/VERIFICATION.md` for a separate lifecycle follow-up.

## 2026-10-01 — Startup and CLI follow-through

Startup parsing now recognizes help/version only in flag positions, preserving
those strings when they are command or directory values. Duplicate singleton
options are rejected, and command validation reports its actual 4096-byte UTF-8
limit. Four parser tests, executable help/version/error-exit checks, the desktop
build, and the isolated live Windows CLI/startup matrix pass. Startup remains
free and one-shot; `docs/STARTUP.md` documents its shell-settling limits.

Five real loopback HTTP regressions exercise the shipped Node CLI's stalled
headers, stalled response body, overall polling deadline, uncertain write
without retry, and malformed-response redaction. They all pass, as does the
14-tool MCP fake-API contract. `docs/CI-FOLLOWUP.md` preserves the proposed Node
CI job: GitHub rejected the workflow edit because the logged-in token lacks
workflow permission. The unpublished commit excludes that optional workflow
edit so the implementation and tests can still be pushed normally.
The preserved Agent Mode prototype requires its own explicit debug-only
`BUTTONSCLI_NATIVE_DEV_AI_AGENT=1` opt-in in addition to AI Help; normal AI Help
development access keeps it hidden. Release and live Agent Mode acceptance are
not claimed.

The full Windows Rust suite passes 222 library tests and two compatibility
fixtures, with three optional library tests ignored. Native strict Clippy
still fails on existing warnings (18 with all test targets on this toolchain).
WASM Clippy without denial compiles with 52 warnings: 49 dead-code warnings,
two unused variables and one loop-style lint. Strict CI keeps those gates;
the optimized WASM build also passes (51 compiler warnings). The PR remains
draft. Linux CI exposed host-dependent parsing of Windows shell
paths in an existing installer test; Windows shell basename matching now
recognizes both separator styles on every host. `CONTRIBUTING.md` records the
branch, push, draft PR, review, and merge workflow.

The optional `-WithLoadProbe` Windows control smoke passes at 1, 4 and 10
terminals, verifying 200 Unicode output lines per terminal, isolated target
delivery, retained session IDs, increasing output sequence and clean child-shell
shutdown. The final runs took 6,716 / 25,112 / 31,655 ms, including command
delivery, shell processing, polling and deliberate sleeps; these are functional
VM measurements, not frame-rate or maximum-throughput benchmarks. The harness
waits for a PowerShell prompt because API readiness only identifies a live PTY.
Initial narrow-pane probes also exposed ConPTY wrapping inside long raw markers;
short markers now fit the physical pane width. The API intentionally preserves
that raw VT contract, and `docs/CONTROL-API.md` records the matching limitation.
Cleanup retries transient Windows directory handles and preserves the original
test error. The isolated four-pane screenshot at `target/load-review.png` shows
live Unicode output and wrapping tabs, with the first-run language chooser still
open; it does not certify onboarding or keyboard interaction. Broader contention,
frame-cost, focus and DPI acceptance remain open.

## 2026-10-01 — Remote backup checkpoint

Moved the local migration checkpoint and remaining worktree edits onto
`codex/native-migration` for a GitHub backup and draft pull request. The
existing checkpoint covers native effects, layout repairs, AI Help window
behavior and bounded CLI requests. Additional edits include one-shot startup
tabs/commands, compatible provider base URLs, and an unfinished Agent Mode
prototype. This checkpoint preserves that prototype; it does not change the
documented priority of core parity and review-first AI Help, or certify Agent
Mode for release. Startup/CLI live acceptance and documentation follow next.

## 2026-09-30 — Bounded CLI request deadlines

The native Node CLI previously had no deadline for ordinary API requests or
response bodies, allowing a stalled read to defeat `wait-for-text` and
`wait-for-quiet` timeouts. Calls now use an AbortSignal through body reading;
ordinary requests get 10 seconds, with extra allowance for the existing
30-second paced delivery and 60-second run-wait bounds. Poll requests and
intervals are capped by the remaining wait deadline. Writes are never retried
automatically, and their timeout message explains that delivery may have begun.
Malformed response JSON produces a concise error without echoing its body.
`node --check scripts/buttonsclictl.mjs` passes. Runtime timeout/PTY acceptance
has not been rerun for this change.

## 2026-09-30 — AI Help window follow-through

Isolated Windows GUI review with a loopback fake provider exercised streaming,
explicit terminal-context preview and transmission, and HTTP 503 failure/retry.
No public provider or real credential was used. This review found that Windows'
system light-mode update could select an unconfigured egui style after startup,
leaving menus and AI Help light and Settings headings unreadable against the
native palette. Native palette application now explicitly selects its configured
style, including detached Settings and AI Help. AI Help applies its own font
zone and has a scroll fallback for unusually large composer content.

Streaming now displays the answer portion of the response envelope rather than
flashing XML tags or action JSON. Suggested actions still become available only
after complete reply validation. Transcript space remains stable during streaming
so growing text cannot move the Cancel button under the pointer. Cancellation showed a cancelled status and Retry; Retry completed successfully.
Closing/reopening AI Help preserved that conversation. Updated native palette
screenshots are saved locally in `target/ai-help-review.png` and
`target/settings-review.png`. The library and a separately linked current
`target/debug/buttonscli-updated.exe` build successfully. This is partial M4
evidence; target switching and broader focus/DPI/platform checks remain open.

Local checkpoint commits use the explicit agent identity `Codex <codex@localhost>`
through per-command Git options. User and global Git settings are unchanged;
missing personal author configuration does not block migration work.

## 2026-09-30 — Windows effects and layout follow-through

Ported the original analog-static procedural shader to a native WGSL callback,
including screen blending, half-resolution sampling, intensity, density, drift,
brightness and opacity. The pipeline validates on this VM's graphics adapter.
Simple noise now uses physical display pixels, keeps animating at nonzero idle
amounts, avoids repeated uploads within a noise frame and owns its bounded
textures on the terminal tab so closing tabs releases them. Unsupported GPU
contexts retain a bounded grayscale fallback; this does not implement HSync or
Shader Lab post-processing.

Menu, preset and status bars size to their content. Settings has one bounded
scrolling body, distinct nested IDs, adaptive theme columns and footer buttons
anchored to the window's original bounds. Moved the dock's Compact checkbox to
its own row. A large-font minimum-window layout regression passed during this
work; subsequent screenshot review caught and fixed horizontal footer overflow.

`uvx appsnap` works on this host; the smoke script now falls back to it when no
standalone `appsnap.exe` is installed. A separately linked review executable
used an isolated native home, leaving the user's running app untouched.
`target/static-review.png` shows dense native analog static and readable 22 px
status text; `target/settings-review.png` shows 24 px Settings text and both
footer buttons. GPU pipeline validation and `cargo build --lib` passed. Wider
DPI/platform and 4/10-pane performance acceptance remain open.

Fresh Windows system-default launches now prefer installed PowerShell 7, then
bundled Windows PowerShell, so the PowerShell starter presets work. Explicit
cmd profiles keep their selected shell. User priorities remain core parity,
then reliable AI explanations/single commands, vibe themes and CLI hardening.

## 2026-09-30 — Fine-grain effects and status-bar sizing

Replaced the native simple-noise rectangle mesh with a bounded grayscale
texture sized from the legacy resolution setting. Analog static now paints a
dense animated noise texture instead of a few hundred isolated dots. Both
effects cap texture dimensions and pixel count to keep large panes bounded.
Changed the status panel from a fixed 31 px height to a minimum height so its
controls and custom status font can determine the required row size.

`cargo check --bin buttonscli` passes on Windows. `cargo build --bin
buttonscli` passed after the user closed the running app. The isolated GUI
smoke reached its screenshot step, but this VM has no `appsnap.exe`, so no
capture was saved. The computer-use inventory also exposed no native windows.
Live appearance and frame cost at 1/4/10 panes still need review.

Priority clarification from the user: complete core migration/parity first,
then modernize AI Help for dependable explanations and single-command
suggestions, improve vibe-coded theme generation, and harden the mostly
working `buttonsctl` flow. Agent Mode remains outside the planned migration.

## 2026-09-29 — Partial Windows AI Help GUI acceptance

Ran the debug native app with an isolated temporary home and the explicit
development AI Help override. First-run language setup completed; **Help → AI
Help** opened a separate viewport. A loopback fake provider was configured in
the isolated native profile; Settings discovered its one model, and AI Help
rendered a streamed reply plus an inert command suggestion targeted at `term1`.
The test terminal showed no marker before review; clicking **Insert + Enter**
produced the expected harmless echo. Two identical markers appeared during
pointer calibration, so the terminal action was exercised more than once. No
real provider, API key, or user profile was used.

This is partial M4 evidence, not full acceptance. The provider editor's Test
Connection was not validated: the first fake server returned SSE where that
button expects a non-streaming JSON reply, so it correctly reported an invalid
response. Context preview, cancellation, retry/failure, target switching,
main-window shutdown/reopen, and broader focus/DPI checks remain open. The
detached Settings screenshot also showed red egui duplicate ScrollArea/widget
ID diagnostics; the source of that overlay is not yet identified.

## 2026-09-29 — Windows tab-close-during-paste acceptance

Added `scripts/test-close-during-paste.ps1`, which starts a test-owned native
app with a disposable home, opens two PowerShell PTYs, starts a long paced send
to the active tab, and closes that tab with the app's `Ctrl+Shift+W` shortcut.
The live request returned a closed-target error, the receiving tab stayed
available, and its output did not contain the payload marker. The first
harness attempt left the first-run language dialog open, so the shortcut did
not reach the app; the script now completes first-run setup before exercising
the race and removes only its GUID-named temp root after shutdown. No app-code
fix was needed: stable-ID dispatch already fails closed when the target closes.
Contended throughput/frame measurements and cross-platform behavior remain
open.

## 2026-09-29 — P06 signed-manifest verifier core

Added `src/distribution.rs` with exact-byte Ed25519 strict signature checking,
canonical compact JSON validation, manifest/product/target/version/channel and
updater gates, bounded artifact length plus SHA-256 validation, and Windows
archive-path checks. Ten focused tests use a deterministic throwaway signing
key, reject manifest/signature/package tampering and traversal, and prove the
compiled verifier fails closed with its empty trust list. Formatting, focused
tests, and `cargo check --bin buttonscli` passed. `cargo clippy --lib --
-D warnings` still fails on existing warnings in unrelated app, assistant,
control, fonts, theme, and terminal code; it reported none in the new module.
At this stage there was no production key, package builder, extraction/staging,
rollback, or updater integration, so P06 remained partial.

## 2026-09-29 — P06 archive preflight and versioned staging core

Pinned `zip` 7.2.0 with the `deflate-flate2-zlib-rs` backend; its declared Rust
1.83 MSRV fits the project's 1.85 minimum, and the optional Zopfli compressor
is excluded. Signed package verification now parses the real ZIP, caps the entry
count and expanded sizes, checks every stream/CRC before extraction, and rejects
traversal, links, special files, Windows-invalid names, duplicates and
file/directory collisions. The Windows target also requires a root-level
`buttonscli.exe`; versioned stage paths are bounded, and staging roots reject
symlinks/reparse points. A verified package can be extracted into a fresh
version directory; it refuses overwrite and leaves the active-version record
untouched. Fourteen focused tests pass. The production trust list remains empty;
package creation, active-version switching, startup rollback and the launcher
are still open. `cargo audit` is not installed in this checkout.

## 2026-09-29 — P06 Windows local package builder

Added an opt-in `native-release` command that packages a prepared directory,
validates the resulting Windows ZIP, and writes the artifact, canonical
manifest, and detached signature into a new output directory. It requires an
external 32-byte seed file, rejects that key inside the package tree, refuses
links/reparse points and unsafe paths, and prints the derived public key for
manual trust-root review. Two throwaway-key tests verify the generated package,
missing-executable rejection, key-location guard, and output collision. No
production key was created or installed. P06 remains incomplete pending the
production trust root, activation/rollback launcher, update UI, and release
endpoint. The release tool is internal developer tooling and adds no customer
entitlement tier.

## 2026-09-29 — A02 loopback provider transport acceptance

Added a real local HTTP-server test for the app-owned Reqwest transport. It
exercises the provider connection-check POST with a test-only bearer key, model
discovery on `/v1/models`, redirect refusal, and fragmented server-sent-event
streaming. The test uses loopback only; no external provider or real credential
is contacted.
`cargo test --lib assistant::client::tests::reqwest_transport_completes_loopback_provider_check_discovery_and_streaming`
passes. AI Help GUI/provider setup interaction and real-provider use remain
unverified. Access tier remains Pro (`aiHelp`).

## 2026-09-29 — MCP control contract coverage

Expanded `scripts/test-mcp-smoke.mjs` from initialize/list/status into a
test-owned fake API contract pass for all 14 advertised MCP tools. It checks
route and body mapping, selector escaping, bearer authentication, bracketed
delivery, a server-side feature denial, invalid payload handling, descriptor
path selection, and a stopped-server error. Node syntax checks and the smoke
pass succeeded. This remains protocol evidence only: no live ButtonsCLI API,
PTY, external MCP client, or GUI handoff was exercised. C05's fake-API test
criterion is complete; its live M3 workflow is still open.

## 2026-09-29 — Official MCP client interoperability

Added `scripts/test-mcp-sdk.mjs`, an optional integration smoke that installs
the official TypeScript MCP client SDK into a temporary directory, launches the
native MCP helper over stdio, lists all 14 tools, and calls status/tabs against
a fake authenticated API. It passed with SDK 2.2.0; no runtime or repository
dependency was added. `scripts/test-control-live.ps1 -SkipBuild -WithMcpSdk`
also passed against its isolated live ButtonsCLI app with the exact published
descriptor. This closes external-client protocol acceptance; direct GUI M3
interaction remains open.

## 2026-09-29 — Windows live PTY observer smoke

Added `scripts/test-control-live.ps1`. It builds or launches the debug app with
a disposable home and development-only control access, authenticates with that
instance's descriptor, creates two PowerShell PTYs, and confirms unique output
markers through run/read while one PTY is visible and then backgrounded. On
shutdown the test verifies the test-owned shell child processes exit and
removes only its GUID-named temp root. The installed Node helper was exercised
against that app for status, tabs, create, rename, read, both waits, run, send
from base64/file/stdin and slow-typed modes, key, type-only preset, and a
two-pane grid. The full
build-and-run and `-SkipBuild` run passed. WGPU emitted validation-layer and
registry warnings, but both app/test runs completed successfully. This verifies
the live Windows CLI matrix, visible/background capture, and a slow-typed send
stopping with a clear error after its target PTY exits. Closing a still-running
tab during paste, external MCP-client launch, direct GUI focus/typing, and
Linux/macOS runtime checks remain open.

## 2026-09-29 — R02 output transcript fixture and microbenchmark

Added `tests/fixtures/output-transcript.json` for raw ANSI/cursor controls,
blank lines, carriage-return redraw, alternate-screen markers, and split
UTF-8 chunks. Focused tests confirm chunk-wise replacement behavior,
repeated-output activity, and the Unicode-safe 200,000-character tail. An
ignored release probe ran five 32 MiB trials with 4 KiB chunks on Windows 11
Pro/i5-12600K/Rust 1.96: throughput median 686.29 MiB/s, per-write p95 5–7
μs, worst observed write 125 μs, and saturated-tail read median 179 μs. These
times include the whole capture/read path; no allocation count, isolated mutex
timing, contention, or live PTY was measured. Visible/hidden PTY behavior and
close/cancel races remain open; see `docs/migration/OUTPUT-CAPTURE-DECISION.md`.
`cargo fmt --all -- --check` passed. The full `cargo test --release` suite
passed 191 library tests and 2 synthetic fixture tests; the manual benchmark
remains ignored in ordinary test runs.

## 2026-09-29 — P06 native distribution design

Recorded the native product/artifact IDs, existing `.buttonscli-native`
settings boundary, exact-byte Ed25519 manifest signature, manual Windows ZIP
first package, versioned staging, and last-known-good rollback behavior in
`docs/migration/NATIVE-DISTRIBUTION-DECISION.md`. Native release files, keys,
endpoint, and pointer stay separate from Tauri. No production key, package,
updater, or release endpoint was created. P06 remains open pending verifier,
tamper/rollback tests, and platform adapters.

## 2026-09-29 — P04 explicit feedback and privacy

Added the free `userFeedback` feature under **Help → Send Feedback**. The
dialog shows a best-effort redacted message preview and sends only after an
explicit click. Each request uses fresh, non-persistent random IDs and includes
the message, category, optional reply address, app version, OS, and locale. It
never attaches terminal output, commands, clipboard data, files, or
diagnostics. Failed HTTP responses preserve the draft and never report success.
Native currently has no routine product analytics sender or queue; see
`docs/PRIVACY-AND-FEEDBACK.md` and `docs/migration/FEEDBACK-CONTRACT.md`.

Validation passed on Windows: `cargo fmt --all -- --check`, all 188 library
tests plus 2 fixture tests, `cargo check --bin buttonscli`, `cargo build --bin
buttonscli`, and isolated `scripts/native-smoke.ps1` startup at 1471×975. Mock
tests cover the source-snapshotted feedback contract and 400/429/500 failures;
no production request was made. The smoke proves startup only. Clicking the
feedback menu/dialog, live receiver behavior, and Linux/macOS acceptance remain
unverified. The optional Vulkan validation-layer warning was nonfatal.

## 2026-09-29 — P03 read-only guide tabs

Added a free Help → Read-only guides window with bundled Quick Start and AI
Help/provider setup tabs. A third tab can request one fixed ButtonsCLI `/dsp/`
Markdown file after an explicit click. The request runs in a worker, requires
HTTPS on the allowlisted host/path, rejects redirects, times out, validates
text/plain or text/markdown, and caps content at 256 KiB and 8,192 rendered
lines. Markdown is rendered as text; HTML and embedded links remain inert. A
separate fixed ButtonsCLI browser button is the only external-open action.
Fetch errors leave both bundled guides available offline. The guide feature is
free (`readOnlyGuides`) and has no PTY/session action path.

Windows validation passed: `cargo fmt --all -- --check`, all 178 library tests,
2 synthetic fixture tests, `cargo check --bin buttonscli`, `cargo build --bin
buttonscli`, and the isolated `scripts/native-smoke.ps1` startup pass with a
1471×975 window. The smoke only verifies app startup; live guide-menu/tab
interaction and the hosted request remain unverified. WGPU's missing optional
Vulkan validation layer warning was nonfatal. Linux/macOS checks remain open.

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
acceptance remain separate follow-up checks. A follow-up attempt to automate
shortcuts from this tool session could not focus the GUI; posted window
messages did not change the tab UI and were discarded. Do not treat the startup
screenshot as keyboard, pointer, or PTY interaction evidence.

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
