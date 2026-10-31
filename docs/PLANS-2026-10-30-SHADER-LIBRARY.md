# Plan: shader engine as its own library (2026-10-30)

Goal: lift the shader engine and AI pipeline out of ButtonsCLI into a reusable
library that runs in Rust apps, on the web and in other languages. Use it for a
web studio, a screensaver, explainer visuals, music-video parts and game
textures. The terminal moves onto it last.

Working name below: **`glow`**. Pick the real name before publishing; check
crates.io, npm and trademarks.

## 1. What can be reused today

| Piece | File | Ties to the app |
|---|---|---|
| Program assembly, naga validation, reserved names, y-up | `src/plugins/effects/shader_expression.rs` | `crate::theme::CustomShader`, naga through `eframe::wgpu` |
| WGSL host: presets, helpers, 144-byte uniform | `shader_background.wgsl`, `shader_helpers.wgsl` | none |
| Renderer | `src/plugins/effects/shader_background.rs` | egui paint callback, `Color32`, theme types |
| AI writer/judge loop, prompts, parsers | `src/shader_ai.rs` | `crate::assistant` (providers, transport, ChatGPT keys) |
| Data types | `CustomShader`, `ShaderVariable`, `SavedShader`, `ShaderBackground` in `src/theme.rs` | serde, theme color parsing |

About 2,300 lines. Most of the coupling is egui types and the assistant
module, so the extraction is mechanical, not a rewrite.

## 2. Key idea: the format is the product, the code is one runtime

Shader code doesn't depend on any language, so the long-lived thing is a
**document format**, not a Rust API. Call it a *piece*:

```jsonc
{
  "glow": 1,                       // format version; v1 = today's uniform layout
  "name": "Lava lamp",
  "code": "fn ... return mix(bg, c1, glow * strength);",   // WGSL body
  "params": [{ "name": "speed", "min": 0, "max": 4, "default": 1, "label": "Speed" }],
  "palette": ["#ff5a36", "#ffd166", "#06d6a0", "#118ab2", "#ef476f"],
  "y_up": true,
  "inputs": { "audio": false, "feedback": false, "image": [] },   // later phases
  "timeline": null,                // later phase: keyframes/scenes
  "meta": { "prompt": "...", "model": "...", "license": "CC-BY-4.0" }
}
```

Every host implements "load piece → compile → set time/params → draw". Any
language can then use pieces in one of three ways, cheapest first:

1. **Export** (no runtime needed): naga turns the assembled WGSL into GLSL,
   HLSL, MSL or SPIR-V. A piece can therefore be exported as a Shadertoy
   shader, an ISF file, a Godot `.gdshader`, a Unity/Unreal HLSL include or a
   plain GLSL fragment. This gives game engines and other tools access with no
   bindings at all.
2. **WASM/JS**: the Rust runtime compiled to WebAssembly and published as an
   npm package. wgpu uses WebGPU in the browser and falls back to WebGL2.
3. **Native bindings**: a C ABI (cbindgen) for C/C++, C#, Godot GDExtension
   and Unity native plugins. UniFFI or PyO3 for Python/Swift/Kotlin. Add
   these only when an app needs them.

Following ISF (JSON header plus shader code with typed inputs) keeps the
format familiar to VJ tools. ISF import/export is a stretch goal because naga's
GLSL input is partial.

## 3. Crate layout (Cargo workspace, new repo)

```text
glow/
  crates/
    glow-format    serde types, JSON schema, version migration. No GPU deps.
    glow-compile   naga: validate, assemble program, uniform layout, export
                   (GLSL/HLSL/MSL/ISF/Shadertoy). No wgpu device needed.
    glow-render    wgpu renderer: to a surface, to a texture, or offscreen
                   frames. Deterministic: same piece + time + seed = same pixels.
    glow-ai        prompts, writer→compile→judge→fix loop, parsers for
                   <colors>/<sliders>, random ideas. `Transport` trait;
                   optional `reqwest` feature. No keys stored by the library.
    glow-egui      thin egui paint-callback adapter (what the terminal uses).
    glow-wasm      wasm-bindgen API + npm package + <glow-canvas> web component.
    glow-ffi       C ABI (later).
  apps/
    glow-cli       validate, render PNG / frame sequence, export formats.
    glow-studio    web app: prompt → candidates → sliders/colors → save/share.
    glow-saver     Windows .scr / macOS .saver / Linux xscreensaver host.
  library/         curated built-in pieces (from Release picks).
```

Best-practice notes:
- Library crates are MIT OR Apache-2.0 (Rust norm). Pieces in the gallery get
  their own content license field.
- Only `glow-render`, `glow-egui` and `glow-wasm` depend on wgpu. naga and
  wgpu versions move together, so pin one wgpu line per release.
  **Trap:** ButtonsCLI is on eframe 0.31 / wgpu 24. `glow-egui` must match the
  app's wgpu version, so either start `glow` on wgpu 24 or upgrade eframe
  before the terminal moves over.
