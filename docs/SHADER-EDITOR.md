# Shader backgrounds

Open **Settings → Themes → Shaders**, enable GPU backgrounds, and create a variant. Select a preset or turn on **Edit custom shader expression**. Preview is behind synthetic terminal text; saved variants use normal theme Library/Edit/apply scopes. Per-terminal theme overrides remain separate.

Ten additions: Prism ribbons, Lava lamp, Kaleidoscope, Rainbow mist, Chromatic tunnel, Fireflies, Sunset dunes, Ocean currents, Hex bloom, Electric silk. Selecting these gives a pink/cyan/gold palette; the three color pickers can change it. Turn off custom colors to inherit the theme palette.

Custom source is a **WGSL color expression**, returning `vec3<f32>`:

```wgsl
mix(c1, c2, 0.5 + 0.5 * sin(p.x * amount + t))
```

- `p`: scaled pane coordinates; `t`: time controlled by Speed/Animate/Calm.
- `c1`, `c2`, `c3`: RGB picker values; Strength controls blending with the terminal background.
- **Add variable slider:** choose a name, min/max and value, then use that name in the expression. Up to four floats, range -100 to 100. For example add `size` (0.1–20) and use `mix(c1, c3, 0.5 + 0.5 * sin(length(p) * size - t))`.
- Expressions and variables save inside `effects.shaderBackground.custom`; colors inside `effects.shaderBackground.colors`. Import/export retains other JSON fields.

Source is limited to 2,048 bytes/128 expressions and validated with the renderer's pinned Naga version. Loops, arrays, structures, extra functions, statements and GPU resources are rejected. The complete generated fragment is validated before GPU compilation. Diagnostics appear in the editor; invalid source falls back to the gradient. This is a bounded color-expression editor, not unrestricted WGSL/GLSL or terminal post-processing. Legacy GLSL metadata still does not execute.

Validation and compiled pipelines are cached with fixed limits; changing slider/color values only updates each pane's uniform buffer. GPU checks cover distinct pixels/motion for all 17 presets and custom-variable changes. Broader GPU/platform compatibility remains unverified.

## October 15 additions

- **15 full-color presets** (Neon plasma, Synthwave, Rainbow curtains, Galaxy swirl, Candy stripes, Stained glass, Liquid rainbow, Disco dots, Rainbow rings, Fire and ice, Jellyfish, Pixel rain, Opal, Tropical, Cosmic dust). They pick their own colors, so c1–c3 do not apply to them. All 32 presets are checked on a real GPU for distinct, moving pixels.
- **Helpers in expressions:** `palette(x)` (smooth rainbow), `hsv(h, s, v)`, `noise(p)`, `cloud(p)`, `hash(p)` and `stars(p, t)` are callable from custom expressions. They are fixed and loop-free; adding functions is still rejected.
- **Editor:** live compile status (green check or the Naga error with line numbers relative to the expression), a **Load example** menu, and **Write with AI** / **Fix with AI** (VibeCodeShaders access). AI answers use the Shader task model, must come back in an `<expression>` tag, are validated by the same checker, and get one automatic repair round with the compiler error. The result goes into the unsaved draft; **Undo AI change** restores the previous expression.
- Gallery images: `BUTTONSCLI_SHADER_GALLERY_DIR=<dir> cargo test --locked --lib render_shader_gallery -- --ignored` writes one 480x270 PNG per preset.
