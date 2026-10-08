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
    } else if u.settings.w > 1.5 {
        field = 0.5 + 0.5 * sin(length(p + vec2<f32>(0.3, 0.2)) * 12.0 - t * 2.0);
        glow = 0.4 + 0.6 * (0.5 + 0.5 * sin(p.y * 8.0 + p.x * 3.0 + t));
    }
    let color = mix(mix(u.first.rgb, u.second.rgb, field), u.third.rgb, (1.0 - field) * 0.3);
    // Bound strength in the host and keep the base color dominant for legibility.
    return vec4<f32>(mix(u.base.rgb, color, clamp(glow * u.settings.y, 0.0, 0.65)), 1.0);
}
