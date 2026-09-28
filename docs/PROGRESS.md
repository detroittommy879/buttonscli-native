# Reconstruction journal

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
