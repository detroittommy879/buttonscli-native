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

## Font spacing and mixed-theme follow-up

- Started from clean `7fe35aa`; fetched origin and inspected open draft PR #2.
- UI letter spacing was persisted but never passed into text layout. Added shared
  text helpers so tracking changes labels/button bounds before wrapping; child UIs
  inherit their font zone. Font samples use their own zone, not Settings tracking.
- Found preview/save bypassing Apply scopes and clearing per-tab themes/fonts;
  drafts also copied one source rather than the resolved shared appearance.
- V3/collection format remains a proposal only; no new tab assignment schema.
- Preview/save now honors Apply scopes and Calm mode; Edit exposes those scopes.
  Preview uses a separate ID so tabs referencing the saved source stay unchanged.
  Saving forks sources used by individual tabs/unchecked sections. Apply Fonts
  no longer changes how existing per-tab fonts render just by toggling the checkbox.
- New from current appearance and AI seed capture resolved shared colors, fonts,
  gradients/effects and divider overrides while preserving unsupported source fields.
- Added three complete current-format AI prompts and a collection proposal. Examples
  exercise real file import/export. Initial fixture failed on CRLF, then on the store's
  lazy directory creation; normalize newlines and create the owned fixture root first.
- Tracking regression checks real button width and child-zone inheritance. Initial
  test exposed a prior-frame zone leak; stamp zones by viewport/render pass so stale
  UI IDs can't change later screens. Native probe needed an explicit Instant import.
- Cancel no longer restores per-tab maps it never modifies; this preserves manual
  tab/font resets during a preview. Scope toggles immediately refresh an active preview.
  Hide temporary previews from Library results. Shared apply helper must compile on
  WASM too (initial native-only annotation failed that check).
- Final: 258 library tests and 2 import fixtures pass; 16 optional probes skipped.
  Explicit mixed-theme/spacing native capture, terminal-navigation and three-pane
  selection/copy/scroll/IME/input probes pass. Images: `.private/theme-spacing-probe/`.
  Prompt examples import and sample text contrast stays >=5.04:1. Formatting,
  native/WASM strict Clippy, optimized build and isolated release startup pass.
  Existing cache/PDB warnings remain nonfatal. No Linux/macOS or long-soak claim.

## OpenRouter theme comparison

- Verified the existing env key and all four requested exact image-capable IDs.
  Added a four-worker, three-brief palette trial with strict JSON/contrast checks,
  one bounded correction, raw responses and usage/cost/latency reports.
- Native capture uses an owned temporary profile, real file import/parser and
  synthetic PTY text. Hash-match the reference JSON/PNG before sending it; require
  native validation before exclusive writes to the active personal-theme folder.
- Probe initially treated import's filename/document tuple as a path; fixed that.
  Close is asynchronous, so guard the finished index on the following frame.
- V3 remains a separate proposal.
- Calibration caught Haiku rejecting `temperature` despite catalog support and
  GLM requiring reasoning. Don't retry permanent HTTP 4xx errors. Keep calibration
  results. User then requested defaults for both: omit temperature/reasoning;
  stopped the temporary low-reasoning trial and started a fresh defaults trial.
- Defaults: Haiku and MiMo produced three passing palettes each. GLM exhausted
  the 8k output budget on reasoning with empty content; allow 32k in a separate
  extended trial, still omitting both temperature and reasoning settings.
- Qwen billed more total/reasoning tokens than requested max_tokens: don't call
  that field a universal cost cap. Record actual usage. Keep trial budgets apart.
- Defaults trials: 16 passing files installed in active profile (Haiku/MiMo 3
  each, Qwen/GLM 5 each), after native importer/color round-trip and captures.
  Failures retained; initial calibration/low-reasoning outputs were not installed.
- Five offline failure checks, 258 library tests + 2 fixtures, formatting,
  strict native/WASM Clippy and release build pass. Native captures cover all 16
  outputs. Reports/raw responses/PNGs in ignored `.private/theme-evals/`.
