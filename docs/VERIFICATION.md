# Verification record

Last run: 2026-10-04 on Windows, Rust 1.96.0. Earlier Linux-only checks
are labeled by date in the sections below.

## 2026-10-04 SSH/htop terminal-capability regression

- Before the change, real local WSL htop reproduces a blank screen with only
  uncolored bottom menus when given the launcher's `TERM=dumb`. The same saved
  theme/typography shows full colored meters/process rows with xterm-256color.
  Passing the actual Windows child shell's TERM through to WSL reproduces it too.
- The new ignored ConPTY check passes from a dumb parent: its spawned shell has
  xterm-256color/truecolor, the parent remains dumb, and parsed standard ANSI,
  indexed 256-color and explicit RGB cells resolve to the expected colors.
- `cargo test --locked`: 247 library tests and two import fixtures pass, with 11
  optional tests ignored. Strict native Clippy, formatting/diff and PowerShell
  parsing checks pass. The optimized release rebuild passes. Inspected before/
  after captures of the identical Windows-child-to-WSL fixture: blank uncolored
  menus before, full colored CPU/memory meters, process rows and menu after.
- The fixture is local only; no remote SSH login, package install or server
  configuration change is involved. Owned htop sessions terminate on app close.

## 2026-10-04 local JSON access and release testing

- `cargo build --release --locked --bin buttonscli` builds the standalone Windows
  executable. Native and WASM strict Clippy, formatting, diff checks and
  PowerShell script parsing pass. Compiler incremental-cache access notes and
  the pre-existing bin/lib PDB collision warning do not fail these checks.
- `cargo test --release --locked --lib`: 247 pass, 10 optional tests ignored.
  New tests cover every catalog key, explicit false/missing/invalid JSON,
  unknown keys, oversized files and preservation of existing profile data.
- `cargo test --release --locked --lib ai_help_fixture_stream_retry_cancel_and_reviewed_target_delivery -- --ignored --test-threads=1`
  passes with a test-owned JSON flag, loopback provider and CMD sessions. No
  debug-only access override is required. The app gates for AI Help, Agent Mode,
  theme generation, Quick Secrets and remote control are available in optimized
  builds. Streaming, context, review/target routing, retry and cancel pass.
- `pwsh -NoProfile -File scripts/native-smoke.ps1 -SkipBuild -Release -WithLocalFeatureFlags`
  passes startup, visible window bounds, enabled control handoff and opening AI
  Help in a separate window. The probe removes development access flags and
  uses an isolated profile. Optional `-AiHelpCapturePath` captures the chat.
  Initial capture found Send clipping with the newly exposed Agent Mode row;
  the reserved composer height was increased. The final rebuild/capture was
  inspected: Agent Mode, context, composer and the full Send button are visible.
- The separate assistant viewport currently exposes a placeholder UIA tree, so
  this probe does not certify its composer accessibility. Main terminal/reader
  accessibility remains covered by the earlier dedicated checks.
- The user's normal native JSON flag was enabled independently of active-profile
  settings; existing presets/providers and profile JSON were preserved. Normal
  launch uses the standalone executable, with no temporary-home launcher.
  The final normal-profile launch and AI Help window-opening check pass; the app
  was left open for manual real-provider comparison. Its active Mistral model
  field is empty, so a model must be selected before sending. No real provider
  request or terminal action was triggered during these checks.

## 2026-10-03 terminal accessibility, history and AI Help fixture

- Full native suite: 245 library tests and two import fixtures pass; 10 optional
  library tests are ignored by default. Strict native/WASM checks, desktop and
  optimized WASM builds, formatting and diff checks pass.
- The explicit three-pane ConPTY check now also verifies the Find shortcut and
  text focus, live match highlights, absence of shell input from search/F6,
  independent history truncation, zero scrollback and three dated text files
  containing real test output. Selection/right-click copy and keyboard/scroll
  routing continue to pass.
- `cargo test ai_help_fixture_stream_retry_cancel_and_reviewed_target_delivery --lib -- --ignored --test-threads=1`
  passes against the reusable Node fixture with test-owned CMD sessions. It
  covers visible streaming, context off/on, inert suggestions, reviewed Insert
  and Insert + Enter, focus/reopen target retention, retry, cancel and closed
  target rejection. The debug launcher uses an isolated temporary profile.
