# Bugfix log — 2026-10-07

- Started at `6c31314` on `codex/pane-fonts-theme-editor`; fetched origin.
  Branch is 13 commits ahead of its remote; no open PR. `.aicp/` is untracked.
- Scope: predictable tab replacement, numbered pane/tab indicators, upper-right
  hover labels with a rounded backing and idle fade; concise agent/cargo notes.
- Preserve native profiles, vendored terminal fixes and scoped theme sources.
  Use test-owned shells/profiles; leave merging for the user's testing.
- Baseline five-ConPTY regression reproduced pane reordering when count changes.
  Ordinary skipped-tab clicks passed; size-limited requested groups also needed
  an explicit focused-slot swap rather than shifting the whole visible window.
- Preserve group order when changing counts and keep focus when shrinking.
  Number actual displayed slots, highlight matching tab/pane, and move hover
  labels to the upper right with an 80% backing, 3-second delay and 1.5-second fade.
- Fixed probe passes skipped clicks, keyboard routing, count changes, size-hidden
  swaps and upper-right label placement. Fade uses fake time and stops repainting.
- Added `AGENTS.md`; `.aicp/` is ignored and was never tracked.
- Tab activation returns keyboard control after F6; tab moves remap displayed
  indices too. Native screenshots verify the upper-right badge and its disappearance.
  Updated an older hover probe to use the preserved first pane, not assume tab zero.
- Cargo investigation: `cargo package` fails on path-only `egui_term`; preserve both
  patched forks. crates.io name lookup currently returns 404. Declared Rust 1.85
  is below locked dependency requirements (AES needs 1.89); recommend current stable
  and verify an honest minimum before publishing. No registry upload performed.
- Final native suite: 253 tests pass, 15 optional probes skipped; both import
  fixtures pass. Explicit five-terminal, membership/lifecycle, navigation-key,
  three-pane selection/scroll/input and native hover/idle probes pass.
  Native/WASM strict Clippy and formatting pass. Existing cache/PDB warnings persist.
- Optimized local and public-branch Git cargo installations pass on Rust 1.96;
  installed executables report 0.1.0. Cargo moves the final executable into its
  install root; copied the verified Git binary back to `target/release/buttonscli.exe`
  for the existing startup probe, which passes with an isolated profile.
- Initial GitHub web check on Rust 1.97 rejects an older untyped `Stroke::new(2.0)`.
  Made stroke widths explicitly f32, including new indicators, without changing behavior.

## Button alignment and terminal inset follow-up

- User screenshots show descending tab/filter rows, a favorite name wrapping into
  a tall narrow column with its star below, and text against terminal borders.
- Started from pushed `5650b6f`; PR #2 remains open/draft; working tree was clean.
- Pinned egui wrapping rows center widgets against a growing row height; `push_id`
  inherits wrapping and does not keep the favorite name/star pair atomic.
- Reproduced theme control drift with a 24 pt font before changing layout. Shared
  top-aligned wrapping rows fix tabs, filters and other wrapped controls. Favorites
  allocate each complete pair, truncating long names with a full-name tooltip.
- At 220 pt width, earlier oversized controls expand egui's available width beyond
  the window. Constrain wrapping rows to the clip bounds; all four stars now fit.
- Inset terminal content 4 pt horizontally/2 pt vertically to clear pane borders.
  Selection probe now starts inside actual pane geometry rather than fixed margins.
- Old scrollbar-derived drag coordinates landed in the new gutter and failed the
  multiline assertion. Deriving every drag start from the pane rectangle fixes it.
- Final follow-up: 254 library tests, 2 import fixtures, strict native/WASM Clippy
  and formatting pass. Explicit large-font tab/input and three-pane selection,
  copy/scroll/IME probes pass. Native captures in `.private/button-layout-probe/`
  confirm level tabs/filters, readable first columns and aligned favorites.
  Optimized build and isolated native startup pass; existing shutdown pipe warning
  persists. Updated the existing draft PR; leave merging for user testing.
