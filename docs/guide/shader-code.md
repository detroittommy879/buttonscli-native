---
title: Shader code reference
description: The inputs, helpers and rules for writing custom WGSL shader code in ButtonsCLI.
---

# Shader code reference

Custom shaders are written in **WGSL**, the shader language of WebGPU. You
write the *body* of one function: a few `let` lines that end with
`return <color>;`. The app wraps it, checks it, and runs it once per pixel,
every frame.

```wgsl
let d = length(p);
let ring = 0.5 + 0.5 * sin(d * 8.0 - t * 2.0);
let color = mix(c1, c2, ring);
return mix(bg, color, strength);
```

A single color expression also works, e.g. `mix(c1, c2, 0.5 + 0.5 * sin(p.x + t))`.

## Inputs

| Name | Type | What it is |
| --- | --- | --- |
| `p` | `vec2<f32>` | Position in the pane, centered, scaled by **Scale**. With **Y up** on, `p.y` grows upward. |
| `t` | `f32` | Time in seconds, scaled by **Speed**. Frozen when Animate is off or Calm is on. |
| `strength` | `f32` | The Strength slider, 0 to 1. |
| `bg` | `vec3<f32>` | The terminal's background color. |
| `c1` … `c5` | `vec3<f32>` | The five colors from the Colors row. |
| your sliders | `f32` | Each slider is available by its name, e.g. `speed`. |

## Helpers

| Helper | Returns |
| --- | --- |
| `hash(p)` | A pseudo-random number 0–1 for each point (use with `floor()` for cells). |
| `noise(p)` | Smooth value noise, 0–1. |
| `cloud(p)` | Layered noise, soft and cloudy. |
| `stars(p, t)` | Twinkling star field brightness. |
| `palette(x)` | A smooth rainbow color for any number. |
| `hsv(h, s, v)` | A color from hue, saturation and value (all 0–1). |

Built-in WGSL functions work too: `sin`, `cos`, `fract`, `floor`, `mix`,
`smoothstep`, `length`, `atan2`, `exp`, `clamp`, `select`, and so on.

## Strength

If your code uses `strength`, you decide what it does, usually
`return mix(bg, color, strength);`. If your code never mentions `strength`,
the app fades the whole result by the slider for you.

## Rules

The checker rejects code that could hang or crash a GPU driver:

- No loops, `if`, `var`, extra functions, arrays or textures. Use `select()`,
  `step()` and `smoothstep()` instead of `if`, and `fract()`/`floor()` to repeat
  shapes instead of loops.
- Write floats with a decimal point: `2.0`, not `2`. Both sides of an
  operator must be the same type (`f32` with `f32`, `vec3` with `vec3`).
- At most 4,096 bytes of code and four sliders. Slider names are letters,
  numbers and `_`, start with a letter, and can't reuse an input name.

Errors show under the code with line numbers. **Fix with AI** can repair them.

## Tricks that look great

- **Tiling:** `let cell = floor(p * 6.0); let local = fract(p * 6.0) - 0.5;`
  then use `hash(cell)` to vary each tile.
- **Polar coordinates:** `let a = atan2(p.y, p.x); let r = length(p);` for
  spirals, petals and tunnels.
- **Domain warping:** feed noise into noise:
  `cloud(p + vec2<f32>(cloud(p + t * 0.1), cloud(p - t * 0.1)))`.
- **Soft shapes:** `1.0 - smoothstep(r0, r0 + 0.02, length(p - center))`.
- **Glow:** `exp(-d * 8.0)` for a soft halo around distance `d`.

## Learn more

- [The Book of Shaders](https://thebookofshaders.com/) — the classic intro
  (GLSL, but the ideas carry over directly).
- [WGSL specification](https://www.w3.org/TR/WGSL/) — the full language.