- `pwsh -NoProfile -File scripts/native-smoke.ps1 -SkipBuild -WithAccessibility`
  passes Windows UI Automation discovery, menu Invoke, reader TextPattern and
  ValuePattern, read-only metadata and nonempty text selection. The startup
  capture was inspected. A headless AccessKit tree check also passes.
- Six Node CLI/fake-provider checks and the MCP contract pass. Real-provider
  GUI acceptance, NVDA/Narrator and Linux/macOS screen readers remain open.
  Existing PDB/graphics-driver and ended-pipe diagnostics remain; they did not
  fail the functional checks.

## 2026-10-02 pane interaction and compact theme browser

- Created checkpoint `0dd74df` before the requested changes on
  `codex/pane-fonts-theme-editor`.
- Full library suite: 242 pass, nine optional tests ignored; both legacy import
  fixtures pass. Strict native Clippy, formatting and desktop/browser compile
  checks pass. Five CLI deadline checks and the MCP stdio contract pass.
- Explicit Windows `three_panes_select_copy_scroll_and_route_keyboard_independently`
  passes with three finite, test-owned ConPTY sessions: selection on the first
  drag, independent selection state, direct right-click clipboard commands,
  wheel/scrollbar routing without changing other panes, keyboard input with the
  pointer outside the focused pane, and releasing a drag outside the window.
  Multiline copy preserves line breaks. Turning direct copy off restores the
  menu; hovering font controls does not expand them, and open pane menus block
  terminal keyboard input. All test shells are closed.
- The deeper-history fixture also exposed an invalid grid index in bounded
  terminal text snapshots. Tail reads now count backwards from the final screen
  line correctly; short reads stay bounded and larger reads include history.
- Pointer tests verify that hovering the favorites section does not open it,
  and status Favorites/Random apply every theme zone while clearing pane theme
  and font overrides, even with the editor's Apply scopes unchecked.
- Compact-browser UI tests render palette swatches and click a native theme;
  search/filter/sort tests cover name order, favorites, legacy separation and
  newer native versions. Native authoring versions survive export/parse.
- Isolated native startup and status-bar capture pass. The compact browser is
  covered by headless egui interaction tests; full manual visual acceptance
  remains open. Existing bin/lib PDB and graphics-driver notices remain.

## 2026-10-02 pane fonts and theme-editor follow-up

- Started `codex/pane-fonts-theme-editor` from merged `origin/main`; the new
  GitHub README was preserved.
- Full native suite: 239 library tests and two import fixtures pass; eight
  explicitly optional tests are ignored. Strict Clippy and desktop/browser
  compile checks pass. Final native color round-trip coverage includes separate
  shell/control, tab, dock/button, Settings and status-bar fields.
- Font regressions cover two themes with different families/sizes, terminal-only
  legacy font metadata, manual overrides and the Fonts apply scope. The explicit
  Windows ConPTY lifecycle check passes with font retention across reorder,
  random-theme reset, close and recovery, and stale popup targeting checks.
- Fake-time editor checks verify the 500 ms debounce, held-mouse suppression,
  all-section preview despite unchecked card scopes, animated-gradient state,
  disabling preview without losing draft edits, and excluding an unsaved preview
  from autosave. Existing minimum-window/footer and menu-pointer tests pass.
- Windows isolated GUI startup/capture passes. The captured 1296×859 frame was
  inspected: **★ Favorites** and **Random theme** are visible in the status bar.
  This does not replace interactive acceptance of every editor/scrollbar control.
- Five CLI deadline regressions and the MCP fake-API contract pass. The isolated
  live CLI/PTY matrix and official SDK smoke pass; the SDK lists 14 tools and calls
  status/tabs. The full real MCP tool matrix remains open.
- Provider 2's originally pasted line break caused an invalid header; trimming
  fixed transport construction. Both original/replacement credentials then got
  HTTP 429 on completion. Native `/v1/models` discovery succeeded with the bounded
  500-ID list. Further Provider 2 testing stopped at the user's request.
- The user-supplied Mistral endpoint and exactly `codestral-latest` passed a real
  completion, streamed AI Help response and generated palette schema/contrast
  validation. Prompts were synthetic and no terminal context or keys were logged.
  Mistral was saved as a separate active provider with its key in the OS vault.
  No generated theme was written by the test. Full AI GUI acceptance remains open.
