# Native Shader Lab decision

Decision date: 2026-09-29. The compatibility warning and metadata handling are
implemented on Windows. Native shader editing and rendering are not.

## Original rendering contract

The original `terminalShaderLabGPU.ts` combines xterm canvas layers into a
composite canvas, reads its pixels with `getImageData`, uploads them as a WebGL
texture, and draws a fullscreen fragment shader. Its catalog has 17 GLSL ES
presets and its custom editor accepts that GLSL ES contract. Several presets
sample neighboring pixels or distort the image, so painting colored shapes
over terminal cells cannot reproduce them.

## Native renderer boundary

Native `TerminalView` passes the Alacritty terminal grid to `egui_term`, which
paints terminal cells and glyphs directly through an egui painter. It does not
expose a texture containing the completed terminal image. Eframe 0.31 has a
wgpu render-state hook, but egui-wgpu's custom callback paints into the same
render pass as other egui shapes; the callback does not receive the already
painted terminal result as a sampleable texture. Adding a fullscreen callback
would draw above the terminal, not apply a fragment transform to it. Do not ship
that shortcut as a Shader Lab port.

## Native implementation choice

If the native Shader Lab is resumed, first change the terminal renderer to
produce a bounded offscreen color texture per visible pane. The texture must
include the same cell backgrounds, glyphs, cursor, selection, clipping, and
pixel scaling that users see before the post-process. Then compose a WGSL
pipeline over that texture. Define the WGSL bindings and entry-point contract
in the application; do not automatically translate arbitrary imported GLSL.
Any supported old preset needs an explicit reviewed WGSL implementation.

The pipeline must bound source size, compilation work, dimensions, allocations,
and frame cost. Compile and validate a candidate before making it active. A
compile or device-pipeline failure keeps the prior valid effect, and missing or
lost GPU support leaves the ordinary readable terminal active. The editor must
show diagnostics and keep unsupported imported values intact. These controls
are future acceptance requirements; they do not exist yet.

## Implemented compatibility behavior

The native theme model retains `effects.shaderLabEnabled` as a request marker.
When enabled on an imported or personal theme, Themes settings shows a warning
in the user's locale. This marker is metadata only: it does not enable any
shader. Theme JSON is still preserved by the existing personal-theme flow.
Shader Lab remains a free beta catalog entry but is unavailable until the
renderer, editor, compiler, and safe fallback are complete. AI shader
generation (`vibeCodeShaders`, Pro) stays behind that dependency.

## Evidence and status

- Original: `src/plugins/effects/terminalShaderLabGPU.ts`,
  `src/data/shaderLabPresets.ts`, and `src/components/ShaderLabCard.tsx` in the
  original checkout.
- Native: `vendor/egui_term/src/view.rs::TerminalView::ui` paints with an egui
  painter; `src/theme.rs` records the legacy request without executing it.
- Framework: eframe/egui-wgpu 0.31.1 render-state and `CallbackTrait::paint`
  APIs; custom paint callbacks are in the same egui render pass.
- Automated tests cover enabled/disabled marker parsing and localized warning
  coverage. No Shader Lab renderer, editor, compiler, visual preview, GPU
  fallback test, or live desktop review is claimed.

**V04 status:** design decision and compatibility warning are complete. The
native Shader Lab feature remains partial and requires the offscreen renderer
work as a separate architecture change.
