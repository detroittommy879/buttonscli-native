// Port of the original ButtonsCLI analogStatic.ts fragment shader.
struct Uniforms {
    geometry: vec4<f32>,
    settings: vec4<f32>,
    amount: vec4<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    return vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
}

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

@fragment
fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    // The legacy overlay renders at half physical resolution. Quantize its
    // samples accordingly, retaining fine grain when DPI or pane size changes.
    let pixel = floor((position.xy - uniforms.geometry.xy) * 0.5) * 2.0;
    let uv = vec2<f32>(pixel.x, uniforms.geometry.w - pixel.y) / uniforms.geometry.zw;
    let time = uniforms.settings.x;
    let density = uniforms.settings.y;
    let drift = uniforms.settings.z;
    let brightness = uniforms.settings.w;
    let slow_wave = sin(time * 0.3) * 0.5 + 0.5;
    let med_wave = sin(time * 0.8 + 1.0) * 0.5 + 0.5;
    let noise_coord = uv * 1200.0 * density * (0.8 + slow_wave * 0.4);
    let t1 = time * 55.0;
    let t2 = time * 37.0;
    let t3 = time * 71.0;
    let n1 = hash(noise_coord + vec2<f32>(t1, t1 * 0.7));
    let n2 = hash(noise_coord * 1.6 + vec2<f32>(t2 * 0.9, t2));
    let n3 = hash(noise_coord * 2.4 + vec2<f32>(t3, t3 * 1.2));
    let mix1 = 0.5 + med_wave * 0.2;
    let mix2 = 0.3 + slow_wave * 0.15;
    var value = n1 * mix1 + n2 * mix2 + n3 * (1.0 - mix1 - mix2);
    let instability = sin(uv.y * 200.0 + time * 2.0) * (0.004 + drift * 0.032)
        * (sin(time * 0.5) * 0.5 + 0.5);
    let drift_uv = vec2<f32>(fract(uv.x + instability), uv.y);
    let drift_noise = hash(drift_uv * 800.0 + vec2<f32>(time * 40.0, 0.0));
    value = mix(value, drift_noise, 0.15);
    value *= 1.0 + sin(time * 1.2) * 0.03;
    value = value * 2.0 - 1.0;
    value = sign(value) * pow(abs(value), 0.7);
    value = clamp(value * (0.65 + brightness * 1.25), 0.0, 1.0);
    value = pow(value, 1.2);
    let alpha = uniforms.amount.x * uniforms.amount.y;
    return vec4<f32>(vec3<f32>(value * alpha), alpha);
}