- Existing bin/lib PDB naming and unavailable Vulkan-driver/validation-layer
  messages remain; startup/rendering and functional assertions pass using the
  available graphics backend.

## 2026-10-02 README and integration checks

- Formatting, strict native Clippy, desktop build and `git diff --check` pass.
- Full native suite: 235 library tests and two import fixtures pass; five
  optional library tests remain ignored. WSL decoding coverage still runs on
  non-Windows test hosts; Windows-only discovery helpers now compile only there.
- Five Node CLI deadline regressions and the 14-tool MCP fake-API contract pass.
- `pwsh -NoProfile -File scripts/test-control-live.ps1 -SkipBuild -WithMcpSdk`
  passes against isolated test state. The official client lists 14 tools and
  calls status/tabs on the live app; the installed CLI matrix checks targeted
  startup, input formats, layouts, visible/background output, paced cancellation
  and child-shell cleanup. No full live MCP tool matrix or GUI handoff is claimed.
- The desktop build retains the existing bin/lib PDB naming warning. Vulkan
  validation-layer warnings and ended-pipe messages appeared during the live
  smoke; functional and cleanup assertions pass.
- The README's local links resolve; its four public screenshots were inspected,
  copied without modification, and checked in the GitHub render. The private
  provider import capture was excluded. GitHub CI is pending at this checkpoint.

## 2026-10-01 favorites and migration checks

- Full native suite: 235 library tests and two import fixtures pass; five
  optional library tests are ignored in the default run.
- Explicit `auto_tile_session_lifecycle` passes: per-pane theme IDs remain
  correct across reorder, random avoids the current theme, global clears only
  that override, and closed targets cannot affect another pane. The actual
  workspace painter shows the hovered tab name, hides it when disabled, and
  preserves session IDs.
- Explicit `rapid_close_terminates` passes: eight CMD shells close immediately
  without sending `exit`, plus a busy PowerShell shell and its CMD descendant
  exit after dropping the pane. Test-owned handle guards prevent orphaned
  processes on assertion failures.
- Pointer/menu tests verify the pane actions and favorite ID selection;
  settings tests verify profile round-trip, deduplication and bounds.
- Strict native `cargo clippy --all-targets --locked -- -D warnings` and WASM
  `cargo clippy --target wasm32-unknown-unknown --no-default-features --locked
  -- -D warnings` pass. No new lint suppression or relaxed CI gate was added.
- Desktop and optimized WASM builds pass. The pre-existing bin/lib PDB filename
  warning remains; the browser compiler warnings have been eliminated.
- Five Node CLI deadline regressions and the 14-tool MCP contract pass.
- The live Windows control/startup matrix passes after the cleanup changes,
  including visible/background PTYs, paced-send cancellation and shell cleanup.
- `native-smoke.ps1 -SkipBuild -WithThemeControls -CapturePath
  target/favorites-startup.png` passes. The inspected 1296×859 capture shows
  readable Favorite themes and Random theme status buttons with no onboarding
  overlay or clipped controls.
- `native-smoke.ps1 -SkipBuild -WithThemeControls -WithAbruptExit` passes:
  killing only the test host terminates its tracked shell, without recursively
  killing children from the harness.

These supersede the older failing-lint and orphan-CMD records below. Manual
full-screen app mouse behavior, monitor/DPI, clipboard and cross-platform GUI
acceptance are still separate checks. `native-smoke.ps1 -WithThemeControls`
seeds only its isolated profile so the first-run chooser does not cover the
status controls during capture.

## 2026-10-01 workspace follow-up

- `cargo test --locked --quiet`: 232 library tests and two import fixtures
  pass; four optional tests ignored.
- `cargo test --locked --lib auto_tile_session_lifecycle -- --ignored
  --nocapture`: passes with four test-owned Windows ConPTY shells, count
  restoration, stable membership, reorder, solo view and close/reopen.
- Headless egui pointer tests pass for right-click tabs and both preset
  collections, optional dots, and the AI Help status button.
- Both old metrics production adapters return Disabled before constructing
  a client or sending feedback. Mock tests cover the retained contracts.
