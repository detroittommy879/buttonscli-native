# Shader backgrounds: first native version

Settings → Themes → Shaders has seven app-owned WGSL patterns: Aurora,
Flowing ink, Waves, Nebula, Starfield, Circuit and Water caustics.
Enable **GPU shader backgrounds** (off by default),
create a variant or open a theme in Edit, then choose a pattern and strength,
scale and motion. The preview uses synthetic text. Preview/save follows the
Library's Apply scopes: **Special effects** must be checked to apply a shared shader.
Existing individual tab themes keep their overrides. Cancel restores the draft
preview; saving keeps the configuration in the theme JSON.

A shader replaces the visible gradient behind terminal cells. The gradient or
solid fill stays underneath as a fallback when GPU backgrounds are disabled,
unavailable, or strength is zero. Explicit cell backgrounds, selection and the
cursor paint above it; a full-screen program can cover the shader. Effects master
off disables it. Calm and Animate off produce a still pattern. Settings and pane
input coordinates remain outside the effect.

The current extension is **app-owned presets**, separate from the proposed
`effects.nativeShaders` custom-code interface:

```json
"effects": {
  "shaderBackground": {
    "version": 1,
    "preset": "aurora",
    "strength": 0.35,
    "speed": 0.3,
    "scale": 1.0,
    "animated": true
  }
}
```

`preset`: `aurora`, `ink`, `waves`, `nebula`, `starfield`, `circuit`, `caustics`. Strength is clamped to 0–0.65, speed to
0–2 and scale to 0.25–4. Set `shaderBackground` to null to remove it. Unknown
versions/presets stay inert on import. Source strings are never compiled. Colors
come from the terminal background and gradient colors (or app accents if no
gradient). This is a background pass, not glyph distortion/post-processing.

Native only; the browser demo retains its existing gradient/solid fallback.
Patterns have fixed, loop-free work, one shared pipeline and at most 32 tiny
per-pane uniform buffers. Hidden panes do not draw and minimized viewports skip
the shader callback. Motion requests repaint at the lesser of the user's frame
limit and 30 FPS. Input/output or resize can cause extra renders: this is not a
strict GPU frame budget, and no old-GPU performance guarantee is made.

The `shaderBackgrounds` feature is active/free. `ShaderLab` and `VibeCodeShaders`
retain their existing planned/access states. Before accepting AI-written WGSL,
implement the restricted validator, helper preview and recovery from
[Astra's plan](AI-THEME-AND-SHADER-PLAN.md). Native shader compilation has real
driver/compiler exposure; use the checked APIs documented for the
[pinned wgpu 24 device](https://docs.rs/wgpu/24.0.5/wgpu/struct.Device.html).

## Built-in experiments

Search the Library for **Shader** to try Shader Nebula, Shader Starfield,
Shader Circuit or Shader Water Caustics. Enable GPU backgrounds in Shaders and
include Special effects when applying. Each theme also carries a dark gradient
fallback. The original three patterns remain available in the preset chooser.

- Nebula: three fixed noise octaves with domain warping, drifting colored clouds.
- Starfield: two hashed-cell layers, gentle parallax and twinkle.
- Circuit: broken grid traces, junctions and travelling light pulses.
- Water caustics: warped wave interference forming moving bright filaments.

These are original texture-free WGSL implementations informed by
[The Book of Shaders: noise](https://thebookofshaders.com/11/),
[fBM and domain warping](https://thebookofshaders.com/13/), and
[TypeShade's starfield example](https://typeshade.dev/guide/examples/starfield/).
No third-party shader source was copied. Stars and thin lines use pixel derivatives
to soften edges. Nebula costs more arithmetic than the original sine patterns;
no old-GPU performance claim is made. All presets share the existing limits,
Calm/still behavior and disable toggle.

## AI fonts

Themes → Generate now requests all seven typography zones as well as the color
palette. **Allow installed and custom fonts** defaults off: only bundled text
families and their real weights are offered. On includes fonts actually loaded
from the system/profile, with monospace eligibility. Bundled icon/emoji fallback
faces are excluded as primary fonts. Responses are validated against that catalog;
terminal tracking must be zero and sizes must be 8–32. Font names/weights are
sent to the chosen provider only as described in the UI. File paths/font bytes
and terminal content are not sent. Installed/custom families retain their names
on import; missing faces fall back to bundled faces on another machine. Detection
is bounded and currently supports TTF/OTF, not every OS font format.

Existing generation access gates are unchanged. Rich gradient/effect generation,
creative sliders and AI-authored shaders remain later work; this slice adds the
background foundation and usable font generation first.
