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

// Building blocks for custom code. Custom code has no loops, so anything that
// needs repetition lives here, with a fixed amount of work per pixel. Each one
// has a price in `shader_expression::HELPER_COSTS`; keep the two in step.

fn hash2(p: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(hash(p), hash(p + vec2<f32>(37.1, 91.7)));
}

// Five octaves, each turned and doubled so the grid does not show. 0..1.
fn fbm(p: vec2<f32>) -> f32 {
    let m = mat2x2<f32>(1.616, 1.212, -1.212, 1.616);
    let p1 = m * p + vec2<f32>(3.1, 7.7);
    let p2 = m * p1 + vec2<f32>(1.7, 9.2);
    let p3 = m * p2 + vec2<f32>(8.3, 2.8);
    let p4 = m * p3 + vec2<f32>(4.1, 5.9);
    return noise(p) * 0.5 + noise(p1) * 0.25 + noise(p2) * 0.125 + noise(p3) * 0.0625
        + noise(p4) * 0.0625;
}

// Sharp creases: veins, lightning, mountain ridges, marble. 0..1.
fn ridged(p: vec2<f32>) -> f32 {
    let m = mat2x2<f32>(1.616, 1.212, -1.212, 1.616);
    let p1 = m * p + vec2<f32>(5.7, 1.3);
    let p2 = m * p1 + vec2<f32>(2.9, 6.1);
    let a = 1.0 - abs(noise(p) * 2.0 - 1.0);
    let b = 1.0 - abs(noise(p1) * 2.0 - 1.0);
    let c = 1.0 - abs(noise(p2) * 2.0 - 1.0);
    return a * a * 0.55 + b * b * 0.3 + c * c * 0.15;
}

// Noise bent by more noise and drifting with time: smoke, ink, silk. 0..1.
fn flow(p: vec2<f32>, t: f32) -> f32 {
    let bend = vec2<f32>(
        cloud(p + vec2<f32>(0.0, t * 0.11)),
        cloud(p + vec2<f32>(5.2 - t * 0.07, 1.3))
    );
    return fbm(p + bend * 2.0);
}

// Cells: x = distance to the nearest cell point, y = distance to the cell
// border (0 on the border), z = a random 0..1 number for that cell.
fn voronoi(p: vec2<f32>) -> vec3<f32> {
    let cell = floor(p);
    let f = fract(p);
    var nearest = 8.0;
    var second = 8.0;
    var to_nearest = vec2<f32>(0.0);
    var to_second = vec2<f32>(1.0, 0.0);
    var id = 0.0;
    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let offset = vec2<f32>(f32(i), f32(j));
            let q = offset + hash2(cell + offset) - f;
            let d = dot(q, q);
            if d < nearest {
                second = nearest;
                to_second = to_nearest;
                nearest = d;
                to_nearest = q;
                id = hash(cell + offset + vec2<f32>(5.3, 2.9));
            } else if d < second {
                second = d;
                to_second = q;
            }
        }
    }
    // Distance to the line halfway between the two nearest points, so
    // borders keep one width wherever they run.
    let across = to_second - to_nearest;
    let border = dot((to_nearest + to_second) * 0.5, across / max(length(across), 0.0001));
    return vec3<f32>(sqrt(nearest), max(border, 0.0), id);
}

fn rotate(p: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);
}

// x = distance from the centre, y = angle as 0..1 around the circle.
fn polar(p: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(length(p), atan2(p.y, p.x) / 6.28318 + 0.5);
}

// Signed distances: negative inside the shape, positive outside.
fn sd_circle(p: vec2<f32>, radius: f32) -> f32 {
    return length(p) - radius;
}

fn sd_box(p: vec2<f32>, half_size: vec2<f32>) -> f32 {
    let q = abs(p) - half_size;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0);
}

fn sd_segment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / max(dot(ba, ba), 0.000001), 0.0, 1.0);
    return length(pa - ba * h);
}

// Smooth union of two distances; k is how far they melt together.
fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (b - a) / max(k, 0.0001), 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}

// A distance turned into a 0..1 mask with a one-pixel soft edge.
fn fill(d: f32) -> f32 {
    let aa = max(fwidth(d), 0.0005);
    return 1.0 - smoothstep(-aa, aa, d);
}

// An outline of the given width along the shape's edge.
fn stroke(d: f32, width: f32) -> f32 {
    return fill(abs(d) - width * 0.5);
}

// Soft light around a shape: 1 inside, fading over about `size` outside.
fn halo(d: f32, size: f32) -> f32 {
    return exp(-max(d, 0.0) / max(size, 0.0001));
}

// Brings sums of bright layers back into 0..1 without flat white patches.
fn tonemap(color: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(1.0) - exp(-max(color, vec3<f32>(0.0)) * 1.4);
}