- Desktop build, formatting/diff checks, five Node CLI deadline regressions
  and the 14-tool MCP contract pass.
- Optimized WASM build passes with the existing 51 compiler warnings.
- Strict Clippy still fails on the existing 18 native and 52 WASM findings.
  No lint gate is weakened. The PR remains draft.
- The isolated desktop startup/capture smoke passes at 1296×859, showing
  AI Help, All and hidden dots. The language chooser remains open, so this
  capture is startup evidence rather than complete native interaction review.

The first live CLI rerun failed a long visible-output marker. Waiting for a
real PowerShell prompt and using short unique markers repaired the functional
matrix. A subsequent run completed those checks but declared child cleanup
failed after only 300 ms; those children exited shortly afterward. The script
now uses a bounded five-second exit poll. The final updated live CLI/startup
matrix passes, including visible/background output and child-shell cleanup.

The initial rapid-close unit fixture used unbounded CMD `/K` processes; four
survived queued `exit` input followed immediately by backend teardown. Only
those verified test-owned processes were stopped. The opt-in membership test
now uses finite PowerShell processes so failures cannot leave unbounded shells.
This test validates selection/session ownership, not graceful shutdown of every
shell. Rapid CMD close/teardown deserves a separate terminal-lifecycle follow-up.

The earlier checkpoint below records additional load/GUI limits; this feature
slice does not certify broad desktop/provider parity.

## 2026-10-01 startup, CLI and remote checkpoint

- `cargo fmt --all -- --check` and `git diff --check` pass.
- `cargo test --locked` passes 222 library tests and two legacy fixture tests;
  three optional library tests are ignored.
- `cargo build --bin buttonscli --locked` passes. Its help/version options exit
  successfully; an invalid tab count exits with code 2. The existing Windows
  bin/lib PDB output-name warning remains.
- `pwsh -NoProfile -File scripts/test-control-live.ps1 -SkipBuild` passes against
  an isolated profile: three startup tabs, one command each in tabs 1 and 3,
  no input in tab 2, the installed Node CLI matrix, visible/background output,
  paced-send cancellation on PTY exit, and child-shell cleanup. This does not
  certify keyboard, clipboard, monitor, DPI or broader visual interaction.
- `node --test scripts/test-cli.mjs` passes five loopback HTTP deadline/error
  regressions. `node scripts/test-mcp-smoke.mjs` passes all 14 tool contracts.
- Native `cargo clippy --all-targets --locked -- -D warnings` fails on existing
  strict lints. WASM Clippy compiles without denial but reports 52 warnings;
  the strict WebAssembly CI gate remains failing. No lint gate was weakened.
- `cargo build --target wasm32-unknown-unknown --release --no-default-features
  --locked` passes with the existing 51 compiler warnings. Browser canvas
  interaction was not rerun.
- The live control smoke with `-WithLoadProbe` passes for 1, 4 and 10 terminals,
  including 200 Unicode output lines per terminal, target isolation and child
  cleanup. Measured completion times were 6,716 / 25,112 / 31,655 ms including
  delivery, shell execution, polling and deliberate sleeps. This is functional
  contention evidence, not an FPS or maximum-throughput benchmark. Raw long
  markers can be split by ConPTY wrap/redraw VT; the test uses short markers.

