# Plan: shader layers and saved scenes (Oct 29)

Nothing here is built yet. Approve, cut or reorder before I start.

## What other terminals do

Most terminals that support custom shaders give you one pass over the whole frame:
Ghostty (it can chain several, but they're all post passes) and Windows Terminal's
pixel shader. To get a background-only look you detect background-colored pixels
inside that one pass. par-term is the closest to what we'd build: it has a background
shader plus a separate cursor shader, and it renders the terminal to a texture first
so its background shader can optionally run over the text too. I haven't found a
terminal that offers a separate background layer and text layer, AI-written, per
pane and per panel. So this would be a real point of difference, not a copy.

## How the cost works today

Each pane draws its own shader into its own rectangle. The GPU's work scales with
the pixels covered, so 8 panes with 8 different shaders cost about the same as one
shader over the whole window, plus a cheap pipeline switch per pane. That's why it
runs fine on an Intel iGPU. The animation FPS limit (Workspace) caps how often any
of it redraws.

A second layer per pane roughly doubles the per-pixel work in the worst case:

- **Overlay layer** (drawn on top of the text with alpha blending, can't read the
  text): about one more pass of the same size. Good for scanlines, sweeps, sparkles,
  tint, vignette, rain over text, grain. Cheap and simple.
- **True post layer** (reads the rendered text): needs the pane rendered to an
  offscreen texture first, then a pass that samples it. That adds a texture per
  pane (about 8 MB at 1080p), an extra copy, and the effect's own cost. Needed for
  glow/bloom, blur, chromatic aberration, CRT curvature, and text that wobbles.

On an iGPU, 8 panes × (background + overlay) at 30 fps should be fine for simple
shaders. Heavy AI shaders are where it would show; the FPS cap and a per-layer
"pause when the window is unfocused" handle that.

## Proposed phases

1. **Overlay layer per pane** (small/medium, about a day). Same expression language
   and AI writer as backgrounds, plus an alpha output. Sliders for strength, speed
   and scale like the background. Off by default. Stored as
   `effects.shaderOverlay` next to `shaderBackground`, so themes carry both.
2. **Panel shaders** (medium). Same callback, different rectangles: left panel
   background, top bar background, and a "buttons" surface where each button's
   rectangle gets the shader (one small draw per button). Picked per surface in a
   new "Surfaces" section of Shader backgrounds.
3. **Multi-layer AI requests** (small once 1-2 exist). One prompt such as "blue
   aliens moving in the background, red smiley faces on the buttons" returns a JSON
   set of layers `{pane_background, pane_overlay, panel_background, buttons}`; each
   is compiled and judged like today's candidates and shown as one combined preview.
4. **True post layer** (large). Offscreen per-pane render and a sampling pass.
   egui 0.31's wgpu callbacks draw straight into the frame, so this means a second
   egui render pass into a texture per pane. Worth it only if phase 1 leaves people
   wanting glow and blur.

## Saved scenes (name ideas: Scenes, Looks, Setups)

"Scene" = per-slot looks for up to 8 panes, plus the shared theme.

- Stores: shared theme ID, then slot 1..8 → theme ID (built-in or personal), plus
  the per-pane font if one was set. Keyed by **slot number**, not terminal ID,
  because IDs change every launch.
- Loading with 5 tabs open applies slots 1-5. Tabs opened later pick up slots 6-8
  while the scene is still active. Switching scenes never restarts a shell.
- Prerequisite: per-pane themes are currently forgotten on restart. They need to
  be saved first (also useful on its own).
- UI: "Save scene…" and a scene picker next to Tab groups, and in the pane menu.

## Questions for you

- Overlay first, or go straight to true post effects (glow/blur)?
- Scene name preference?
- Should a scene also save the layout (columns/rows, pane count), or only looks?
