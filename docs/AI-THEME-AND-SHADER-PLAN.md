# Expressive AI themes and generated shaders

Assessment: 2026-10-07, native source at `4f32fde`. **Proposal only; no app or prompt implementation changed.** The legacy app was inspected read-only. `THEME-EVALS.md` describes the previous experiment; its palette-only restrictions are not requirements for this work.

The recommendation is to unlock the full existing appearance format first, give models the complete bundled-font catalog, and make creative controls measurable. Then add restricted, generated WGSL background shaders and overlays. Add true terminal post-processing after an offscreen-rendering prototype proves it can preserve terminal interaction and performance. Both backgrounds and post-effects are feasible, but the second needs substantially more rendering work.

## Why the previous results stayed plain

- `scripts/theme-eval.py::system_prompt` explicitly says no fonts, shaders or new keys; gradients/animation must be false and effects disabled. Its validator enforces those restrictions. Models were prevented from doing what is now wanted.
- `src/theme_generation.rs::generation_prompt` requests a palette patch and explicitly excludes effects and fonts. The response handler only applies palette fields. Changing that prompt alone would not enable full appearance generation.
- `themeprompts.md` does include a full-appearance example, but emphasizes restrained/quiet effects and recommends only a small subset of the fonts. It encourages conservative output and is separate from both generation paths above.
- A single reference palette/screenshot can anchor models to the same composition. Contrast checks are useful, but the previous trial had no test for typography variety, visible gradients, or non-color differences.

This is not evidence that those models cannot design expressive themes. Keep the old results as a palette baseline, not as the ranking for full themes.

## What works now, and what does not

| Surface | Current native behavior | Prompt consequence |
| --- | --- | --- |
| App colors | Shell, tabs, preset dock, settings, status bar; divider color/thickness | Describe all surfaces, including button/hover text relationships |
| Typography | Seven zones: shell, tabs, presetDock, settings, assistant, statusBar, terminal | Require intentional family, size, weight and UI tracking choices |
| Terminal fonts | Real bundled face selection; separate requested bold weight and bold/bright option | Use actual faces; weight 700 does not create a missing bold font |
| Gradients | Linear, radial, conic and their repeating variants; four colors; angle/position; fixed native color animation | Show every geometry; do not advertise arbitrary stop counts or custom animation speed |
| Existing effects | Analog static, scanlines, row banding, simple noise and idle noise controls | Include flags, units and combinations, not merely their names |
| Custom shader strings | Legacy shader request/source can survive as inert data; native warns that legacy GLSL does not run | Never mark an imported shader as working just because JSON imported |
| Native GPU hook | Built-in WGSL analog-static callback, with per-terminal uniforms | Useful starting pattern, not a general shader engine |
| Browser demo | Uses glow; native uses wgpu | New WGSL support is native-first; browser needs an explicit fallback or separate renderer |

Sources: `src/theme.rs` (`parse_legacy_value`, `parse_effects`, `parse_typography`), `src/fonts.rs`, `src/app.rs` (`render_terminal` and effect painting), `vendor/egui_term/src/view.rs`, `src/plugins/effects/analog_static.rs`, `Cargo.toml`/`Cargo.lock`. The pinned stack is eframe 0.31.1, wgpu 24.0.5 and Naga 24.0.0.

Important implementation details:

- `theme.terminal` font fields override matching `theme.typography.terminal` fields. Export one consistent terminal choice into both when both are present.
- UI font controls expose sizes 8–32 and tracking −1–4; these are not proof of strict importer bounds. `parse_zone` assigns numeric values directly. The generation validator must enforce finite, useful ranges itself. Terminal tracking is not connected to cell rendering: keep it zero.
- Only four gradient colors are consumed. Explicit terminal cell backgrounds paint over the gradient, so full-screen programs can cover it. Master disable stops gradient animation but does not remove the static gradient; Calm and scoped appearance choices can suppress effects independently.
- `rowBandingOpacity` uses the legacy “unit” conversion: 0.06 means 6%, but 1 means 100% before its 35% clamp. The existing guide's blanket percentage wording is ambiguous at that boundary. Generate canonical fractions for opacity fields and percentages only for fields parsed with `percent`.
- Fonts, gradients and effects have independent apply scopes/source IDs, with per-terminal overrides. An eval must apply all intended scopes in an isolated profile; a stored font change alone proves nothing about the visible result.

## Complete bundled font choices

These exact family strings come from `FONT_FACES`, not fonts assumed to be installed on Windows. All are available to UI selection; terminal eligibility below follows the current native resolver. Descriptive style hints are starting points to verify in a specimen sheet, not a substitute for seeing the font.

