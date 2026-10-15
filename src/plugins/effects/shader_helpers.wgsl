// Shared pattern helpers. Also available to custom color expressions.
// Original implementations of value noise, domain warping and hashed star cells.
fn hash(p: vec2<f32>) -> f32 {
    let q = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    let r = q + dot(q, q.yzx + 33.33);
    return fract((r.x + r.y) * r.z);
}

fn noise(p: vec2<f32>) -> f32 {
    let cell = floor(p);
    let f = fract(p);
    let blend = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(cell), hash(cell + vec2<f32>(1.0, 0.0)), blend.x),
        mix(hash(cell + vec2<f32>(0.0, 1.0)), hash(cell + vec2<f32>(1.0)), blend.x), blend.y);
}

fn cloud(p: vec2<f32>) -> f32 {
    // Three fixed octaves keep the per-pixel workload bounded.
    return noise(p) * 0.57 + noise(p * 2.03 + 7.1) * 0.29 + noise(p * 4.07 + 19.3) * 0.14;
}

fn stars(p: vec2<f32>, t: f32) -> f32 {
    let cell = floor(p);
    let seed = hash(cell);
    let center = vec2<f32>(0.2 + 0.6 * seed, 0.2 + 0.6 * hash(cell + 47.2));
    let distance = length(fract(p) - center);
    let aa = max(length(fwidth(p)), 0.002);
    let core = 1.0 - smoothstep(0.01, 0.025 + aa, distance);
    let halo = exp(-distance * 19.0) * 0.22;
    let twinkle = 0.75 + 0.25 * sin(t * 1.4 + seed * 30.0);
    return (core + halo) * smoothstep(0.48, 0.95, seed) * twinkle;
}

// Smooth full-spectrum cosine palette: x in [0, 1] cycles through the rainbow.
fn palette(x: f32) -> vec3<f32> {
    return 0.5 + 0.5 * cos(6.28318 * (vec3<f32>(x) + vec3<f32>(0.0, 0.33, 0.67)));
}

// Hue/saturation/value to RGB, each component in [0, 1].
fn hsv(h: f32, s: f32, v: f32) -> vec3<f32> {
    let k = fract(vec3<f32>(h) + vec3<f32>(0.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0 - 3.0;
    return v * mix(vec3<f32>(1.0), clamp(abs(k) - 1.0, vec3<f32>(0.0), vec3<f32>(1.0)), s);
}

