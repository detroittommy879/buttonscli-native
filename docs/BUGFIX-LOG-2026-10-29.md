# Bugfix log 2026-10-29

## Fixes from the first ChatGPT/shader test (before merging PR 11)

- Shader Strength/Speed/Scale were capped at 0.65 / 2 / 4, and the GPU shader
  also clamped the final mix to 0.65, so raising the slider alone would not
  have helped. Limits now live in `ShaderBackground::{STRENGTH, SPEED, SCALE}`
  (1.0 / 6 / 0.1–10, scale is logarithmic) and the WGSL clamp is 1.0.
- ChatGPT plan: Test connection sent a non-streaming request and got
  "HTTP 400: Stream must be set to true". `test_connection` now retries
  through `stream_completion` when a 400 names `stream`.
- ChatGPT connections carry `chatgpt_plan: true` (set by Add connection →
  ChatGPT plan, or on sign-in). The editor then hides Base URL, API format,
  models URL, key fields, Save/Remove key and Detect format, and pins the
  endpoint to `api.openai.com/v1` + Responses. Plain OpenAI connections no
  longer show the ChatGPT block. The model list loads by itself after sign-in.
- A dropped (stale-generation) model-list result left an empty placeholder
  catalog whose fingerprint still matched, so that list never reloaded. The
  placeholder is now removed. I could not reproduce the user's missing list
  without their account, so this is the likely cause, not a confirmed one.
- Shader/theme editor: "Edit for: All panes / Pane N" row (shown with 2+
  panes). In pane mode the draft follows the focused pane, so clicking a
  terminal picks what Settings edits. Preview and save change only that pane's
  `theme_overrides` entry; discard restores it. Switching panes with unsaved
  edits is refused with a notice instead of dropping them.

Trap: per-pane theme overrides are not saved across restarts (only with
reopened closed tabs). The "saved looks" idea needs that persisted first.

Validation: fmt check, `cargo test --locked` (340 passed), native and wasm
clippy clean. New tests: `pane_editor_previews_and_saves_only_on_its_pane`,
`test_connection_streams_when_the_provider_requires_it`. Not checked by eye:
the new provider screen and the pane row (no ChatGPT account here).

## ChatGPT sign-in stopped working after a restart (second test)

- Cause: `ProviderSettings::normalize` (runs on every load and on provider
  removal) cleared any `credential_ref` not starting with `native:v1:`. That
  dropped the `chatgpt:` marker, so every request went out without a token:
  "HTTP 401: Missing bearer or basic authentication in header" in shaders,
  themes, AI Help and simulated terminals. Tokens stayed in the OS store.
- Fix: normalize keeps `chatgpt:native:v1:<hex>` and pins those connections to
  `api.openai.com/v1` + Responses. On startup `chatgpt::reconnect_saved`
  restores the marker for OpenAI-endpoint connections whose tokens are still
  saved, so no new sign-in is needed. `chatgpt_plan` is now accepted in
  provider JSON (it was exported but rejected on import).
- `chatgpt::saved_key` is the one key loader for the web preview and the live
  probes (they compared `credential_ref` to the plain ref and skipped ChatGPT).
- Live check with the user's permission, on their active profile (read-only;
  only the normal token refresh may write to the OS store):
  `BUTTONSCLI_TEST_LIVE_CHATGPT=1 cargo test --locked --lib live_chatgpt -- --ignored --nocapture`.
  Help, Theme, Shader, ShaderJudge and Simulation each got a streamed reply;
  Test connection passed; model discovery returned 8 models.

## Per-pane looks across restarts, theme bundles

- `Preferences::pane_looks` (by slot, up to 8) is filled in
  `preferences_to_save` and reapplied after startup tabs open (not when a
  launch tab group opens, which sets its own themes). Slots without a terminal
  wait and go to the next terminals opened, and are kept in the saved copy.
- Theme bundles: shared theme sources, per-slot theme + font, and the layout.
  Applying never opens/closes terminals; missing themes are skipped with a
  count; the layout is applied only with 2+ tabs.
- Plan doc for panel/button shaders and par-term ideas:
  `docs/PLANS-2026-10-29-PANEL-SHADERS.md`.
