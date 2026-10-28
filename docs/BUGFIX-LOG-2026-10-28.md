# Bug fix log 2026-10-28

## AI shaders: blank previews, thumbnails and terminals

Reported: AI shader previews blank; a chosen shader worked only once, after save/reopen.
Reproduced live in the dev build with the user's active profile
(`imported-default-da364bab3a22`, shader + judge models from AI models).

- **Root cause: the theme's legacy `effects.masterDisabled` hid every shader.** The edited
  theme was a copy of an imported theme (`20261007-defaults-ai-halloween-mimo`) with
  `masterDisabled: true`; 22 of that profile's 178 themes carry it. `paint_shader_preview`
  and the terminal shader filter both skipped shaders for it, with no message. The one
  success was on a theme without the flag. Edit labels that flag "animated effects
  (gradient, noise and scanlines)", so shaders no longer check it (preview and terminals).
- **AI candidates lost their slider variables.** Lanes validate code against the draft's
  sliders, or `CustomShader::default()` (`amount`) when the draft has no custom code. The
  thumbnail and "Use this one" used the draft's sliders or none, so code that used `amount`
  failed validation and drew nothing. `Candidate` now carries the variables it was checked
  with; choosing keeps the user's values for same-named sliders.
- Regression test `shaders_draw_in_legacy_effects_off_themes_and_candidates_keep_their_sliders`;
  each half fails without its fix.

How it was found: an offscreen GPU probe rendered every saved custom shader in the profile
correctly, so the AI output and pipeline were fine. A temporary trace in
`paint_shader_preview` then showed `master_disabled=true`. Probes were removed.

Live check (dev build, user's profile): saved `shad11` shader draws in the Settings preview
and the terminal; "fireball gradients" x4 with the user's models gave four drawn thumbnails;
choosing one updated the preview and terminal. Changes were reverted with "Revert and
close", so no theme file was saved.

Traps:
- `cumulative_pass_nr()` is per viewport. The shader cache evicts by it, so detached
  Settings entries look older than main-window ones. Not the cause here; only matters past
  32 cached panes/pipelines.
- On Windows, bash `~` is not `%USERPROFILE%`; the app profile is under `C:\Users\<user>`.
- `CloseMainWindow` on the app closes the detached Settings window first; call it twice.
- No python in this shell; `perl -0pi` may match an earlier identical snippet, so check
  which line changed (or use Edit).

Validation: `cargo fmt --check`, `cargo test --locked` (331 passed, 33 ignored), clippy
all-targets and wasm clippy pass. Not run: release build, Windows input probes.