| Exact family | Real weights | Terminal eligible | Suggested role |
| --- | --- | --- | --- |
| Commit Mono Bundled | 200 | Yes | Light, precise technical text |
| Daddy Time Mono Bundled | 400 | Yes | More informal mono character |
| Fira Code Bundled | 400 | Yes | Familiar coding text |
| Go Noto Current Bundled | 400 | No | Broad-coverage UI/fallback |
| Noto Sans KR Bundled | 400 | No | Korean subset/UI fallback |
| JetBrains Mono Bundled | 100, 200, 300, 400 | Yes | Thin through regular technical text |
| Monoisome Bundled | 400 | Yes | Alternative mono personality |
| Noto Color Emoji Bundled | 400 | No | Emoji asset; not a general text face |
| Recursive Mono Csl St Bundled | 300 | Yes | Casual mono |
| Recursive Mono Lnr St Bundled | 300 | Yes | More linear mono |
| Recursive Sans Csl St Bundled | 300 | No | Casual UI |
| Recursive Sans Lnr St Bundled | 300 | No | Linear UI |
| Roboto Bundled | 100, 300, 400, 500 | No | Neutral UI hierarchy |
| Serious Shanns Bundled | 300 | No | Novelty UI/tabs; current terminal resolver excludes it |
| Symbols Nerd Font Mono Bundled | 400 | Yes* | Icon fallback, not primary prose/code |
| Code New Roman Bundled | 400 | No | Alternative UI; catalog currently classifies it as UI |
| iA Writer Mono S Bundled | 400 | Yes | Writing-oriented mono |
| monof55 Bundled | 400 | Yes | Monofur regular |
| monof56 Bundled | 400 | Yes | Monofur italic as a separate family |
| saxmono Bundled | 400 | Yes | Alternative mono |

*Technically allowed by the current resolver; exclude it from AI primary-font choices. Roboto LightItalic is also bundled, but shares family and weight 300 with upright Light. The current family/weight JSON cannot reliably distinguish it; do not advertise an italic switch. Likewise, bundled Noto Color Emoji does not establish full color-emoji rendering support.

Export this catalog from Rust into a machine-readable capability manifest and human-readable prompt appendix. Include face IDs, real weights, eligibility and explicit exclusions. Add a generated specimen sheet for image-capable models and user comparison. That prevents future prompts from forgetting fonts or advertising unsupported choices.

## A prompt system that encourages experiments

Use one versioned capability contract for external copy/paste prompts, the in-app generator and evaluations. Separate import compatibility (which preserves unknown legacy data) from strict generation validation (which rejects unsupported output). Do not make every old theme fail because the AI contract is stricter.

Each request should include:

1. **Creative brief and controls.** Describe the mood plus what must visibly change. Dark by default. Lockable palette/fonts/effects scopes preserve a user's existing choices.
2. **Capability packet.** Exact JSON paths/types/enums/units, complete font table, supported effects and explicit non-features. Generate it from shared definitions where possible, with parser/exporter contract tests.
3. **A few contrasting valid examples.** For example, playful typography with conic color, thin technical typography with angular gradients, and warm casual typography with texture. Examples illustrate mechanisms; do not make one bland theme the mandatory aesthetic template.
4. **A concrete variation assignment.** Ask for one candidate per request with a designated direction. Collect several distinct candidates in the host. This keeps weaker models from exhausting their output budget on a huge five-theme JSON array.
5. **Explicit output contract.** Full supported appearance JSON, all ANSI colors, all seven typography zones, deliberate gradient/effect values. A disabled effect is an intentional choice, not an omitted capability. No prose around the JSON.

Suggested controls (proposal; not all exist now):

| Control | Low | Middle | High |
| --- | --- | --- | --- |
| Wildness | Coherent familiar treatment | Distinct visual identity | Unusual pairings, geometry and composition; still usable |
| Colorfulness | Tight hue range | Several coordinated hues | Wide hue range across UI, ANSI and gradients |
| Gradient presence | Off | Clearly visible depth | Strong multi-hue geometry; not automatically brighter |
| Font adventurousness | Conventional families | Alternate mono + expressive UI | Novelty UI, italic Monofur, thin/large contrasts within role limits |
| Type scale | Compact | Clear size hierarchy | Larger tabs/labels and terminal size variation, tested for clipping |
| Motion | Still | Slow animation | More visible animation; never coupled automatically to wildness |
| Texture/effects | Clean | One visible texture | Several compatible effects within performance/readability limits |

Keep readability a floor, not a slider that turns text illegible. Thin faces need visual checking even when mathematical contrast passes. For an exploratory batch of five at high font/gradient settings, initially require at least three terminal families, three UI families, three gradient geometries, and visible size hierarchy in at least three candidates. These are tunable coverage targets, not a universal taste score. Do not force every effect into every theme; cover the feature set across the batch.

Example creative instruction to place above the generated schema/catalog:

