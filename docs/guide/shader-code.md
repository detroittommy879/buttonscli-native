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
| `p` | `vec2<f32>` | Position in the pane, centered: `p.y` runs from -0.5 to 0.5 and `p.x` about -0.9 to 0.9 on a wide pane, both multiplied by **Scale**. With **Y up** on, `p.y` grows upward. |
| `t` | `f32` | Time in seconds, scaled by **Speed** and the status bar's **Motion**. Frozen when Animate is off, Calm is on or Motion is 0%. |
| `strength` | `f32` | The Strength slider, 0 to 1. |
| `bg` | `vec3<f32>` | The terminal's background color. |
| `c1` … `c5` | `vec3<f32>` | The five colors from the Colors row. |
| your sliders | `f32` | Each slider is available by its name, e.g. `speed`. |

## Helpers

| Helper | Returns |
| --- | --- |
| `hash(p)` | A pseudo-random number 0–1 for each point (use with `floor()` for cells). |
| `noise(p)` | Smooth value noise, 0–1. |
| `hash2(p)` | Two pseudo-random numbers as a `vec2`. |
| `cloud(p)` | Layered noise, soft and cloudy. |
| `fbm(p)` | Rich, detailed noise: clouds, terrain, nebulae. |
| `ridged(p)` | Noise with sharp creases: veins, lightning, marble. |
| `flow(p, t)` | Noise bent by drifting noise: smoke, ink, silk. |
| `voronoi(p)` | Cells as a `vec3`: `.x` distance to the cell's point, `.y` distance to the cell border, `.z` a random number per cell. |
| `stars(p, t)` | Twinkling star field brightness. |
| `rotate(p, angle)` | `p` turned around the center. |
| `polar(p)` | `.x` distance from the center, `.y` the angle as 0–1. |
| `sd_circle(p, r)`, `sd_box(p, half_size)`, `sd_segment(p, a, b)` | Distance to a shape: negative inside, positive outside. |
| `smin(d1, d2, k)` | Two shapes melted together over distance `k`. |
| `fill(d)`, `stroke(d, width)`, `halo(d, size)` | A distance as a filled shape with a smooth edge, an outline, or a glow. |
| `palette(x)` | A smooth rainbow color for any number. |
| `hsv(h, s, v)` | A color from hue, saturation and value (all 0–1). |
| `tonemap(color)` | Brings added-up lights back into 0–1 without flat white. |

The heavier helpers have a price: `flow` 44, `voronoi` 28, `fbm` 20, `cloud`
and `ridged` 12, `noise` 4, `stars` 3, `hash2` 2, the rest 1. One shader can
spend 240 per pixel, which keeps every shader smooth on laptop graphics. Save
a result in a `let` and reuse it instead of calling the helper again.

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
- Helper calls may cost at most 240 in total (see Helpers).
- At most 4,096 bytes of code (about 1,000 operations) and four sliders. Slider names are letters,
  numbers and `_`, start with a letter, and can't reuse an input name.

Errors show under the code with line numbers. **Fix with AI** can repair them.

## Tricks that look great

- **Tiling:** `let cell = floor(p * 6.0); let local = fract(p * 6.0) - 0.5;`
  then use `hash(cell)` to vary each tile.
- **Polar coordinates:** `let a = atan2(p.y, p.x); let r = length(p);` for
  spirals, petals and tunnels.
- **Domain warping:** feed noise into noise:
  `cloud(p + vec2<f32>(cloud(p + t * 0.1), cloud(p - t * 0.1)))`.
- **Shapes:** `let d = sd_circle(p - center, 0.1);` then `fill(d)` for the
  shape and `halo(d, 0.08)` for its glow. Join several with `smin`.
- **Cells:** `let cells = voronoi(p * 7.0);` then color by `cells.z` and
  darken the borders with `smoothstep(0.02, 0.07, cells.y)`.
- **Layers:** two or three layers at different sizes and speeds give depth.
  Add lights together, then `tonemap` the sum.

**Create with AI** gives its models the same reference, with worked
examples: `assets/prompts/shader-guide.md` in the source.

## Learn more

- [The Book of Shaders](https://thebookofshaders.com/) — the classic intro
  (GLSL, but the ideas carry over directly).
- [WGSL specification](https://www.w3.org/TR/WGSL/) — the full language.
