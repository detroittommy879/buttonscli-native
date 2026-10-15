# Work log — 2026-10-15

Branch `claude/project-thread-b7w8o7`, restarted from `main` at 853878c after PR #4 merged.

- Shaders: 15 full-spectrum presets (index 17+, `ShaderBackground::FIRST_VIVID`) pick their own colors via a `vivid` color in the WGSL; c1–c3 still drive presets 0–16. Shared helpers moved to `shader_helpers.wgsl` (`SOURCE` = helpers + main) and gained `palette()`/`hsv()`. Custom expressions may call helpers: validation prepends the helpers and allows only `Statement::Call` to them (function count = helpers + 1, so no new functions). Diagnostics are shifted to `expression:LINE`.
- Editor: live compile status, **Load example** (`SHADER_EXAMPLES`, each tested), **Write/Fix with AI** (`src/shader_ai.rs`, VibeCodeShaders gate, Shader task chain, `<expression>` tag, one repair round with the Naga error), **Undo AI change**. Live check passed with the active profile's chain (mm · Inception/mercury-2.5).
- Trap: `combine()` replaces the exact `let color = mix(...)` line; it must also reset `vivid`, or a custom shader on a vivid preset index would be overridden.
- Trap: inserting a struct before `struct ThemeGenerationEvent` stole its `#[cfg(not(target_arch = "wasm32"))]`; WASM Clippy caught it.
- `render_shader_gallery` (ignored, `BUTTONSCLI_SHADER_GALLERY_DIR`) writes 480x270 PNGs per preset; GPU validation test passes for all 32.
- Website draft in `website/` (published privately as an artifact). WebGL capture needs `preserveDrawingBuffer: true` or screenshots come out black.
- Security: Ctrl+click opened any detected scheme (file://, ssh:, custom) through the OS handler, and a failed open panicked the whole app. Now only http(s)/mailto open, via `open::that_detached`, errors ignored.
- Memory: release startup private bytes 306 MB → 188 MB (working set 363 → 245) with `PowerPreference::LowPower` + `MemoryHints::MemoryUsage` in `main.rs`. CPU-side state is ~7 MB (`memory_by_startup_phase` probe); the rest is the GPU driver/allocator. `WGPU_BACKEND=gl` measured 151 MB but is not enabled by default (less tested). DX12 failed to create a surface on this machine.
- Validation: fmt, 311 unit + 2 integration tests, egui_term 23, native + WASM strict Clippy, GPU preset test, shader Settings capture, release smoke.

## Round 3 (afternoon)

- Dock/status background menu: `color_edit_button_srgba` inside a context menu opens a second popup; clicks in it close both menus unchanged. Use `color_picker_color32` inline. Test now clicks inside the picker and checks the menu stays open.
- Simulated terminal streams: `StreamFilter` shows complete lines, lags one line (echoed prompt), stops at tags/closing fence/JSON; `remainder()` draws the rest on completion. Invariant test: streamed + remainder == parsed output for several replies and chunk sizes. `parse_turn` now removes tags before stripping fences.
- Shaders `vhs` and `crt` (indices 32/33). Ten `shader-*` colorful themes; two carry custom expressions with sliders.
- Website: springs (damped oscillator), easeInCubic scramble, count-up, pointer tilt. Reveals use rect checks plus timers because rAF/IntersectionObserver stall in hidden previews; everything settles within 3 s regardless. `#motion` forces motion when the OS asks for reduced motion.
- Plan: `docs/RELEASE-AND-MESSAGES-PLAN.md`. Blog draft: `docs/blog-drafts/tool-calls-are-just-text.md`.