> Design a Miami-at-night terminal with Art Deco geometry, humid neon color and playful typography. Wildness 85, colorfulness 80, gradient presence 90, font adventurousness 85, type scale 65, motion 20, texture 35. This candidate should use an expressive bundled UI family, a deliberately different eligible terminal family, visible size hierarchy and a clearly visible non-linear four-color gradient. Choose from the complete supplied catalog, not the reference's defaults. Make this a designed environment, not a recolored default. Keep body text and semantic ANSI output usable across the actual background. Use only capabilities in the attached contract. Return the complete theme JSON.

This is a proposed prompt component, not a standalone import-ready prompt. The schema, catalog and examples must accompany it. At motion zero the same visual ambition should work as a still design.

Support ordinary JSON text responses as the common denominator. Use provider structured-output modes only where verified; validate locally regardless. Image inputs are optional enhancements, not a requirement that excludes text-only models. Keep the same core brief/contract across models, record provider/model and settings, omit unsupported sampling/reasoning parameters, and allow one targeted repair with exact JSON paths or compiler errors. A token limit is not a universal billing cap. Do not let repairs silently flatten fonts/gradients to pass.

## What to reuse from the old app

`G:\ccc\z_terminals\w111erd\src\services\themeDesignerService.ts` already translates contrast, colorfulness, wildness and gradient intensity into guidance, checks gradient coverage across candidates, and locally harmonizes typography. Reuse the idea of slider-specific requirements and scope locks. Do not copy its sparse-output advice (“typography or effects sparse if ... not confident”) into this full-appearance mode. Its larger gradient-stop counts and CSS layout fields are not native capabilities. Font adventurousness should be its own new control rather than an accidental side effect of wildness.

`shaderDesignerService.ts` requests WebGL1 fragment code, supplies the exact uniforms, offers subtle/balanced/bold variations, and repairs compile failures. `terminalShaderLabGPU.ts` composites xterm canvases into a texture for `u_composite`, with resolution, texel size, time and strength. This is real post-processing, not just procedural paint over text. Reuse that explicit shader interface and repair feedback; do not copy GLSL directly into the WGSL/native path.

## Generated shaders: yes, with a restricted interface

The practical first version should generate original shader functions, not just choose predefined effects. Offer built-in effects as the lower-risk fallback. Use WGSL on the pinned native stack, a fixed host-owned vertex stage and resource bindings, and a versioned function interface. Models supply bounded fragment math; the app owns entry points, uniforms, output blending and rendering resources.

| Layer | Enables | Required work |
| --- | --- | --- |
| Background | Aurora, flowing ink, stars, water, geometric fields behind text | Hook inside the terminal background path so the normal opaque fill does not hide it; preserve explicit cell backgrounds |
| Overlay | Grain, rain, scanlines, vignette, light streaks | Bounded premultiplied alpha after terminal painting; cannot distort/sample glyphs |
| Post-process | CRT curvature, chromatic split, pixel displacement, small glow kernels | Render each terminal to its own texture, sample it in a pass, composite it back |

Start with at most one background and one overlay per pane. Later allow one post-process pass. No feedback/history buffers or user-controlled multi-pass graphs initially. Effects stay clipped to the pane; workspace chrome and recovery controls remain outside them. Backgrounds usually provide the most visual freedom without damaging glyphs. Post-effects can affect glyphs and need a separate strength control; even a visual warp must not alter PTY geometry or input coordinates.

Proposed optional `effects.nativeShaders` section: a shader-interface version and bounded layer records containing `stage`, `language: "wgsl"`, source, name, parameter definitions/defaults and strength. This is a future schema, **not supported JSON today**. Host-computed source hashes and local trust/preview records must not be accepted as approval from the model. Keep the shader extension version separate from the unrelated proposed theme-collection format. Preserve legacy source as inert metadata rather than silently enabling it.

### Safety and failure handling

