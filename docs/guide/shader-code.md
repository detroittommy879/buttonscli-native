---
title: Shader code reference
description: The inputs, helpers and rules for writing custom WGSL shader code in ButtonsCLI.
---

# Shader code reference

Custom shaders are written in **WGSL**, the shader language of WebGPU. You
write the *body* of one function: a few `let` lines (and, if you like, short
loops) that end with `return <color>;`. The app wraps it, checks it, and runs
it once per pixel, every frame.

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
and `ridged` 12, `noise` 4, `stars` 3, `hash2` and `prev` 2, the rest 1. One shader can
spend 240 per pixel, which keeps every shader smooth on laptop graphics. Save
a result in a `let` and reuse it instead of calling the helper again.

Built-in WGSL functions work too: `sin`, `cos`, `fract`, `floor`, `mix`,
`smoothstep`, `length`, `atan2`, `exp`, `clamp`, `select`, and so on.

## Strength

If your code uses `strength`, you decide what it does, usually
`return mix(bg, color, strength);`. If your code never mentions `strength`,
the app fades the whole result by the slider for you.

## Loops, `if` and `var`

Code may use `var`, `if`/`else` and `for` loops that always end:

```wgsl
var glow = 0.0;
for (var i = 0; i < 8; i++) {
    let k = f32(i);
    let c = vec2<f32>(0.7 * sin(t * 0.3 + k * 1.7), 0.35 * cos(t * 0.4 + k * 2.3));
    glow += halo(sd_circle(p - c, 0.03), 0.05);
}
return mix(bg, tonemap(mix(c1, c2, glow * 0.5) * glow), strength);
```

- A loop must look like `for (var i = 0; i < 8; i++)`: whole numbers written
  in the code (not a slider or `t`), a step of one, and a body that never
  changes `i`. One loop runs at most 64 times; nested loops at most 256 times
  in all.
- `while` and `loop` are not allowed: nothing can prove they end, and a
  shader that never ends freezes the graphics driver.
- Every turn of a loop costs 2 plus the helpers called in it, from the same
  budget of 240. Keep `flow`, `voronoi` and `fbm` outside loops.

These are on by default. **Create with AI → Options → Allow `var`, `if`/`else`
and fixed-count `for` loops** switches them off for every shader; shaders that
use them (some built-in ones too) then stop drawing until it is back on.

## Rules

The checker rejects code that could hang or crash a GPU driver:

- No `while`/`loop`, extra functions, arrays, structs or textures (except
  `prev`, below).
- Write floats with a decimal point: `2.0`, not `2`. Both sides of an
  operator must be the same type (`f32` with `f32`, `vec3` with `vec3`).
- Helper calls may cost at most 240 in total (see Helpers). **Create with AI
  → Options → Work limit per pixel** changes the limit for every shader; 0
  turns it off.
- At most 4,096 bytes of code (about 1,000 operations) and four sliders. Slider names are letters,
  numbers and `_`, start with a letter, and can't reuse an input or helper
  name.

Errors show under the code with line numbers, and say what to change (for
example the cost and the limit). **Fix with AI** can repair them.

## Previous frame (experiment)

With **Settings → Debug → Shader experiments → Feedback** on, code can call
`prev(q)`: the color this shader drew at position `q` last frame, as a
`vec3<f32>`. That makes trails, ripples, smears and slow build-up possible:

```wgsl
let c = vec2<f32>(0.6 * sin(t * 0.7), 0.3 * sin(t * 1.3));
let dot = halo(sd_circle(p - c, 0.02), 0.04);
let new_color = mix(bg, c1, clamp(dot, 0.0, 1.0));
return max(new_color, prev(p) * 0.96);
```

- Fade the old frame a little every time (`prev(p) * 0.96`), or it never
  goes away. Read it at a moved spot to make it flow:
  `prev(p + vec2<f32>(0.0, 0.004))`.
- A new frame is drawn each time the animation clock ticks (the FPS
  setting), so fades are faster at a higher FPS, and the picture holds
  while motion is paused. The first frame, and every resize, starts from
  black.
- What is stored is the finished pixel, after Strength.
- `prev` costs 2. Each pane (and AI result card) that uses it keeps two
  extra frame-sized images on the GPU; all of them together are capped at
  about two 4K screens' worth (around 265 MB), oldest dropped first.
- Settings → Debug is shown in debug builds, or with `"debug": true` in
  `feature-flags.json` ([local feature flags](../dev/LOCAL-FEATURE-FLAGS.md)).
  With the switch off, code that calls `prev` does not draw and its error says
  how to turn it on. Create with AI is told about `prev` only while it is on.

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
- **Many objects:** a `for` loop over 8–32 dots, each placed with
  `hash2(vec2<f32>(f32(i), 1.0))`, then `halo` each one and add them up.

**Create with AI** gives its models the same reference, with worked
examples: `assets/prompts/shader-guide.md` in the source.

## Learn more

- [The Book of Shaders](https://thebookofshaders.com/) — the classic intro
  (GLSL, but the ideas carry over directly).
- [WGSL specification](https://www.w3.org/TR/WGSL/) — the full language.
