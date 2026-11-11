# Writing a terminal background shader

You write the body of one WGSL function. It runs once for every pixel of a
terminal pane and returns that pixel's color as `vec3<f32>` RGB in 0..1. Text
is drawn on top of it.

## The contract

Write `let` lines and end with one `return <color>;`.

Not allowed: `var`, loops, `if`/`else`, `switch`, your own functions, arrays,
structs, textures. Use `select(when_false, when_true, condition)`, `mix` or
`step` instead of `if`. Each `let` name is set once and never changed.

Keep it under 3000 characters.

## Values you get

- p: vec2<f32>, pane coordinates centred on 0. p.y runs from -0.5 to 0.5 and p.x about -0.9 to 0.9 on a wide pane, so a circle of radius 0.1 is a fifth of the pane's height. The user's Scale slider multiplies p (2 shows twice as much); the settings say which way p.y points
- t: f32, time in seconds (already scaled by the user's speed). It restarts every hour, so move things with `sin`, `cos` and `fract`, which hide the restart
- strength: f32 from 0 to 1, the user's Strength slider. Always use it: it is how the user turns the whole effect up or down. At 0 the result should be just bg, at 1 the full effect. The simplest way is to end with `return mix(bg, color, strength);`. Nothing else fades your result
- bg: vec3<f32>, the terminal background color behind the text
- c1, c2, c3, c4, c5: vec3<f32>, five colors (the settings say where they come from). Use these names instead of hard-coding colors, so the user can change them later
- any slider variables listed in the request, as f32

## Helpers

The number in brackets is the price of one call. One shader may spend 240;
most good ones spend under 120. Save a result in a `let` and reuse it
instead of calling again.

Texture and noise, all 0..1:
- hash(vec2) -> f32 [1], a random number per point; `hash(floor(p * 8.0))` is one number per grid cell
- hash2(vec2) -> vec2 [2], two random numbers
- noise(vec2) -> f32 [4], smooth blobs
- cloud(vec2) -> f32 [12], soft noise with some detail
- fbm(vec2) -> f32 [20], rich detailed noise: clouds, terrain, nebulae
- ridged(vec2) -> f32 [12], sharp creases: veins, lightning, marble, mountain ridges
- flow(vec2, time) -> f32 [44], noise bent by noise that drifts on its own: smoke, ink, silk, fire
- voronoi(vec2) -> vec3 [28], cells: `.x` distance to the cell's point, `.y` distance to the cell border (0 on the border), `.z` a random number per cell
- stars(vec2, time) -> f32 [3], a twinkling star field

Space:
- rotate(vec2, angle) -> vec2, turn around the centre
- polar(vec2) -> vec2, `.x` distance from the centre, `.y` angle as 0..1 around the circle

Shapes are signed distances: negative inside, 0 on the edge, positive outside.
- sd_circle(p, radius) -> f32
- sd_box(p, half_size: vec2) -> f32
- sd_segment(p, a: vec2, b: vec2) -> f32, distance to the line from a to b
- smin(d1, d2, k) -> f32, melts two shapes together over distance k; `min(d1, d2)` joins them with a hard seam

Turning a distance d into light, all 0..1:
- fill(d) -> f32, the inside of the shape with a clean, smooth edge
- stroke(d, width) -> f32, an outline
- halo(d, size) -> f32, glow that fades over `size` outside the shape

Color:
- palette(f32) -> vec3, smooth rainbow that repeats every 1.0
- hsv(h, s, v) -> vec3, all 0..1
- tonemap(vec3) -> vec3, squeezes a sum of bright layers back into 0..1 without flat white patches. Use it whenever you add lights together

Plus WGSL built-ins: sin, cos, tan, abs, sign, fract, floor, ceil, round, mix, smoothstep, step, select, length, distance, dot, normalize, atan2, exp, log, pow, sqrt, clamp, saturate, min, max, vec2<f32>(...), vec3<f32>(...).

## How to make it look good

Build in this order: a background wash, one main subject, one layer of small
detail, then color, then `strength`.

- Pick one clear idea. One subject done well beats four fighting each other.
- Give it depth: two or three layers at different sizes and speeds, far layers smaller, dimmer and slower.
- Many copies without loops: scale p, then `let cell = floor(g);` names the tile and `fract(g) - 0.5` is the position inside it. `hash(cell)` gives each tile its own size, color, speed and delay. Keep each shape and its glow inside its tile, or the tile edges show.
- A few moving objects: write each one's position by hand from t (two to four of them), then join their distances with `smin`.
- Rays, petals, spirals, tunnels: use `polar(p)`. Multiply the angle by a whole number so the pattern meets itself.
- Never draw an edge with `step` on a distance: it is jagged. Use `fill`, `stroke` or `smoothstep`.
- Color: blend along c1..c5 with `mix` and `smoothstep` instead of using one flat color. Let one color lead, a second support it and a third appear only as an accent. Darken with `* 0.3`, never with black outlines.
- Light: add glows together, then `tonemap`. Large areas of pure white hide the text.
- Motion: slow reads as quality. Drift at `t * 0.05` to `t * 0.3`; keep anything faster small. Give layers different speeds, and use `hash(cell)` as a delay so copies do not move in step.
- At strength 1 it must be clearly visible and colorful, not mostly near-black. The user dims it with Strength.

## Mistakes that stop it compiling

- Write 2.0, not 2. Both sides of every operator need the same type: f32 with f32, vec3 with vec3 (a vec3 times an f32 is fine).
- `vec3<f32>(1.0, 0.5, 0.2)`, not `vec3(...)`, `vec3f(...)` or `float`.
- `atan2(y, x)`, not `atan(y, x)`. `fract(x)` or `x - y * floor(x / y)`, not `mod`.
- No `a ? b : c`. Write `select(c, b, a)`: the false value comes first.
- `smoothstep(low, high, x)` needs low < high. For the reverse, write `1.0 - smoothstep(low, high, x)`.
- `pow(x, y)` needs x >= 0: use `pow(abs(x), y)`. Add a small number before dividing by something that can reach 0.
- Do not give a `let` the name of a value or helper above, and do not use a name twice.
- p is a vec2. To move it, add a vec2: `p + vec2<f32>(t * 0.1, 0.0)`.

## Examples

Each one is a complete answer.

### Ink in water

```wgsl
let smoke = flow(p * 2.5, t * 0.6);
let veins = ridged(p * 4.0 + vec2<f32>(smoke, t * 0.03));
let wash = mix(c4 * 0.5, c2, smoothstep(0.35, 0.75, smoke));
let color = mix(wash, c1, veins * veins * 0.6);
let vignette = clamp(1.0 - 0.8 * dot(p, p), 0.0, 1.0);
return mix(bg, color * vignette, strength);
```

### Rising lanterns (many copies from one tile)

```wgsl
let column = floor(p.x * 6.0);
let pace = hash(vec2<f32>(column, 1.0));
let g = vec2<f32>(p.x * 6.0, p.y * 4.0 - t * (0.15 + 0.2 * pace) + pace * 9.0);
let cell = floor(g);
let seed = hash(cell);
let sway = vec2<f32>(0.1 * sin(t * 0.7 + seed * 6.28), 0.0);
let d = sd_circle(fract(g) - 0.5 - sway, 0.04 + 0.05 * seed);
let lit = step(0.4, seed);
let tint = mix(c1, c3, hash(cell + vec2<f32>(7.0, 3.0)));
let sky = mix(c4 * 0.25, c2 * 0.2, smoothstep(-0.5, 0.5, p.y));
let lantern = mix(tint, vec3<f32>(1.0), fill(d) * 0.5) * halo(d, 0.07) * lit;
return mix(bg, tonemap(sky + lantern * 1.3), strength);
```

### Stained glass (cells)

```wgsl
let cells = voronoi(p * 7.0 + vec2<f32>(t * 0.05, 0.0));
let tint = mix(mix(c1, c2, cells.z), c3, step(0.66, cells.z));
let pulse = 0.7 + 0.3 * sin(t * 0.8 + cells.z * 6.28);
let lead = smoothstep(0.02, 0.07, cells.y);
let shine = 1.0 - 0.5 * cells.x;
return mix(bg, tint * pulse * lead * shine, strength);
```

### Flower with rings (polar)

```wgsl
let q = polar(rotate(p, t * 0.05));
let petals = 0.5 + 0.5 * cos(q.y * 6.28318 * 7.0);
let d = q.x - (0.2 + 0.1 * petals);
let rings = 0.5 + 0.5 * sin(q.x * 40.0 - t * 1.2);
let inside = mix(c3, c1, clamp(q.x * 3.5, 0.0, 1.0));
let outside = mix(c4 * 0.3, c2 * 0.5, rings) * clamp(1.2 - q.x, 0.0, 1.0);
let color = mix(outside, inside, fill(d)) + c1 * halo(d, 0.06) * 0.35;
return mix(bg, tonemap(color * 1.4), strength);
```

### Night hills (layers at different depths)

```wgsl
let sky = mix(c1 * 0.5, c4 * 0.35, smoothstep(-0.3, 0.5, p.y));
let twinkle = stars(p * 18.0, t) + stars(p * 30.0 + vec2<f32>(20.0, 4.0), t + 5.0) * 0.6;
let far = fill(p.y + 0.08 - 0.22 * noise(vec2<f32>(p.x * 3.0 + t * 0.02, 1.0)));
let near = fill(p.y + 0.3 - 0.2 * noise(vec2<f32>(p.x * 5.0 + t * 0.05, 9.0)));
let scene = mix(mix(sky + vec3<f32>(twinkle), c2 * 0.3, far), c5 * 0.12, near);
return mix(bg, scene, strength);
```

### Bouncing fireballs (a few objects placed by hand)

```wgsl
let ground = -0.38;
let a = vec2<f32>(-0.7 + 1.4 * fract(t * 0.11), ground + 0.55 * abs(sin(t * 1.7)));
let b = vec2<f32>(0.7 - 1.4 * fract(t * 0.07 + 0.4), ground + 0.35 * abs(sin(t * 2.3 + 1.0)));
let c = vec2<f32>(0.4 * sin(t * 0.4), ground + 0.75 * abs(sin(t * 1.3 + 2.0)));
let d = smin(smin(sd_circle(p - a, 0.06), sd_circle(p - b, 0.045), 0.05), sd_circle(p - c, 0.075), 0.05);
let core = mix(c1, c3, fill(d + 0.03));
let fire = core * halo(d, 0.03) + c1 * halo(d, 0.16) * 0.35;
let floor_glow = c4 * 0.4 * fill(p.y - ground + 0.085);
return mix(bg, tonemap(fire * 1.5 + floor_glow), strength);
```

## Before you answer

- Does it end with one `return`, and does the result depend on `strength`?
- Is every number written with a decimal point, and every operator between matching types?
- Would it be clearly visible and colorful at strength 1, with no large pure white area?
- Does something move (unless the settings say animation is off), slowly?
- Do the colors come from c1..c5?
