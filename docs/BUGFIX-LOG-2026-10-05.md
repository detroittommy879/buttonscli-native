# Bugfix log — 2026-10-05

- Started on `codex/pane-fonts-theme-editor` at `c841b2b`. Existing untracked
  `.aicp/` and `assets/themes/abyssal-bloom.json` belong to the user; preserve them.
- Read the current handoff, prior Settings/clipboard notes and pinned source.
- Confirmed detached Settings is included in `modal_open`, disabling terminal
  input and menus. Clipboard support is compiled in; previous checks tested
  generated copy commands rather than the OS clipboard.
- Theme catalog embeds `assets/themes/` at build time. Personal themes are loaded
  from the active native profile. Catalog filtering can hide most themes; a bad
  personal file is skipped individually. Exact transient three-theme cause is
  not reproduced yet.
- Wrote `SETTINGS-UX-PLAN.md` before larger changes. Plan: narrow fixes, a separate
  subtab experiment, generated concept images, and local commits by work area.
- Native Computer Use inventory currently finds no ButtonsCLI window. Use an
  isolated, app-owned test fixture for runtime checks; preserve normal profiles.

- Fixed detached Settings input blocking; shortcut recording is scoped to the
  Settings viewport. The selected Settings button reveals its existing window.
- Native clipboard probe exposed a second real bug: nested status-bar wrapping
  consumed all terminal height at 1000-point width. Flattened wrapping and kept
  the favorites group compact. The real terminal now retains its height.
- Right-click and automatic selection copy now pass a native eframe/Windows OS
  clipboard probe; a main-window click and text input work with Settings open.
  The fixture restores the prior text clipboard and cleans up its own shell.
- Added selection edge scrolling (20 ticks/sec, bounded speed), stationary-wheel
  selection updates and release cleanup. The explicit three-ConPTY check passes
  in both scroll directions, including copy on release outside the pane.
- Added Settings padding and contrasting 2-point checkmarks; scrollbar contrast
  uses foreground color, avoiding the previous checkbox-background override.
- Theme browser shows filtered/total counts, Show all, active theme folder,
  Reload personal themes and malformed-file warnings. Updated catalog tests to
  accept extra assets while preserving all 127 original themes and 428 code
  themes. Abyssal Bloom parses and is included by a rebuild; its legacy GLSL
  shader remains inert, as the app already documents.
- Validation so far: 251 library tests and two fixtures pass; 12 optional tests
  ignored by default. Native clipboard/Settings and three-ConPTY interaction
  probes pass explicitly. Existing incremental-cache/PDB warnings are nonfatal.
- The exact transient three-theme view and hours-long state are not reproduced.
- Separate layout experiment: Library / Edit / Generate subtabs use one ordinary
  page scrollbar and retain independent positions. Removed the stacked resize
  rules; old section-height profile data remains compatible. New from current
  theme opens a draft without applying another theme; renamed Save Variant to
  New variant to distinguish creating a draft from writing a file.
- Native captures found import/export fields pushed their buttons outside the
  window; reserved action width. Contrast changes initially darkened bold labels
  because egui shares the active foreground; corrected foreground/fill contrast.
  Final Edit capture shows padding, readable checks and the entire import action.
- App screenshot events capture the root viewport; immediate child Settings
  captures were unavailable through that path. Used read-only Computer Use for
  actual Settings captures. A linker retry succeeded after the owned test window
  exited; do not rebuild a running test executable.
- Final checks: 251 library tests pass, 12 optional tests ignored by default;
  two import fixtures and explicit native clipboard/Settings and three-ConPTY
  probes pass. Strict native and WASM Clippy and optimized Windows build pass.
  These checks do not certify a multi-hour soak or macOS/Linux runtime behavior.
- Generated two dark Settings concepts with built-in ImageGen, saved exact prompts
  and both images under `docs/design/settings-concepts-2026-10-05/`. Larger
  appearance-preset/default-slot/provider changes remain proposals in the plan.
