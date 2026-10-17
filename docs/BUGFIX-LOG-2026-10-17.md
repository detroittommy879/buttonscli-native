# Bugfix log 2026-10-17

## Shaders page and AI shader generation

- Shaders page rebuilt as shaded sections: live preview on top, style, Create with AI
  (always visible), custom code, Keep it. No more hidden checkboxes to reach AI tools.
- `shader_ai.rs`: parallel lanes (1-6 versions, different directions). Each lane: writer
  model streams code, naga validates, judge model (new `AiTask::ShaderJudge`, own fallback
  chain) reviews/fixes; a broken judge edit never replaces working code. Results appear as
  a grid of color previews as soon as each lane is ready. Animated Write→Check→Judge→Ready
  flow per lane.
- Shader code may now be a short body (`let` lines + `return`), which removes most
  paren/comma errors from one-line expressions. Loops/`var`/`if` still rejected. Limit 4096 bytes.
- "Fix with AI" repairs a compile error in place; "Undo AI change" restores.
- Raw theme file editor (JSON) from Edit and Shaders: validates, loads into the draft,
  normal preview/save/discard apply.

## Traps

- `theme.rs` is shared with wasm; keep shader limits in `theme::MAX_CUSTOM_SHADER_BYTES`,
  not in native-only `shader_expression`.
- Files may be CRLF on Windows; perl/sed multi-line edits silently miss. Normalize or use
  a Python replace with `assert old in s`.
- Models page grew a row; tests that look for Simulation text need a taller viewport.

## Validation

- fmt, `cargo test --locked` (317 pass), clippy native + wasm clean.
- Live: `BUTTONSCLI_TEST_LIVE_SHADER_AI=1 cargo test --lib live_shader_ai_with_active_profile -- --ignored`
  3 lanes ready in ~25 s; one judge edit failed to compile and was correctly discarded.

## Deferred

- Clickable `.txt`/`.md` file names in the terminal with an Edit menu.
- Section shading on the other Settings pages (helper `settings_section` is ready).
- Vision check of rendered candidates.