- No vendor patches in `glow`, so it can be published to crates.io (unlike the
  terminal; see `cargo-how-to.md`).
- Tests: format round-trips, a compile test for every library piece,
  golden-image GPU tests offscreen (port the gallery test), and AI parser
  tests with a fake transport (port the existing ones).
- Keep the 144-byte uniform as format v1 so every saved shader keeps working.
  New inputs (audio, feedback, more params) become v2 with a migration.

## 4. Adding time (music videos, Fantasia-style, explainers)

Today a piece has `t`, sliders and colors. Add these in order; each one is
useful on its own:

1. **Deterministic clock + offline render**: fixed fps, seeded randomness,
   `glow-cli render piece.json --fps 30 --seconds 20 --out frames/`, then pipe
   to ffmpeg for MP4/WebM, including alpha for overlays. This is the core of
   music-video and background-texture use.
2. **Feedback buffer** (previous frame as an input): this gives MilkDrop its
   trails, warps and zoom tunnels. It's the single biggest step toward that
   look, and it's one extra texture.
3. **Audio input**: an FFT/waveform texture plus `bass/mid/high/beat` uniforms,
   from the mic, a file or a stream. Offline renders analyze the file first,
   so frames stay deterministic.
4. **Timeline**: keyframes for params/colors/strength with easing, plus
   *scenes* (piece A → transition → piece B). Transitions are pieces too, with
   a `progress` input and two images, following ISF's transition convention.
5. **AI in time**: the user gives a song structure (BPM, sections with
   timestamps, mood words) or an explainer outline. The AI writes a *score*:
   a scene list, a piece prompt per scene, keyframes and transitions. Pieces
   go through the same writer/judge loop, and the score is checked and
   previewed like a shader. For a "changes with the time of day" app, the
   score is keyed to the clock instead of a song.
6. **More than shaders**: layers with blend modes, images and video as inputs,
   text/SVG overlays (for explainers), and compute-shader particles. The piece
   format becomes a small layer graph. Shaders stay the default leaf.

## 5. AI pipeline as a portable service

- `glow-ai` builds prompts and parses replies. The transport and keys belong
  to the host app, as they do in ButtonsCLI today.
- Browsers must never hold a shared API key. The web studio either uses a
  small proxy server (needed if it's ever sold or hosted) or lets users paste
  their own key, kept in the browser and labeled clearly. ChatGPT sign-in is a
  native-app feature and does not carry over to the web.
- Save successful generations (prompt, code, judge notes) as data. A curated
  set later works as few-shot examples, or for fine-tuning a small model (as
  MilkDropLM did for MilkDrop presets).

## 6. Phases

| Phase | Result | Size |
|---|---|---|
| P0 | New repo; `glow-format`, `glow-compile`, `glow-ai` copied from ButtonsCLI with tests; `glow-cli validate` | 1–2 sessions |
| P1 | `glow-render` offscreen + window preview; `glow-cli render` PNG/frames → ffmpeg | 1–2 |
| P2 | `glow-wasm` + npm + minimal dark-mode web studio (prompt, candidates, sliders, colors, share JSON) | 2–3 |
| P3 | Time: feedback buffer, audio input, timeline/scenes, AI score generation | 3+ |
| P4 | Exports: GLSL/Shadertoy/ISF/Godot/Unity; `glow-saver` | 2 |
| P5 | ButtonsCLI switches to `glow-egui` + `glow-ai` (git dependency first), deletes its copies | 1–2 |
| later | C ABI / UniFFI / Python bindings, layer graph, particles | as needed |

Start P0 by copying, not moving: ButtonsCLI keeps working unchanged until P5,
and the copies can be compared against its tests during the switch.

## 7. Existing tools (checked 2026-10-30)

- **ShaderGPT** (14islands): text prompt → GLSL fragment shader with a live
  WebGL preview, several model choices, and a community gallery. This is the
  closest match to the AI part. Per the descriptions, it has no
  generated-slider/judge loop, timeline or native/engine runtime.
- **ShadAR** (CHI EA '26 paper): voice → LLM → shader applied to AR
  passthrough on Quest 3.
- **MilkDropLM**: a 7B model fine-tuned on MilkDrop presets. Its authors
  hand-picked about 130 good presets from roughly 3,000 generated ones.
  **Prism** combines butterchurn with LLM prompt routing.
- **MilkDrop runtimes**: projectM (C++, LGPL) and butterchurn (web).
  **ISF** is the cross-app shader format used by VJ tools.
- Non-AI creative tools: Shadertoy, Hydra, TouchDesigner, Synesthesia,
  KodeLife, cables.gl.

Combining several of these is where `glow` differs:
- AI that writes the code **and** its sliders and palette, plus a
  compile/judge/fix loop.
- One format that runs natively, on the web and in exports.
- AI-written timelines for time-based work.
