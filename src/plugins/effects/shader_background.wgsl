// App-owned, loop-free single-pass patterns. No textures or external resources.
struct Uniforms {
    geometry: vec4<f32>,
    settings: vec4<f32>,
    base: vec4<f32>,
    first: vec4<f32>,
    second: vec4<f32>,
    third: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;

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

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    return vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = (position.xy - u.geometry.xy) / u.geometry.zw;
    let p = (uv - 0.5) * vec2<f32>(u.geometry.z / u.geometry.w, 1.0) * u.settings.z;
    let t = u.settings.x;
    var field = 0.5 + 0.5 * sin(p.x * 4.0 + sin(p.y * 3.0 + t) * 2.0 + t * 0.7);
    var glow = exp(-abs(p.y + sin(p.x * 3.0 + t * 0.4) * 0.22) * 3.0);
    if u.settings.w > 0.5 && u.settings.w < 1.5 {
        field = 0.5 + 0.5 * sin(p.x * 5.0 + sin(p.y * 4.0 - t) * 3.0);
        glow = 0.5 + 0.5 * cos(p.y * 3.0 + sin(p.x * 4.0 + t) * 2.0);
    } else if u.settings.w > 1.5 && u.settings.w < 2.5 {
        field = 0.5 + 0.5 * sin(length(p + vec2<f32>(0.3, 0.2)) * 12.0 - t * 2.0);
        glow = 0.4 + 0.6 * (0.5 + 0.5 * sin(p.y * 8.0 + p.x * 3.0 + t));
    } else if u.settings.w > 2.5 && u.settings.w < 3.5 {
        let drift = vec2<f32>(t * 0.08, -t * 0.035);
        let warp = vec2<f32>(cloud(p * 2.1 + drift), cloud(p * 2.1 + 23.7 - drift));
        field = cloud(p * 3.2 + warp * 2.4 + drift);
        glow = smoothstep(0.15, 0.8, field) * (0.65 + 0.35 * warp.y);
        field = smoothstep(0.2, 0.75, field);
    } else if u.settings.w > 3.5 && u.settings.w < 4.5 {
        let near = stars(p * 12.0 + vec2<f32>(t * 0.16, t * 0.05), t);
        let far = stars(p * 19.0 + vec2<f32>(31.7 + t * 0.06, 8.2), t + 11.0);
        field = clamp(near, 0.0, 1.0);
        glow = clamp(near + far * 0.55, 0.0, 1.0);
    } else if u.settings.w > 4.5 && u.settings.w < 5.5 {
        let grid = p * 9.0;
        let cell = floor(grid);
        let f = fract(grid) - 0.5;
        let aa = max(fwidth(grid), vec2<f32>(0.002));
        let horizontal = (1.0 - smoothstep(0.015, 0.015 + aa.y, abs(f.y))) * step(0.35, hash(cell));
        let vertical = (1.0 - smoothstep(0.015, 0.015 + aa.x, abs(f.x))) * step(0.45, hash(cell + 31.0));
        let node = 1.0 - smoothstep(0.065, 0.065 + length(aa), length(f));
        let pulse = pow(0.5 + 0.5 * sin(grid.x * 0.8 + grid.y * 1.3 - t * 2.0), 8.0);
        glow = clamp(max(horizontal, vertical) * (0.25 + pulse * 0.75) + node * 0.5, 0.0, 1.0);
        field = pulse;
    } else if u.settings.w > 5.5 {
        let q = p * 12.0;
        let warp = vec2<f32>(sin(q.y * 1.3 + t * 0.4), cos(q.x * 1.1 - t * 0.3));
        let a = sin(q.x + warp.x + t * 0.6) + cos(q.y + warp.y - t * 0.5);
        let b = sin(q.x * 1.7 - q.y * 0.8 + warp.y - t * 0.4);
        let c = sin(q.y * 1.6 - q.x * 0.5 + warp.x + t * 0.3)
            + cos(q.x * 1.1 + q.y * 0.7 - t * 0.4);
        let ridge = min(abs(a + b * 0.8), abs(c));
        let aa = max(fwidth(ridge), 0.01);
        glow = 0.08 + 0.92 * (1.0 - smoothstep(0.04, 0.14 + aa, ridge));
        field = 0.5 + 0.5 * sin(q.y * 0.4 + t * 0.2);
    }
    let color = mix(mix(u.first.rgb, u.second.rgb, field), u.third.rgb, (1.0 - field) * 0.3);
    // Bound strength in the host and keep the base color dominant for legibility.
    return vec4<f32>(mix(u.base.rgb, color, clamp(glow * u.settings.y, 0.0, 0.65)), 1.0);
}
