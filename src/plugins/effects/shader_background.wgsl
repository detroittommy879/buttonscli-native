// App-owned, loop-free single-pass patterns. No textures or external resources.
struct Uniforms {
    geometry: vec4<f32>,
    settings: vec4<f32>,
    base: vec4<f32>,
    first: vec4<f32>,
    second: vec4<f32>,
    third: vec4<f32>,
    knobs: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;

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
    } else if u.settings.w > 5.5 && u.settings.w < 6.5 {
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
    if u.settings.w > 6.5 {
        let kind = u.settings.w;
        if kind < 7.5 { // Prism ribbons
            field = fract(p.x * 1.8 + sin(p.y * 4.0 + t) * 0.4 + t * 0.1);
            glow = 0.35 + 0.65 * pow(sin(field * 3.14159), 2.0);
        } else if kind < 8.5 { // Lava lamp
            field = cloud(p * 4.0 + vec2<f32>(sin(t * 0.4), t * 0.12));
            glow = smoothstep(0.25, 0.7, field);
        } else if kind < 9.5 { // Kaleidoscope
            let angle = atan2(p.y, p.x);
            field = 0.5 + 0.5 * sin(angle * 6.0 + length(p) * 8.0 - t);
            glow = 0.3 + 0.7 * abs(cos(angle * 3.0 - t * 0.2));
        } else if kind < 10.5 { // Rainbow mist
            field = 0.5 + 0.5 * sin(p.x * 3.0 + p.y * 2.0 + cloud(p * 4.0) * 3.0 + t * 0.7);
            glow = cloud(p * 3.0 + vec2<f32>(t * 0.1, 7.0));
        } else if kind < 11.5 { // Chromatic tunnel
            field = fract(length(p) * 3.0 - t * 0.3);
            glow = 0.2 + 0.8 * pow(0.5 + 0.5 * cos(length(p) * 18.0 - t * 2.0), 3.0);
        } else if kind < 12.5 { // Fireflies
            field = noise(p * 5.0 + t * 0.1);
            glow = clamp(stars(p * 8.0 + vec2<f32>(sin(t * 0.3), t * 0.1), t) + stars(p * 13.0 + 19.0, t) * 0.5, 0.0, 1.0);
        } else if kind < 13.5 { // Sunset dunes
            field = clamp(uv.y + sin(p.x * 4.0 + t * 0.3) * 0.2, 0.0, 1.0);
            glow = 0.25 + 0.75 * pow(0.5 + 0.5 * sin(p.y * 15.0 + sin(p.x * 2.0 + t) * 2.0), 2.0);
        } else if kind < 14.5 { // Ocean currents
            field = noise(p * 5.0 + vec2<f32>(t * 0.2, sin(t * 0.3)));
            glow = 0.25 + 0.75 * (0.5 + 0.5 * sin((p.x + p.y) * 10.0 + field * 5.0 - t));
        } else if kind < 15.5 { // Hex bloom
            let q = vec2<f32>(p.x * 10.0 + p.y * 5.0, p.y * 8.66);
            let cell = fract(q) - 0.5;
            field = 0.5 + 0.5 * sin(floor(q.x) + floor(q.y) + t);
            glow = exp(-length(cell) * 8.0) * (0.3 + 0.7 * field);
        } else { // Electric silk
            field = 0.5 + 0.5 * sin(p.x * 6.0 + sin(p.y * 4.0 - t) * 4.0);
            glow = pow(0.5 + 0.5 * cos(p.y * 18.0 + field * 5.0 + t), 6.0);
        }
    }
    // Full-spectrum presets pick their own colors; -1 means "use the c1–c3 mix".
    var vivid = vec3<f32>(-1.0);
    if u.settings.w > 16.5 {
        let kind = u.settings.w;
        if kind < 17.5 { // Neon plasma
            let v = sin(p.x * 4.0 + t) + sin(p.y * 5.0 - t * 0.8)
                + sin((p.x + p.y) * 3.0 + t * 0.6) + sin(length(p) * 6.0 - t);
            vivid = palette(v * 0.125 + t * 0.05);
            glow = 0.55 + 0.45 * sin(v * 1.5);
        } else if kind < 18.5 { // Synthwave
            // Screen y grows downward: sky above the horizon, grid below.
            let horizon = 0.12;
            let height = clamp((horizon - p.y) / 0.6, 0.0, 1.0);
            let sky = mix(vec3<f32>(1.0, 0.35, 0.35), vec3<f32>(0.25, 0.05, 0.45), height);
            let sun_d = length(p - vec2<f32>(0.0, horizon - 0.2));
            let slits = step(0.0, sin(p.y * 70.0 - t * 2.0)) + step(p.y, horizon - 0.25);
            let sun = (1.0 - smoothstep(0.26, 0.28, sun_d)) * min(slits, 1.0);
            let ground = p.y > horizon;
            let depth = 0.25 / max(p.y - horizon, 0.01);
            let gx = abs(fract(p.x * depth * 0.6) * 2.0 - 1.0);
            let gz = abs(fract(depth - t * 0.8) * 2.0 - 1.0);
            let lines = smoothstep(0.85, 0.97, max(gx, gz)) * clamp((p.y - horizon) * 4.0, 0.0, 1.0);
            let sky_color = mix(sky, mix(vec3<f32>(1.0, 0.85, 0.2), vec3<f32>(1.0, 0.3, 0.55), clamp((p.y - horizon + 0.45) * 2.0, 0.0, 1.0)), sun);
            vivid = select(sky_color,
                mix(vec3<f32>(0.12, 0.0, 0.2), vec3<f32>(1.0, 0.2, 0.9), lines), ground);
            glow = select(0.55 + 0.45 * sun, 0.35 + 0.65 * lines, ground);
        } else if kind < 19.5 { // Rainbow curtains
            let wave = p.x * 2.0 + sin(p.x * 3.0 + t * 0.5) * 0.6;
            let curtain = exp(-abs(p.y - 0.15 * sin(wave + t * 0.3)) * 4.0);
            vivid = palette(wave * 0.15 + p.y * 0.3 + t * 0.03);
            glow = curtain * (0.6 + 0.4 * sin(p.x * 30.0 + t * 2.0));
        } else if kind < 20.5 { // Galaxy swirl
            let r = length(p);
            let a = atan2(p.y, p.x);
            let arms = 0.5 + 0.5 * sin(a * 2.0 - log(r + 0.05) * 5.0 + t * 0.4);
            vivid = palette(r * 0.6 - t * 0.04 + arms * 0.2);
            glow = arms * exp(-r * 1.2) + stars(p * 14.0, t) * 0.8;
        } else if kind < 21.5 { // Candy stripes
            let s = (p.x + p.y) * 3.0 - t * 0.4;
            vivid = palette(floor(s * 2.0) / 7.0);
            glow = 0.45 + 0.55 * smoothstep(0.05, 0.2, abs(fract(s * 2.0) - 0.5));
        } else if kind < 22.5 { // Stained glass
            let q = p * 5.0 + vec2<f32>(noise(p * 2.0 + t * 0.1), noise(p * 2.0 - 9.0)) * 0.8;
            let cell = floor(q);
            let edge = min(min(fract(q.x), 1.0 - fract(q.x)), min(fract(q.y), 1.0 - fract(q.y)));
            vivid = hsv(hash(cell) + t * 0.02, 0.85, 1.0);
            glow = 0.15 + 0.85 * smoothstep(0.02, 0.08, edge);
        } else if kind < 23.5 { // Liquid rainbow
            let w = vec2<f32>(cloud(p * 2.0 + t * 0.07), cloud(p * 2.0 + 5.2 - t * 0.05));
            let f = cloud(p * 3.0 + w * 3.0);
            vivid = palette(f * 1.6 + t * 0.03);
            glow = 0.35 + 0.65 * smoothstep(0.2, 0.8, f);
        } else if kind < 24.5 { // Disco dots
            let g = p * 8.0;
            let cell = floor(g);
            let d = length(fract(g) - 0.5);
            let beat = 0.5 + 0.5 * sin(t * 3.0 + hash(cell) * 6.28);
            vivid = hsv(hash(cell + 3.1) + t * 0.05, 0.9, 1.0);
            glow = (1.0 - smoothstep(0.18, 0.3 + 0.1 * beat, d)) * (0.4 + 0.6 * beat);
        } else if kind < 25.5 { // Rainbow rings
            let r = length(p - 0.25 * vec2<f32>(sin(t * 0.3), cos(t * 0.2)));
            vivid = palette(r * 1.5 - t * 0.1);
            glow = 0.3 + 0.7 * pow(0.5 + 0.5 * sin(r * 24.0 - t * 2.0), 2.0);
        } else if kind < 26.5 { // Fire and ice
            let f = cloud(p * 3.0 + vec2<f32>(0.0, -t * 0.25));
            let side = smoothstep(-0.3, 0.3, p.x + (f - 0.5) * 0.8);
            let fire = mix(vec3<f32>(1.0, 0.15, 0.0), vec3<f32>(1.0, 0.85, 0.2), f);
            let ice = mix(vec3<f32>(0.0, 0.35, 1.0), vec3<f32>(0.5, 1.0, 1.0), f);
            vivid = mix(fire, ice, side);
            glow = 0.3 + 0.7 * smoothstep(0.3, 0.8, f);
        } else if kind < 27.5 { // Jellyfish
            let sway = sin(p.y * 6.0 - t * 1.5) * 0.15;
            let strands = pow(0.5 + 0.5 * sin((p.x + sway) * 18.0), 12.0);
            let bell = exp(-length((p - vec2<f32>(0.0, -0.3)) * vec2<f32>(1.0, 2.0)) * 3.0);
            vivid = mix(vec3<f32>(1.0, 0.25, 0.85), vec3<f32>(0.2, 1.0, 0.9), 0.5 + 0.5 * sin(p.y * 4.0 + t));
            glow = max(strands * smoothstep(-0.3, 0.6, p.y), bell);
        } else if kind < 28.5 { // Pixel rain
            let col = floor(p.x * 24.0);
            let speed = 0.5 + hash(vec2<f32>(col, 1.0));
            let y = fract(p.y * 1.5 * 0.3 + t * speed * 0.3 + hash(vec2<f32>(col, 7.0)));
            let cell = floor(vec2<f32>(col, p.y * 24.0));
            vivid = hsv(col * 0.07 + t * 0.03, 0.8, 1.0);
            glow = pow(y, 6.0) * step(0.3, hash(cell + floor(t * 4.0)));
        } else if kind < 29.5 { // Opal
            let n = cloud(p * 4.0 + t * 0.05);
            let iris = dot(normalize(vec3<f32>(dpdx(n), dpdy(n), 0.02)), vec3<f32>(0.4, 0.6, 0.7));
            vivid = palette(n * 2.0 + iris * 0.8 + t * 0.02);
            glow = 0.4 + 0.6 * n;
        } else if kind < 30.5 { // Tropical
            let a = cloud(p * 2.5 + vec2<f32>(t * 0.05, 0.0));
            let b = cloud(p * 2.5 + vec2<f32>(11.0, t * 0.04));
            vivid = mix(mix(vec3<f32>(0.1, 0.9, 0.5), vec3<f32>(1.0, 0.85, 0.1), a),
                vec3<f32>(1.0, 0.3, 0.6), smoothstep(0.45, 0.7, b));
            glow = 0.4 + 0.6 * max(a, b);
        } else if kind < 31.5 { // Cosmic dust
            let drift = vec2<f32>(t * 0.03, -t * 0.02);
            let neb = cloud(p * 2.2 + drift + cloud(p * 3.0 - drift));
            vivid = palette(neb * 1.3 + 0.55) + stars(p * 16.0, t) * vec3<f32>(1.0);
            glow = clamp(smoothstep(0.3, 0.75, neb) + stars(p * 16.0, t), 0.0, 1.0);
        } else if kind < 32.5 { // VHS tape: snow, tracking band, ghosting, chroma bleed
            let line = floor(uv.y * 240.0);
            let frame = floor(t * 24.0);
            let snow = hash(floor(position.xy * 0.5) + vec2<f32>(frame * 7.3, frame * 1.9));
            let roll = fract(uv.y * 0.8 - t * 0.11);
            let band = smoothstep(0.0, 0.02, roll) * (1.0 - smoothstep(0.02, 0.1, roll));
            let jitter = (hash(vec2<f32>(line, frame)) - 0.5) * 0.02 * (1.0 + band * 6.0);
            // A faint shifted copy of the stripes reads as RF ghosting.
            let stripes = 0.5 + 0.5 * sin((uv.x + jitter) * 18.0 + sin(uv.y * 3.0 + t) * 2.0);
            let ghost = 0.5 + 0.5 * sin((uv.x + jitter - 0.035) * 18.0 + sin(uv.y * 3.0 + t) * 2.0);
            let chroma = vec3<f32>(
                0.5 + 0.5 * sin((uv.x + jitter) * 9.0 + t * 0.5),
                0.5 + 0.5 * sin((uv.x + jitter) * 9.0 + 2.1 + t * 0.5),
                0.5 + 0.5 * sin((uv.x + jitter) * 9.0 + 4.2 + t * 0.5)
            );
            let scan = 0.75 + 0.25 * sin(uv.y * 754.0);
            vivid = (chroma * (0.45 + 0.35 * stripes) + vec3<f32>(0.2, 0.25, 0.35) * ghost) * scan
                + vec3<f32>(snow) * (0.18 + 0.7 * band);
            glow = clamp(0.35 + 0.4 * stripes + band * 0.6 + snow * 0.15, 0.0, 1.0) * scan;
        } else if kind < 33.5 { // CRT static: phosphor snow, scanlines, rolling bar, vignette
            let frame = floor(t * 30.0);
            let snow = hash(floor(position.xy * 0.75) + vec2<f32>(frame * 3.7, frame * 11.1));
            let scan = 0.6 + 0.4 * sin(uv.y * 900.0);
            let bar = 1.0 - smoothstep(0.0, 0.06, abs(fract(uv.y - t * 0.07) - 0.5));
            let center = uv - 0.5;
            let vignette = 1.0 - dot(center, center) * 1.6;
            let tint = mix(vec3<f32>(0.25, 1.0, 0.55), vec3<f32>(0.55, 0.75, 1.0), 0.5 + 0.5 * sin(t * 0.2));
            vivid = tint * (0.25 + 0.75 * snow) * scan * vignette + vec3<f32>(bar * 0.25);
            glow = clamp((0.3 + 0.5 * snow + bar * 0.4) * vignette * scan, 0.0, 1.0);
        } else { // Cosmic dust (fallback for unknown future indices)
            let drift = vec2<f32>(t * 0.03, -t * 0.02);
            let neb = cloud(p * 2.2 + drift + cloud(p * 3.0 - drift));
            vivid = palette(neb * 1.3 + 0.55) + stars(p * 16.0, t) * vec3<f32>(1.0);
            glow = clamp(smoothstep(0.3, 0.75, neb) + stars(p * 16.0, t), 0.0, 1.0);
        }
    }
    let color = mix(mix(u.first.rgb, u.second.rgb, field), u.third.rgb, (1.0 - field) * 0.3);
    let shade = select(color, clamp(vivid, vec3<f32>(0.0), vec3<f32>(1.0)), vivid.x >= 0.0);
    // Bound strength in the host and keep the base color dominant for legibility.
    return vec4<f32>(mix(u.base.rgb, shade, clamp(glow * u.settings.y, 0.0, 0.65)), 1.0);
}
