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