Shader code has no direct shell, filesystem or network API, but executes through a compiler and GPU driver. It can still consume excessive time/memory or trigger bugs/device loss. WGSL explicitly allows device loss as an outcome of unbounded loops; native wgpu is not a browser process sandbox. Validation is necessary, not a guarantee of harmless execution. See [WGSL loop behavior](https://www.w3.org/TR/WGSL/#loop-statement).

Proposed enforceable boundary:

- First accept a small, loop-free fragment subset. Parse and inspect syntax/IR, then validate the complete host-wrapped module. Bound source bytes, expression depth, functions, expanded call cost and operations; no recursion, compute, storage writes, atomics, user bindings or arbitrary texture access. A keyword blacklist is insufficient.
- Background/overlay inputs: normalized coordinates, resolution, bounded time, palette and typed bounded parameters. Post-process additionally gets only that pane's read-only texture, with a bounded sample count/offset. No terminal-text buffers or arbitrary host resources.
- App controls output alpha/strength and rejects nonfinite parameters. Add compile/pipeline error capture, a disposable helper process with timeout/resource limits for first validation and synthetic preview, and no GPU submission until static checks pass. The helper reduces app-crash exposure but shares the physical GPU: killing it cannot guarantee cancellation of a wedged GPU job.
- Keep runtime checks enabled. wgpu 24 exposes checked shader creation, error scopes and device-loss callbacks; it also warns of parser stack demands for user input. Avoid unchecked/trusted bypasses. See [pinned wgpu Device API](https://docs.rs/wgpu/24.0.5/wgpu/struct.Device.html).
- Begin with conservative resolution/FPS/pane-count budgets; cache by source/interface/backend, suspend hidden/minimized panes, and pause motion in Calm mode. Measure compilation time, frame time and memory before raising limits. A frame watchdog can stop future submissions, not undo a submitted stall.
- Import/save source without executing it. User preview is explicit, initially with synthetic terminal content. Require a successful preview before applying new source; edits invalidate the previous preview. Recovery: effects-off startup path, last-known-good theme, persistent “preview in progress” marker cleared on clean completion, and quarantine after a crash. Do not auto-run suspicious saved shaders on restart.

Example initial limits to prototype: 16 KiB source/layer, no loops, eight numeric controls, one background + one overlay, 30 FPS maximum for generated motion, bounded helper lifetime. These are design starting points, not measured safe limits. Keep GPU-driver residual risk explicit; raw unrestricted shaders are not the recommended default.

## Implementation sequence and acceptance

Each row is a focused implementation slice with its own commit. Complete the full-appearance path before shader integration; begin shader work with a small feasibility prototype.

| Step | Files / work | Acceptance evidence |
| --- | --- | --- |
| 1. Capability contract | New shared capability module; `fonts.rs`, `theme.rs`, `theme_files.rs`; generated prompt reference | Every advertised font resolves as requested; every field imports/exports; unsupported fields identified; opacity units/ranges unambiguous |
| 2. Full-appearance generation | `theme_generation.rs`, `app.rs` Generate controls; replace palette-only validation for this mode | Fonts/sizes/tracking/gradients/effects survive response → preview → save → reload; scope locks and cancel preserve per-tab overrides |
| 3. Creative evaluation | `scripts/theme-eval.py`, fixtures, native capture probe, `themeprompts.md`, `THEME-EVALS.md` | Text-only and image-capable paths; coverage checks; several timestamps for motion; visible differences and readability judged on real captures |
| 4. Background shader prototype | `plugins/effects/`, terminal `view.rs` background hook; one hand-authored WGSL fixture | Visible behind glyphs; explicit cell backgrounds/selection/cursor work; resize/DPI and multiple panes correct; no shell respawn |
| 5. Restricted shader validator and helper | Pinned parser/validator, IPC helper, negative fixtures and resource limits | Reject forbidden resources/loops/oversize/deep expressions before GPU work; malformed source fails cleanly; helper timeout/crash recovery works |
| 6. Generated backgrounds/overlays | Versioned extension, provider prompt, import/export/editor/scopes, feature registry | Model creates a new effect; bounded repair; explicit preview/apply; disable/Calm/cancel/restart recovery; no live terminal data sent to provider |
| 7. Post-process feasibility | Per-pane offscreen target and composition integration | Texture really contains the terminal; bounded sampling works; selection/copy/scroll/IME/TUI input remain correct; measured memory/frame cost acceptable |
| 8. Post-process release work | Add stage contract, generator examples, recovery and platform tests | Both background and post-effect work together; Settings remains usable; Linux/macOS separately verified; browser fallback honest |

Existing `ShaderLab` / `VibeCodeShaders` feature entries are planned, not active. Integrate through that registry and existing access rules; do not treat the enum entries as implementation or silently change release access.

For theme comparisons, use owned temporary profiles and synthetic fixtures containing ANSI normal/bright colors, selections, explicit cell backgrounds, long tab labels, box drawing, Unicode, and a full-screen TUI. Test small/large windows and DPI. Assert **resolved** family/weight/size, not only stored JSON; capture UI zones as well as the terminal. Check text contrast across gradient positions and sampled animation times; sampling is evidence, not proof over all possible shader outputs. Also inspect bright backgrounds behind dark ANSI text and white-hover buttons.

Keep old palette tests as a separate track. Start new creative trials on the same briefs at moderate and high controls, then repeat on held-out briefs and multiple samples. Record first-pass validity, repair rate, capability coverage, similarity/near-duplicates, blind human preference, comfort, latency and actual cost. A syntactically valid theme that ignores requested font/gradient choices should fail creative compliance. Do not automatically install all technically passing candidates.

Implementation validation includes the repository's formatting, locked tests, native/WASM Clippy and release build checks, plus explicit native interaction probes. No runtime, GPU-safety, performance or cross-platform certification was performed for this assessment.