The current migration is backed up on `codex/native-migration` with draft
[PR #1](https://github.com/detroittommy879/buttonscli-native/pull/1). The draft
preserves the unfinished Agent Mode prototype with a separate explicit debug
opt-in. Windows results do not certify Linux/macOS behavior or production
entitlement, signing and update services.

GitHub rejected a proposed Node CI job because the current token lacks workflow
permission. That optional workflow edit was removed from the unpublished
commit, preserving the existing CI configuration. The reviewed job is saved in
[`CI-FOLLOWUP.md`](CI-FOLLOWUP.md); local Node checks above still pass.

## 2026-09-29 P01 feature grant resolution

`cargo test --lib features::access::tests` passes four tests. New coverage
verifies that a non-expired server grant unlocks only its named Pro feature
and that expiry closes access. Kill switches, rollout state and development
override precedence remain covered. Native account login and credential
handling are now implemented and contract-tested; live hosted service
behavior remains unverified.

## 2026-09-29 P01 runtime-config client

`cargo test --lib account::tests` passes two tests for missing-field defaults,
accepted runtime flags, request URL, non-2xx and invalid JSON responses, and
remote cleartext/credentialed URL rejection. Runtime config is fetched on a
background thread every five minutes and cached only in process memory; any
failed refresh clears the flags to their closed defaults. The native smoke
harness sets `BUTTONSCLI_NATIVE_DISABLE_REMOTE_CONFIG=1` and
`BUTTONSCLI_NATIVE_DISABLE_ACCOUNT=1` to keep startup verification offline.
Account API mocks cover code start/verify, `/me`, entitlement resolve, logout,
online restore, expiry, 401, unknown plan/features, session-token redaction,
and rollback if keyring persistence is partial. These tests use an in-memory
credential store; Windows Credential Manager behavior and live hosted auth,
Turnstile configuration, runtime flags, and revocation remain unverified, so
no Pro feature is enabled.

The Windows pass also completed `cargo fmt --all -- --check`, `cargo test`
(159 library tests and 2 fixture tests), `cargo check --bin buttonscli`,
`cargo build --bin buttonscli`, and the isolated
`pwsh -NoProfile -File scripts/native-smoke.ps1 -SkipBuild` run. The GUI opened
at 1471x975 pixels; the smoke launch explicitly disabled account and runtime
requests. The
WASM library check passed with the existing 50 native-only warnings. WGPU
reported the optional Vulkan validation layer missing during GUI startup; it
was nonfatal. None of these checks verifies live service behavior or account
sign-in.

## 2026-09-29 P05 isolated Windows GUI startup

`pwsh -NoProfile -File scripts/native-smoke.ps1` built and opened the native
GUI using a unique temporary home/AppData root and working directory. The
test locates the app window by the child PID, verifies it is visible and at
least 500x350 pixels, then closes only that test-owned process. Optional
Appsnap capture passed and showed the full-size first-run language chooser on
the dark native UI. It did not exercise layout or PTY interaction. The
original app process and user profile were not used. WGPU logged the missing
optional Vulkan validation layer but continued successfully.

## 2026-09-29 V04 Shader Lab compatibility warning

`cargo test --lib` passes 150 tests, including checks that imported themes
retain enabled and disabled `shaderLabEnabled` values as inert metadata, and
that the warning has translations for all 21 supported locales. `cargo check
--bin buttonscli`, the WASM library check, `cargo build --bin buttonscli`, Rust
formatting, and `git diff --check` pass. The WASM build has the existing 50
native-only warnings; the Windows build has the existing bin/lib PDB filename
collision. The renderer decision is based on the original WebGL
composite/readback path, `TerminalView` painting through the egui painter, and
the pinned egui-wgpu callback contract. No native shader editor, compilation,
preview, GPU fallback, or visual acceptance is claimed.

## 2026-09-29 V03 Windows source checks

The native theme generator sends only a bounded user brief and selected theme
palette through the configured provider transport. Mock-transport tests cover
valid output, malformed palette rejection, unknown-field/effect preservation,
palette-only provider context, contrast checks, cancellation, and exactly one
correction request. `cargo test --lib` passes 148 tests; `cargo check --bin buttonscli`,
`cargo build --bin buttonscli`, formatting, and the WASM library check pass.
The WASM check reports 50 existing native-only warnings and the Windows build
reports the existing bin/lib PDB filename collision. Live provider requests,
candidate preview/save GUI flow, and Windows theme appearance remain
unverified. The release feature remains locked pending entitlement integration.

## 2026-09-29 V02 Windows source checks

Windows `cargo test` passes (142 library tests, 2 fixture tests),
`cargo check --bin buttonscli`, `cargo build --bin buttonscli`, and the WASM
library check pass. The WASM check reports 50 native-only dead-code warnings;
the binary build reports the existing bin/lib PDB filename collision. The
vendored renderer suite passed 16 tests in the preceding V02 row-banding check.
Row-banding configuration/master gates, focused-pane policy, RGBA conversion,
personal-theme export, and cell-height-aligned overlay geometry have focused
tests. Simple-noise tests cover idle timing, bounded/deterministic mesh output,
master-off, legacy parameter mapping, lock-free PTY activity timestamps, and
export. The HSync decision is based on the original CPU/GPU implementation and
the native renderer's direct epaint path; it does not claim runtime or GPU
acceptance. Live screenshots at 1/4/10 panes and frame-cost review remain open.

## 2026-09-29 V01 Windows checks

Repeating gradient sampling/mesh, legacy geometry projection, master-off
animation behavior, focused-pane filtering, and personal-theme export have
automated coverage. The existing terminal grid still paints ANSI cell
backgrounds over the gradient mesh. `cargo test` passed with 133 library and
2 fixture integration tests; the vendored renderer suite passed 14 tests.
Windows `cargo check --bin buttonscli`, `cargo build --bin buttonscli`, Rust
formatting, and `cargo check --lib --target wasm32-unknown-unknown
--no-default-features` passed. The WASM check reports existing native-only
dead-code warnings. The binary build reports the existing bin/lib PDB filename
collision. Live Windows theme editing, multi-pane visuals, and repaint/frame-
cost review remain unverified.

## 2026-09-28 Windows source/build check

`cargo fmt --all`, `cargo check --bin buttonscli`, `node --check
scripts/buttonsclictl.mjs`, and the helper's `--help` path passed after adding
AI Help and the native control API. This is source/build evidence only. Tests,
live Windows GUI/API/PTY interaction, provider calls, output throughput, and
Linux/macOS runtime validation were not run for those changes. The detailed
current boundary is in `docs/PROGRESS.md`, `docs/AI-HELP.md`, and
`docs/CONTROL-API.md`.

## 2026-09-28 U08 Windows fonts check

`cargo fmt --all -- --check`, `cargo test --lib` (120 passed), `cargo check
--bin buttonscli`, `cargo build --bin buttonscli`, and `git diff --check` passed
after implementing offline system-font discovery and profile-local font
imports. The discovery scan is bounded and import validates faces before
registration. These checks do not replace a live Windows font selector review
or startup-time measurement on machines with large font collections.

## 2026-09-28 U09/U10 Windows check

`cargo fmt --all -- --check`, `cargo test --lib` (128 passed),
`cargo check --bin buttonscli`, `cargo build --bin buttonscli`, and
`git diff --check` passed after adding Windows opacity and Cool Stuff. Tests
cover persisted/imported opacity bounds, source-matched installer selection,
shell selection, path quoting, profile-safe extraction, external links, and
typing without Enter. The build still reports the existing bin/lib PDB filename
collision. Live desktop, opacity, clipboard, and terminal interaction remain
unverified.

## Automated gates

The following completed successfully:

```sh
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo clippy --target wasm32-unknown-unknown --no-default-features -- -D warnings
cargo build --target wasm32-unknown-unknown --release --no-default-features
wasm-pack build --target web --out-dir web/pkg --no-default-features
cargo build --release
```

The native tests prove that all 127 theme documents parse, the combined legacy
catalog contains exactly 555 selections, representative ANSI values survive
verbatim, all 26 scalable font files and the emoji face have catalog entries,
legacy font aliases sanitize safely, and cross-platform shell-title extraction
remains stable. Font tests also validate offline local discovery/import,
collision handling, missing and invalid font behavior, regular/bold selection,
and Hangul fallback. The
shell-profile suite covers quoted command-line parsing, malformed input,
discovery de-duplication, old-preference migration, custom launch resolution,
missing executables/directories, and fallback after removing the selected
default. The test suite also covers one-ID preference migration, proves that
unchecked theme-apply sections remain unchanged, and preserves terminal
bold-weight plus bright-ANSI settings. Preset tests cover old-preference
migration, validated add/edit/delete state, collection isolation, and the exact difference between
immediate execution and type-only templates. Pane tests cover recursive leaf
preservation through ten terminals and persisted divider-ratio round trips.
Gradient tests verify representative legacy radial and conic geometry. The WASM
package contains generated
JavaScript/TypeScript bindings and a 33,498,448
byte uncompressed module before HTTP compression.

The stripped native release executable is 43,553,440 bytes with the complete
offline font and theme payload. On this software-rendered VM, a post-link launch
reached a discoverable X11 window in 554 ms. This is a coarse end-to-end
observation rather than a controlled benchmark.

## Manual desktop smoke test

The native application was launched with `cargo run` and exercised through the
real X11 window:

1. A Bash login shell presented a prompt and accepted typed commands.
2. `printf`, `uname`, `seq`, and `stty size` produced expected terminal output.
3. The window changed from 1280×820 to 940×650; terminal columns/rows and PTY
   content reflowed to the new bounds.
4. A 220-line sequence was generated and mouse-wheel input scrolled backward
   through preserved grid history.
5. A multiline mouse selection rendered visibly. Ctrl+Shift+C placed it on the
   system clipboard and Ctrl+Shift+V wrote it back into Bash.
6. Side-by-side mode created a second PTY; `LEFT_PANE` and `RIGHT_PANE` commands
   ran independently. Stacked mode preserved both grids and resized each PTY.
7. Closing one split tab removed its Bash child and collapsed to a valid single
   pane. Closing the native window through Alt+F4 stopped both the application
   and the remaining Bash child.

The final lifecycle check repeated step 7 against the optimized release binary:
the window, application PID, and Bash child PID all disappeared cleanly.

The visual-parity pass additionally opened the rebuilt Settings window on X11,
confirmed the 559-entry combined theme browser (555 legacy plus four native),
and rendered the full embedded font pack. Closing that development build through
the window manager exposed an event-forwarder shutdown panic; the adapter now
terminates quietly when the application channel closes, and the regression is
covered by the repeated close smoke test.

The preset pass launched the native window and confirmed that the six
platform-aware starter presets and per-button action affordances render in the
top dock without obscuring the terminal. Closing through the window manager
again stopped both the application and its login-shell child cleanly. Preset
editor mutations and type-only payload behavior are covered by the automated
tests above.

The tab-lifecycle pass used an isolated preference directory, created a second
real Bash session, renamed it `Work shell`, moved it left, closed it, and used
the visible recent-close control to reopen it as a fresh PTY with its custom
title restored. Unit tests cover forward and backward index remapping for every
affected slot. Closing the window stopped the app and both shell children.

The multi-pane pass selected balanced-grid mode, increased the live pane count
to four, and wrote distinct markers into separate Bash sessions. It then raised
the count to the supported maximum of ten, confirmed a 4-by-3 tiling and ten
direct Bash children, and closed the window normally. The app and all ten child
PIDs disappeared. Automated coverage checks grid dimensions through ten panes
and visible/hidden close normalization.

The shell-profile pass opened the per-tab profile menu, confirmed that detected
shells were labeled with their executable paths, and launched `/usr/bin/dash`
as a second real login-shell PTY alongside the default Bash session. Process
inspection showed `/bin/bash -l` and `/usr/bin/dash -l` as direct children. The
Workspace editor rendered its detected/custom/default controls at the minimum
supported window size without hiding the command fields or actions. Closing the
window normally removed the app and both shell children. Custom resolution,
working-directory validation, and persistence are additionally covered by the
automated tests above.

The SSH-preset pass started with the legacy-compatible empty SSH collection,
created a `Staging host` entry from the left dock, and clicked it into the
focused Bash prompt with Enter disabled. The command appeared at the cursor and
did not execute. After a normal close and relaunch against the same isolated
preference directory, the SSH entry and its type-only behavior were still
present. The app and Bash child both exited cleanly after each close.

The resizable-pane pass created a four-shell grid and confirmed four direct Bash
children. Dragging the top row's vertical divider changed only that nested
branch; the bottom row remained balanced and every PTY resized in place. After
closing normally, relaunching with the same isolated preferences, and returning
to four panes, the asymmetric top-row ratio was restored. Both close cycles
removed the app and all four child shells.

The gradient-geometry pass applied the bundled `Prismatic Stage` theme and
confirmed a centered conic sweep in the live terminal instead of the old
four-corner approximation. Its 90-degree source angle was retained, terminal
text remained readable, and normal window close removed the Bash child. The
same parser path is covered for the radial `neon_monster_mash` theme.

Screenshots from this run are recorded in `docs/images/` and the reconstruction
journal. The parity pass includes `native-theme-library.png` and
`native-font-settings.png`.

## Browser smoke test

The generated package loaded through an HTTP server and reached eframe startup.
The VM browser reported that WebGL was unavailable. The page then showed its
tested compatibility message instead of a blank canvas. Interactive canvas
testing remains explicitly unverified until run in a WebGL-enabled browser.
