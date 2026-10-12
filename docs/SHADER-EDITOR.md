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
