# Reconstruction journal

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
