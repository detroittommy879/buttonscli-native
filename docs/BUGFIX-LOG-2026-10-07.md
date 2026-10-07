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
