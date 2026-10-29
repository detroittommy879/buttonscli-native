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

## README refresh and planning scans

- README: lightly cleaned the hand-written intro (kept the voice), filled the placeholders,
  new `main.png` alt text, and updated stale rows (AI shaders, custom WGSL, tab groups,
  profile switcher, 34 shader presets, translations). Status date now October 28.
- New [plans file](PLANS-2026-10-28.md): versions, manual-trigger release workflow, Store
  checklist, metrics, security and language scans, tab color and shader-limit plans.
- **Shader preview/terminal mismatch, cause found (not fixed):** `shader_background.rs`
  caches evict by `cumulative_pass_nr()`, which is per viewport. Main-window entries always
  look newest, so detached Settings previews evict each other and `paint` falls back to the
  built-in preset pipeline. Fix plan is in the plans file, section 8.
- `cargo audit` (installed with `cargo install cargo-audit --locked`): 12 advisories; rustls,
  rustls-webpki, webpki-roots and ctutils are semver-compatible updates. `zip` advisories cover
  `extract()`, which the app does not call.
- i18n audit: `py -3 -I scripts/audit-i18n.py` works here (python is available via `py`).

Traps:
- egui 0.31 `Context::cumulative_pass_nr()` is per viewport; never compare it across windows.

## Overnight: shader cache fix, release builds, ChatGPT sign-in, pane colors

- **Shader preview mismatch fixed.** Both effect caches (`shader_background.rs`,
  `analog_static.rs`) now stamp entries from one use counter in their GPU resources, via
  `plugins::effects::make_room`. A missing custom pipeline draws nothing instead of the preset.
  Test `shared_stamps_keep_every_entry_drawn_this_frame`; GPU adapter tests still pass.
- **Release workflow** `.github/workflows/release.yml`: `workflow_dispatch` only, optional draft
  release for the Cargo.toml version (refuses an existing tag). Builds the Windows zip, Linux
  tarball + AppImage (ubuntu-22.04, appimagetool 1.9.1, `APPIMAGE_EXTRACT_AND_RUN=1`) and a
  universal macOS `.app` in a `.dmg` (ad-hoc signed). Placeholder icon, desktop entry and
  Info.plist in `packaging/`. Verified by a run on the branch with a temporary push trigger,
  which was removed before the PR.
- **Sign in with ChatGPT** (`src/assistant/chatgpt.rs`): a provider whose `credential_ref` is
  `chatgpt:<reference>` gets a fresh access token from `provider_key`, so every AI feature works
  unchanged. Model discovery reads the plan catalog's `slug`/`visibility`. The authorization URL
  was checked against auth.openai.com (login page loads); a real sign-in was not done.
- **Pane colors**: one of ten colors per displayed slot for tab and border; the focused tab is
  filled with a keyboard mark (falls back to ▶ or * if the font lacks ⌨) and dims when the window
  is in the background. Workspace setting `pane_colors_distinct` (default on). Checked with an
  isolated 4-pane window capture.

Traps:
- Windows Credential Manager caps a generic credential at 1,280 UTF-16 characters (keyring
  checks `CRED_MAX_CREDENTIAL_BLOB_SIZE`); split long tokens across entries.
- `workflow_dispatch` workflows can only be started once the file is on the default branch;
  test from a branch with a temporary `push: branches:` trigger.
- To screenshot the app without touching the real profile, launch it with `USERPROFILE`/`HOME`
  pointing at a temp folder and seed `.buttonscli-native/profiles/default/native.json`
  (`tab_groups` with `restore_on_launch` opens several panes). `py -3 -I` hides user-installed
  Pillow; run Pillow scripts without `-I`.
- Adding an item above a `#[cfg(...)]`-gated fn moves the attribute; wasm clippy catches it.

Validation: `cargo fmt --check`, `cargo test --locked` (338 passed), native and wasm clippy.
Not run: Windows interactive input probes, a real ChatGPT sign-in, MSIX.
