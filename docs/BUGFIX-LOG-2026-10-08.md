# Bugfix log: 2026-10-08

## Font controls with themed terminals

- Started clean at bb261e9; fetched origin and inspected commits/open PRs.
- Reproduced: changing Settings' general Terminal family updates preferences,
  but terminals with theme/manual font overrides keep the old font. The new
  shader examples explicitly carry fonts, making this conflict easy to hit.
- Relabeled the section All terminals. Editing family, size, weight or bold
  defaults now applies a font override to every open PTY without changing its
  theme/shaders or restarting it. Apply to all terminals also works when the
  desired defaults were already selected. Current terminal remains independent.
- Explicitly request a root repaint for per-terminal edits/reset in detached
  Settings, including still/Calm backgrounds. Settings Revert restores font maps.
- Regression probe failed with Fira Code instead of selected Commit Mono before
  the fix; passes afterwards. Two owned finite ConPTY sessions keep their IDs,
  shader themes and callbacks while painting different chosen faces/sizes.
- Click-tested terminal plus all six UI selectors, their previews and real UI
  widget font styles; these paths already worked. No UI-font code change needed.
- Trap: PowerShell rg does not expand wildcard directory components; use rg's
  glob filter or resolve the directory first. Existing cache warnings are nonfatal.
- Validation: 264 library and two fixture tests pass; the explicit font/ConPTY
  probe passes (20 ignored probes in the ordinary suite). Formatting, diff check,
  strict native/all-target and WASM Clippy pass. No normal user profile touched.
- Optimized Windows build and isolated release startup pass. The known ended-pipe
  (109) shutdown diagnostic is nonfatal. Linux/macOS interaction remains untested.
