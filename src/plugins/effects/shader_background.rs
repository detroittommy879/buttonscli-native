//! Bounded background patterns and validated resource-free color expressions.

use std::collections::HashMap;

use eframe::{egui_wgpu, wgpu};
use egui::{Context, Id, Rect, Ui};

use crate::theme::ShaderBackground;
use egui::Color32;

const FORMAT_ID: &str = "buttonscli-shader-background-format";
const MAX_BUFFERS: usize = 32;

/// Previous frames for `prev()` are kept as 16-bit floats: in 8 bits a slow
/// fade such as `prev(p) * 0.96` gets stuck on rounding and never reaches the
/// background.
const FEEDBACK_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
/// Panes that keep previous frames at once (each holds two frame-sized
/// images), and the largest side of those images.
const MAX_FEEDBACK_PANES: usize = 6;
const MAX_FEEDBACK_SIDE: u32 = 4096;

/// Shows a pane's newest feedback frame on screen.
const COPY_SOURCE: &str = "struct Geometry { geometry: vec4<f32> };
@group(0) @binding(0) var<uniform> u: Geometry;
@group(0) @binding(1) var frame: texture_2d<f32>;
@group(0) @binding(2) var frame_sampler: sampler;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    return vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = (position.xy - u.geometry.xy) / u.geometry.zw;
    return vec4<f32>(textureSampleLevel(frame, frame_sampler, uv, 0.0).rgb, 1.0);
}
";

pub(crate) fn register(ctx: &Context, format: wgpu::TextureFormat) {
    ctx.data_mut(|data| data.insert_temp(Id::new(FORMAT_ID), format));
}

pub(crate) fn callback(
    ui: &Ui,
    rect: Rect,
    shader: &ShaderBackground,
    terminal: u64,
    time: f32,
    mut colors: [Color32; 6],
) -> Option<egui::PaintCallback> {
    if ui.input(|input| input.viewport().minimized.unwrap_or(false)) {
        return None;
    }
    let format = ui
        .ctx()
        .data(|data| data.get_temp::<wgpu::TextureFormat>(Id::new(FORMAT_ID)))?;
    // The shader's own colors replace the theme's; missing ones keep the theme's.
    if let Some(palette) = shader.colors.as_ref().filter(|_| !shader.theme_colors) {
        for (target, color) in colors[1..].iter_mut().zip(palette) {
            *target = crate::theme::parse_color(color)?;
        }
    }
    let mut knobs = [0.0; 4];
    let feedback = shader
        .custom
        .as_ref()
        .is_some_and(|custom| super::shader_expression::uses_feedback(&custom.expression));
    let custom_source = if let Some(custom) = &shader.custom {
        // The editor can temporarily contain an inverted range while typing.
        // Validate before clamping, since f32::clamp panics on invalid bounds.
        let source = custom_source(custom).ok()?;
        for (target, variable) in knobs.iter_mut().zip(&custom.variables) {
            *target = variable.value.clamp(variable.min, variable.max);
        }
        Some(source)
    } else {
        None
    };
    Some(egui_wgpu::Callback::new_paint_callback(
        rect,
        Background {
            rect,
            format,
            terminal,
            values: [
                if shader.animated {
                    time.rem_euclid(3600.0) * shader.speed
                } else {
                    0.0
                },
                shader.strength,
                shader.scale,
                shader.preset_index()? as f32,
            ],
            colors,
            knobs,
            custom_source,
            feedback,
        },
    ))
}

struct Background {
    rect: Rect,
    format: wgpu::TextureFormat,
    terminal: u64,
    values: [f32; 4],
    /// Background, then c1–c5.
    colors: [Color32; 6],
    knobs: [f32; 4],
    custom_source: Option<String>,
    /// The code calls `prev()`: it draws into the pane's feedback frames,
    /// which are then copied to the screen.
    feedback: bool,
}

struct PaneUniform {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    last_used: u64,
}

struct Resources {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    panes: HashMap<u64, PaneUniform>,
    /// `None` is a shader the graphics driver refused: it is not drawn and
    /// not tried again every frame.
    custom_pipelines: HashMap<String, (Option<wgpu::RenderPipeline>, u64)>,
    /// Uniforms plus a frame and its sampler: `prev()` shaders and the copy.
    feedback_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    copy_pipeline: wgpu::RenderPipeline,
    /// `prev()` shaders, drawn into [`FEEDBACK_FORMAT`] frames.
    feedback_pipelines: HashMap<String, (Option<wgpu::RenderPipeline>, u64)>,
    feedback: HashMap<u64, FeedbackPane>,
    /// One use counter for every window; see [`super::make_room`].
    uses: u64,
}

impl Resources {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = uniform_layout(device);
        let pipeline = pipeline(device, format, &layout, super::shader_expression::SOURCE);
        let feedback_layout = feedback_layout(device);
        let copy_pipeline = self::pipeline(device, format, &feedback_layout, COPY_SOURCE);
        Self {
            pipeline,
            layout,
            panes: HashMap::new(),
            custom_pipelines: HashMap::new(),
            sampler: frame_sampler(device),
            feedback_layout,
            copy_pipeline,
            feedback_pipelines: HashMap::new(),
            feedback: HashMap::new(),
            uses: 0,
        }
    }
}

/// Two frames of one pane's `prev()` shader: each frame is drawn into one
/// while reading the other, then shown with the copy pipeline.
struct FeedbackPane {
    size: (u32, u32),
    /// The program these frames belong to; other code starts from black.
    source: String,
    /// Screen geometry, for the copy.
    screen: wgpu::Buffer,
    /// The same values with the frame's own geometry, for drawing into it.
    offscreen: wgpu::Buffer,
    views: [wgpu::TextureView; 2],
    /// `draw[i]` writes frame `i` and reads frame `1 - i`.
    draw: [wgpu::BindGroup; 2],
    /// `copy[i]` shows frame `i`.
    copy: [wgpu::BindGroup; 2],
    /// The frame drawn last.
    latest: usize,
    /// Shader time of the frame drawn last. A new frame is drawn only when
    /// the animation clock moves, so fading follows the FPS limit rather than
    /// how often the window repaints, and a paused shader holds its picture.
    drawn_at: Option<f32>,
    last_used: u64,
}

impl FeedbackPane {
    /// New frames start black (wgpu clears new textures).
    fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        size: (u32, u32),
        source: String,
    ) -> Self {
        let buffer = |label| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: 144,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let screen = buffer("shader feedback screen");
        let offscreen = buffer("shader feedback frame");
        let views = [0, 1].map(|_| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("shader feedback frame"),
                    size: wgpu::Extent3d {
                        width: size.0,
                        height: size.1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: FEEDBACK_FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        });
        let bind = |uniform: &wgpu::Buffer, view: &wgpu::TextureView| {
            frame_bindings(device, layout, uniform, view, sampler)
        };
        Self {
            draw: [bind(&offscreen, &views[1]), bind(&offscreen, &views[0])],
            copy: [bind(&screen, &views[0]), bind(&screen, &views[1])],
            size,
            source,
            screen,
            offscreen,
            views,
            latest: 0,
            drawn_at: None,
            last_used: 0,
        }
    }

    /// Draws the next frame from the last one.
    fn advance(&mut self, encoder: &mut wgpu::CommandEncoder, pipeline: &wgpu::RenderPipeline) {
        let next = 1 - self.latest;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shader feedback frame"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.views[next],
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &self.draw[next], &[]);
        pass.draw(0..3, 0..1);
        drop(pass);
        self.latest = next;
    }

    /// Copies the newest frame into the pane's rectangle.
    fn show(&self, pass: &mut wgpu::RenderPass<'_>, copy: &wgpu::RenderPipeline) {
        pass.set_pipeline(copy);
        pass.set_bind_group(0, &self.copy[self.latest], &[]);
        pass.draw(0..3, 0..1);
    }
}

fn frame_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("shader feedback sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    })
}

fn frame_bindings(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform: &wgpu::Buffer,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("shader feedback bindings"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

/// Builds a custom pipeline once and remembers a driver refusal.
fn cached_pipeline(
    pipelines: &mut HashMap<String, (Option<wgpu::RenderPipeline>, u64)>,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::BindGroupLayout,
    source: &str,
    now: u64,
) {
    if !pipelines.contains_key(source) {
        super::make_room(pipelines, MAX_BUFFERS, |(_, used)| *used);
        let built = checked_pipeline(device, format, layout, source);
        if let Err(error) = &built {
            log::warn!("the graphics driver refused a custom shader: {error}");
            if let Ok(mut refused) = REFUSED.lock() {
                if refused.len() >= 16 {
                    refused.remove(0);
                }
                refused.push((source.to_owned(), error.clone()));
            }
        }
        pipelines.insert(source.to_owned(), (built.ok(), now));
    }
    if let Some(entry) = pipelines.get_mut(source) {
        entry.1 = now;
    }
}

fn custom_source(custom: &crate::theme::CustomShader) -> Result<String, String> {
    super::shader_expression::program(custom)
}

/// Builds a custom shader's pipeline and reports what the graphics driver
/// said. Naga already accepted the code, but the driver's own compiler can
/// still refuse it or run out of memory; without a scope wgpu treats that as
/// fatal and the whole app closes.
fn checked_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::BindGroupLayout,
    source: &str,
) -> Result<wgpu::RenderPipeline, String> {
    use std::future::Future;
    const FILTERS: [wgpu::ErrorFilter; 3] = [
        wgpu::ErrorFilter::Validation,
        wgpu::ErrorFilter::OutOfMemory,
        wgpu::ErrorFilter::Internal,
    ];
    #[cfg(not(target_arch = "wasm32"))]
    let _one_at_a_time = BUILDING.lock();
    for filter in FILTERS {
        device.push_error_scope(filter);
    }
    let built = pipeline(device, format, layout, source);
    let mut problem = None;
    for _ in FILTERS {
        // Native wgpu answers at once. In a browser the answer comes later,
        // and the browser reports the error itself instead of aborting.
        let scope = std::pin::pin!(device.pop_error_scope());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        if let std::task::Poll::Ready(Some(error)) = scope.poll(&mut context) {
            problem.get_or_insert_with(|| error.to_string());
        }
    }
    match problem {
        Some(error) => Err(error),
        None => Ok(built),
    }
}

/// Error scopes belong to the whole device, so two threads building pipelines
/// at once would read each other's errors.
#[cfg(not(target_arch = "wasm32"))]
static BUILDING: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The app's graphics device, for drawing AI results off screen.
#[cfg(not(target_arch = "wasm32"))]
static PICTURE_DEVICE: std::sync::OnceLock<(wgpu::Device, wgpu::Queue)> =
    std::sync::OnceLock::new();

/// Lets [`picture_problem`] draw with the app's own graphics device.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn register_device(device: &wgpu::Device, queue: &wgpu::Queue) {
    let _ = PICTURE_DEVICE.set((device.clone(), queue.clone()));
}

/// Draws a custom shader off screen at full strength, at two moments, and
/// says what is wrong with the picture: one flat color, almost nothing drawn,
/// mostly white, or nothing moving. `None` means it looks fine, or that it
/// could not be drawn here (no graphics device yet); it is advice for the AI
/// writer, never a reason to refuse a shader the user wrote.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn picture_problem(
    custom: &crate::theme::CustomShader,
    background: &str,
    colors: &[String; 5],
    animated: bool,
) -> Option<String> {
    let (device, queue) = PICTURE_DEVICE.get()?;
    let source = custom_source(custom).ok()?;
    picture_problem_on(device, queue, &source, custom, background, colors, animated)
}

/// `source` is `custom`'s full program. A `prev()` shader is drawn from a
/// black previous frame, as its first frame on screen is.
#[cfg(not(target_arch = "wasm32"))]
fn picture_problem_on(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    source: &str,
    custom: &crate::theme::CustomShader,
    background: &str,
    colors: &[String; 5],
    animated: bool,
) -> Option<String> {
    const WIDTH: usize = 64;
    const HEIGHT: usize = 32;
    let feedback = super::shader_expression::uses_feedback(&custom.expression);
    let layout = if feedback {
        feedback_layout(device)
    } else {
        uniform_layout(device)
    };
    let blank: Option<&[u8]> = feedback.then_some(&[]);
    let pipeline = match checked_pipeline(device, wgpu::TextureFormat::Rgba8Unorm, &layout, source)
    {
        Ok(pipeline) => pipeline,
        Err(error) => {
            return Some(format!(
                "the graphics driver refused the code: {}.",
                error.chars().take(400).collect::<String>()
            ))
        }
    };
    // geometry, settings (time, strength, scale, preset), base, c1–c3, knobs, c4, c5.
    let mut values = [0.0_f32; 36];
    values[2] = WIDTH as f32;
    values[3] = HEIGHT as f32;
    values[5] = 1.0;
    values[6] = 1.0;
    for (offset, color) in [8, 12, 16, 20, 28, 32]
        .into_iter()
        .zip(std::iter::once(background).chain(colors.iter().map(String::as_str)))
    {
        let rgba = crate::theme::parse_color(color)?.to_array();
        for channel in 0..4 {
            values[offset + channel] = f32::from(rgba[channel]) / 255.0;
        }
    }
    for (target, variable) in values[24..28].iter_mut().zip(&custom.variables) {
        *target = variable.value;
    }
    let base: Vec<u8> = values[8..11]
        .iter()
        .map(|channel| (channel * 255.0).round() as u8)
        .collect();
    values[4] = 1.3;
    let first = render_pixels(device, queue, &layout, &pipeline, values, blank)?;
    let pixels = (WIDTH * HEIGHT) as f32;
    let differs = |a: &[u8], b: &[u8], by: i32| {
        a.iter()
            .zip(b)
            .take(3)
            .map(|(a, b)| (i32::from(*a) - i32::from(*b)).abs())
            .sum::<i32>()
            > by
    };
    let share = |keep: &dyn Fn(&[u8]) -> bool| {
        first.chunks_exact(4).filter(|pixel| keep(pixel)).count() as f32 / pixels
    };
    let spread = (0..3)
        .map(|channel| {
            let values = first.chunks_exact(4).map(|pixel| pixel[channel]);
            values.clone().max().unwrap_or(0) - values.min().unwrap_or(0)
        })
        .max()
        .unwrap_or(0);
    if spread < 8 {
        return Some("the whole picture was one flat color.".into());
    }
    if share(&|pixel| differs(pixel, &base, 30)) < 0.01 {
        return Some(
            "almost all of it was the plain background: hardly anything was drawn.".into(),
        );
    }
    if share(&|pixel| pixel[..3].iter().all(|channel| *channel > 245)) > 0.35 {
        return Some("more than a third of it was pure white, which hides the text.".into());
    }
    if animated {
        values[4] = 4.9;
        let second = render_pixels(device, queue, &layout, &pipeline, values, blank)?;
        let moved = first
            .chunks_exact(4)
            .zip(second.chunks_exact(4))
            .filter(|(a, b)| differs(a, b, 6))
            .count() as f32
            / pixels;
        if moved < 0.005 {
            return Some("nothing moved between two moments a few seconds apart.".into());
        }
    }
    None
}

fn feedback_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("shader feedback uniforms and frame"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn uniform_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("shader background uniforms"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

/// Draws one frame into a small off-screen image and returns its RGBA bytes.
/// `values` is the uniform block; its size must have a width divisible by 64.
/// `prev` is the previous frame (RGBA bytes, empty for black) for a `prev()`
/// shader, built with [`feedback_layout`].
#[cfg(not(target_arch = "wasm32"))]
fn render_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    pipeline: &wgpu::RenderPipeline,
    values: [f32; 36],
    prev: Option<&[u8]>,
) -> Option<Vec<u8>> {
    use wgpu::util::DeviceExt;
    let width = values[2] as u32;
    let height = values[3] as u32;
    // Rows are copied without padding, which needs 256-byte rows.
    if width == 0 || height == 0 || width % 64 != 0 {
        return None;
    }
    let bytes: Vec<u8> = values.into_iter().flat_map(f32::to_le_bytes).collect();
    let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("shader picture uniforms"),
        contents: &bytes,
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let bindings = match prev {
        Some(prev) => {
            let frame = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("shader picture previous frame"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            if prev.len() == (width * height * 4) as usize {
                queue.write_texture(
                    frame.as_image_copy(),
                    prev,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(width * 4),
                        rows_per_image: Some(height),
                    },
                    frame.size(),
                );
            }
            frame_bindings(
                device,
                layout,
                &uniform,
                &frame.create_view(&Default::default()),
                &frame_sampler(device),
            )
        }
        None => device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        }),
    };
    draw_and_read(device, queue, (width, height), |encoder, view| {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bindings, &[]);
        pass.draw(0..3, 0..1);
    })
}

/// Runs `draw` into a new RGBA8 image of `size` (width a multiple of 64) and
/// returns its bytes.
#[cfg(not(target_arch = "wasm32"))]
fn draw_and_read(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    (width, height): (u32, u32),
    draw: impl FnOnce(&mut wgpu::CommandEncoder, &wgpu::TextureView),
) -> Option<Vec<u8>> {
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("shader picture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(width * height * 4),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    draw(&mut encoder, &view);
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
        },
        size,
    );
    queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    // The app's own frames also poll the device, so wait in short steps
    // rather than blocking on it.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let _ = device.poll(wgpu::Maintain::Poll);
        match receiver.recv_timeout(std::time::Duration::from_millis(5)) {
            Ok(Ok(())) => break,
            Ok(Err(_)) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return None,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if std::time::Instant::now() > deadline {
                    return None;
                }
            }
        }
    }
    let output = readback.slice(..).get_mapped_range().to_vec();
    readback.unmap();
    Some(output)
}

/// What the graphics driver said about custom shaders it refused, newest
/// last. Kept short: it is a hint for the editor, not a history.
static REFUSED: std::sync::Mutex<Vec<(String, String)>> = std::sync::Mutex::new(Vec::new());

/// The driver's message if it refused this custom shader during this run.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn driver_error(custom: &crate::theme::CustomShader) -> Option<String> {
    let source = custom_source(custom).ok()?;
    let refused = REFUSED.lock().ok()?;
    refused
        .iter()
        .find(|(refused, _)| *refused == source)
        .map(|(_, error)| error.clone())
}

fn pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::BindGroupLayout,
    source: &str,
) -> wgpu::RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("shader background pipeline layout"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shader background shader"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("shader background screen overlay"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview: None,
        cache: None,
    })
}

impl egui_wgpu::CallbackTrait for Background {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen: &egui_wgpu::ScreenDescriptor,
        encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if resources.get::<Resources>().is_none() {
            resources.insert(Resources::new(device, self.format));
        }
        let resources = resources
            .get_mut::<Resources>()
            .expect("initialized shader background resources");
        resources.uses += 1;
        let now = resources.uses;
        match &self.custom_source {
            Some(source) if self.feedback => cached_pipeline(
                &mut resources.feedback_pipelines,
                device,
                FEEDBACK_FORMAT,
                &resources.feedback_layout,
                source,
                now,
            ),
            Some(source) => cached_pipeline(
                &mut resources.custom_pipelines,
                device,
                self.format,
                &resources.layout,
                source,
                now,
            ),
            None => {}
        }
        if !resources.panes.contains_key(&self.terminal) {
            // Only tiny uniform buffers are cached, with a hard bound even
            // after many tab-close/reopen cycles. No pane image is retained.
            super::make_room(&mut resources.panes, MAX_BUFFERS, |pane| pane.last_used);
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("shader background pane"),
                size: 144,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("shader background pane bindings"),
                layout: &resources.layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });
            resources.panes.insert(
                self.terminal,
                PaneUniform {
                    buffer,
                    bind_group,
                    last_used: now,
                },
            );
        }
        let pane = resources
            .panes
            .get_mut(&self.terminal)
            .expect("initialized pane uniforms");
        pane.last_used = now;
        let scale = screen.pixels_per_point;
        let values = [
            self.rect.left() * scale,
            self.rect.top() * scale,
            (self.rect.width() * scale).max(1.0),
            (self.rect.height() * scale).max(1.0),
            self.values[0],
            self.values[1],
            self.values[2],
            self.values[3],
        ];
        // Uniform order: geometry, settings, base, c1–c3, knobs, c4, c5.
        let rgba = |color: &Color32| {
            [
                color.r() as f32 / 255.0,
                color.g() as f32 / 255.0,
                color.b() as f32 / 255.0,
                1.0,
            ]
        };
        let bytes: Vec<u8> = values
            .into_iter()
            .chain(self.colors[..4].iter().flat_map(rgba))
            .chain(self.knobs)
            .chain(self.colors[4..].iter().flat_map(rgba))
            .flat_map(f32::to_le_bytes)
            .collect();
        queue.write_buffer(&pane.buffer, 0, &bytes);
        if let (true, Some(source)) = (self.feedback, &self.custom_source) {
            let side = |points: f32| ((points * scale).round() as u32).clamp(1, MAX_FEEDBACK_SIDE);
            let size = (side(self.rect.width()), side(self.rect.height()));
            let Resources {
                feedback,
                feedback_pipelines,
                feedback_layout,
                sampler,
                ..
            } = resources;
            // A new size or new code starts again from black.
            if feedback
                .get(&self.terminal)
                .is_none_or(|pane| pane.size != size || pane.source != *source)
            {
                feedback.remove(&self.terminal);
                super::make_room(feedback, MAX_FEEDBACK_PANES, |pane| pane.last_used);
                feedback.insert(
                    self.terminal,
                    FeedbackPane::new(device, feedback_layout, sampler, size, source.clone()),
                );
            }
            let pane = feedback.get_mut(&self.terminal).expect("feedback frames");
            pane.last_used = now;
            queue.write_buffer(&pane.screen, 0, &bytes);
            // The frame has its own geometry: it starts at 0, 0 and is `size`.
            let mut offscreen = bytes.clone();
            for (index, value) in [0.0, 0.0, size.0 as f32, size.1 as f32]
                .into_iter()
                .enumerate()
            {
                offscreen[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
            }
            queue.write_buffer(&pane.offscreen, 0, &offscreen);
            let time = self.values[0];
            if let (Some((Some(pipeline), _)), true) =
                (feedback_pipelines.get(source), pane.drawn_at != Some(time))
            {
                pane.advance(encoder, pipeline);
                pane.drawn_at = Some(time);
            }
        }
        Vec::new()
    }

    fn paint(
        &self,
        _: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let Some(resources) = resources.get::<Resources>() else {
            return;
        };
        if self.feedback {
            let drawn = self.custom_source.as_ref().is_some_and(|source| {
                matches!(resources.feedback_pipelines.get(source), Some((Some(_), _)))
            });
            if let (true, Some(pane)) = (drawn, resources.feedback.get(&self.terminal)) {
                pane.show(pass, &resources.copy_pipeline);
            }
            return;
        }
        let Some(pane) = resources.panes.get(&self.terminal) else {
            return;
        };
        // A missing custom pipeline draws nothing: the preset pipeline would
        // show a different shader than the one asked for.
        let selected = match &self.custom_source {
            Some(source) => match resources.custom_pipelines.get(source) {
                Some((Some(pipeline), _)) => pipeline,
                // Refused by the driver, or not prepared yet.
                _ => return,
            },
            None => &resources.pipeline,
        };
        pass.set_pipeline(selected);
        pass.set_bind_group(0, &pane.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_editor_ranges_use_fallback_without_panicking() {
        let ctx = Context::default();
        register(&ctx, wgpu::TextureFormat::Rgba8Unorm);
        let mut shader = ShaderBackground {
            custom: Some(Default::default()),
            ..Default::default()
        };
        for (min, max) in [(8.0, 2.0), (1.0, 1.0), (f32::NAN, 2.0)] {
            let variable = &mut shader.custom.as_mut().unwrap().variables[0];
            variable.min = min;
            variable.max = max;
            let _ = ctx.run(Default::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    assert!(
                        callback(ui, ui.max_rect(), &shader, 1, 0.0, [Color32::BLACK; 6]).is_none()
                    );
                });
            });
        }
    }

    /// Draws every example in the AI shader guide and checks the picture
    /// check itself: good pictures pass, bad ones are named, and code the
    /// driver refuses is an error rather than a crash.
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "requires a graphics adapter; optional BUTTONSCLI_SHADER_GUIDE_DIR"]
    fn guide_examples_draw_and_bad_pictures_are_named() {
        let directory =
            std::env::var_os("BUTTONSCLI_SHADER_GUIDE_DIR").map(std::path::PathBuf::from);
        if let Some(directory) = &directory {
            std::fs::create_dir_all(directory).unwrap();
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await
                .expect("native graphics adapter");
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor::default(), None)
                .await
                .unwrap();
            let look = crate::shader_ai::Look::default();
            let custom = |code: &str| crate::theme::CustomShader {
                expression: code.into(),
                variables: Vec::new(),
                ..Default::default()
            };
            let problem = |code: &str, animated: bool| {
                picture_problem_on(
                    &device,
                    &queue,
                    &custom_source(&custom(code)).unwrap(),
                    &custom(code),
                    &look.background,
                    &look.colors,
                    animated,
                )
            };
            let examples = crate::shader_ai::guide_examples();
            assert!(examples.len() >= 5);
            let layout = uniform_layout(&device);
            for (name, code) in &examples {
                assert_eq!(problem(code, true), None, "{name}");
                let Some(directory) = &directory else {
                    continue;
                };
                let source = custom_source(&custom(code)).unwrap();
                let pipeline =
                    checked_pipeline(&device, wgpu::TextureFormat::Rgba8Unorm, &layout, &source)
                        .unwrap();
                for (suffix, strength) in [("", 1.0), ("-dim", 0.35)] {
                    let mut values = [0.0_f32; 36];
                    (values[2], values[3], values[4]) = (512.0, 288.0, 2.0);
                    (values[5], values[6]) = (strength, 1.0);
                    for (offset, color) in [8, 12, 16, 20, 28, 32]
                        .into_iter()
                        .zip(std::iter::once(&look.background).chain(look.colors.iter()))
                    {
                        let rgba = crate::theme::parse_color(color).unwrap().to_array();
                        for channel in 0..4 {
                            values[offset + channel] = f32::from(rgba[channel]) / 255.0;
                        }
                    }
                    let pixels =
                        render_pixels(&device, &queue, &layout, &pipeline, values, None).unwrap();
                    let file: String = name
                        .chars()
                        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                        .collect();
                    image::save_buffer(
                        directory.join(format!("{file}{suffix}.png")),
                        &pixels,
                        512,
                        288,
                        image::ColorType::Rgba8,
                    )
                    .unwrap();
                }
            }
            let named = |code: &str, animated: bool, word: &str| {
                let found = problem(code, animated).unwrap_or_default();
                assert!(found.contains(word), "{code}: {found:?}");
            };
            named("return c1;", true, "flat");
            named(
                "return mix(bg, c1, fill(sd_circle(p, 0.04)));",
                false,
                "hardly anything",
            );
            named(
                "return mix(c1, vec3<f32>(1.0), step(0.0, p.x));",
                false,
                "white",
            );
            let still = "return mix(bg, c1, 0.5 + 0.5 * sin(p.x * 4.0));";
            named(still, true, "nothing moved");
            assert_eq!(
                problem(still, false),
                None,
                "a still is fine when asked for"
            );
            assert!(checked_pipeline(
                &device,
                wgpu::TextureFormat::Rgba8Unorm,
                &layout,
                "fn broken("
            )
            .is_err());
        });
    }

    /// The real feedback path: frames drawn by `FeedbackPane::advance` and
    /// shown by the copy pipeline leave a trail behind a moving dot.
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "requires a graphics adapter"]
    fn feedback_frames_leave_trails_on_the_native_adapter() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await
                .expect("native graphics adapter");
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor::default(), None)
                .await
                .unwrap();
            device.push_error_scope(wgpu::ErrorFilter::Validation);
            let rules = super::super::shader_expression::Rules {
                feedback: true,
                ..Default::default()
            };
            let trail = crate::theme::CustomShader {
                expression: "let c = vec2<f32>(0.6 * sin(t), 0.0);\nlet dot = halo(sd_circle(p - c, 0.03), 0.05);\nreturn max(mix(bg, c1, clamp(dot, 0.0, 1.0)), prev(p) * 0.96);".into(),
                variables: Vec::new(),
                ..Default::default()
            };
            let source = super::super::shader_expression::program_with(&trail, rules).unwrap();
            let format = wgpu::TextureFormat::Rgba8Unorm;
            let resources = Resources::new(&device, format);
            let pipeline =
                checked_pipeline(&device, FEEDBACK_FORMAT, &resources.feedback_layout, &source)
                    .unwrap();
            let values = |time: f32| {
                let mut values = [0.0_f32; 36];
                (values[2], values[3], values[4]) = (64.0, 32.0, time);
                (values[5], values[6]) = (1.0, 1.0);
                // base dark blue, c1 bright orange.
                values[8..12].copy_from_slice(&[0.02, 0.02, 0.1, 1.0]);
                values[12..16].copy_from_slice(&[1.0, 0.6, 0.1, 1.0]);
                values.into_iter().flat_map(f32::to_le_bytes).collect::<Vec<u8>>()
            };
            // Draws frames at `times` into fresh feedback frames and shows the last.
            let frames = |times: &[f32]| {
                let mut pane = FeedbackPane::new(
                    &device,
                    &resources.feedback_layout,
                    &resources.sampler,
                    (64, 32),
                    source.clone(),
                );
                for time in times {
                    queue.write_buffer(&pane.offscreen, 0, &values(*time));
                    queue.write_buffer(&pane.screen, 0, &values(*time));
                    let mut encoder = device.create_command_encoder(&Default::default());
                    pane.advance(&mut encoder, &pipeline);
                    queue.submit([encoder.finish()]);
                }
                draw_and_read(&device, &queue, (64, 32), |encoder, view| {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    pane.show(&mut pass, &resources.copy_pipeline);
                })
                .unwrap()
            };
            let fresh = frames(&[1.5]);
            let trailed = frames(&[0.0, 0.5, 1.0, 1.5]);
            let bright = |pixels: &[u8]| {
                pixels
                    .chunks_exact(4)
                    .filter(|pixel| pixel[0] > 60)
                    .count()
            };
            assert!(
                bright(&trailed) > bright(&fresh) + 8,
                "earlier frames leave a trail: {} vs {}",
                bright(&trailed),
                bright(&fresh)
            );
            // The picture check draws a `prev()` shader too (from black).
            let look = crate::shader_ai::Look::default();
            let problem = picture_problem_on(
                &device,
                &queue,
                &source,
                &trail,
                &look.background,
                &look.colors,
                true,
            );
            assert!(
                !problem.as_deref().unwrap_or_default().contains("refused"),
                "{problem:?}"
            );
            assert!(
                device.pop_error_scope().await.is_none(),
                "feedback pipelines validate"
            );
        });
    }

    fn render_sample(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        resources: &Resources,
        preset: f32,
        time: f32,
    ) -> Vec<u8> {
        render_sample_with_knob(device, queue, resources, preset, time, 3.0)
    }

    fn render_sample_with_knob(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        resources: &Resources,
        preset: f32,
        time: f32,
        knob: f32,
    ) -> Vec<u8> {
        // geometry, settings, base, c1–c3, knobs, c4, c5 (the real uniform order).
        let mut values: [f32; 36] = [
            0.0, 0.0, 64.0, 32.0, time, 0.6, 1.0, preset, 0.02, 0.03, 0.05, 1.0, 0.1, 0.7, 0.5,
            1.0, 0.4, 0.1, 0.8, 1.0, 0.2, 0.3, 0.6, 1.0, 3.0, 0.0, 0.0, 0.0, 0.9, 0.5, 0.2, 1.0,
            0.3, 0.9, 0.4, 1.0,
        ];
        values[24] = knob;
        render_sample_with_values(device, queue, resources, values)
    }

    fn render_sample_with_values(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        resources: &Resources,
        values: [f32; 36],
    ) -> Vec<u8> {
        render_pixels(
            device,
            queue,
            &resources.layout,
            &resources.pipeline,
            values,
            None,
        )
        .expect("rendered sample")
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "requires a graphics adapter; optional BUTTONSCLI_PANE_FOCUS_PREVIEW_DIR"]
    fn pane_focus_shaders_render_on_native_adapter() {
        let directory =
            std::env::var_os("BUTTONSCLI_PANE_FOCUS_PREVIEW_DIR").map(std::path::PathBuf::from);
        if let Some(directory) = &directory {
            std::fs::create_dir_all(directory).unwrap();
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await
                .expect("native graphics adapter");
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor::default(), None)
                .await
                .unwrap();
            device.push_error_scope(wgpu::ErrorFilter::Validation);
            let catalog = crate::theme::ThemeCatalog::load();
            let mut extra = Vec::new();
            if let Some(pack) = std::env::var_os("BUTTONSCLI_PERSONAL_SHADER_PACK") {
                let pack: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(pack).unwrap()).unwrap();
                for item in pack["themes"].as_array().unwrap() {
                    let id = item["id"].as_str().unwrap();
                    extra.push(
                        crate::theme::parse_bundled_theme(id, &item["document"].to_string())
                            .unwrap(),
                    );
                }
            }
            let expected = 8 + extra.len();
            let mut count = 0;
            for theme in catalog
                .all()
                .iter()
                .filter(|t| t.id.starts_with("pane-focus-"))
                .chain(extra.iter())
            {
                count += 1;
                let shader = theme.effects.shader_background.as_ref().unwrap();
                let custom = shader.custom.as_ref().unwrap();
                let source = custom_source(custom).unwrap();
                let mut resources = Resources::new(&device, wgpu::TextureFormat::Rgba8Unorm);
                resources.pipeline = pipeline(
                    &device,
                    wgpu::TextureFormat::Rgba8Unorm,
                    &resources.layout,
                    &source,
                );
                let mut values = [0.0_f32; 36];
                values[2] = 320.0;
                values[3] = 160.0;
                values[5] = shader.strength;
                values[6] = shader.scale;
                let colors = shader.colors.as_ref().unwrap();
                for (offset, color) in [8, 12, 16, 20, 28, 32]
                    .into_iter()
                    .zip(std::iter::once(&theme.terminal_colors.background).chain(colors.iter()))
                {
                    let rgba = crate::theme::parse_color(color).unwrap().to_array();
                    for channel in 0..4 {
                        values[offset + channel] = f32::from(rgba[channel]) / 255.0;
                    }
                }
                for (index, variable) in custom.variables.iter().enumerate() {
                    values[24 + index] = variable.value;
                }
                let still = render_sample_with_values(&device, &queue, &resources, values);
                assert!(
                    still.chunks_exact(4).any(|pixel| pixel != &still[..4]),
                    "{} is flat",
                    theme.id
                );
                if let Some(directory) = &directory {
                    image::save_buffer(
                        directory.join(format!("{}.png", theme.id)),
                        &still,
                        320,
                        160,
                        image::ColorType::Rgba8,
                    )
                    .unwrap();
                }
                let luminance = |rgb: &[u8]| -> f32 {
                    rgb.iter()
                        .zip([0.2126, 0.7152, 0.0722])
                        .map(|(byte, weight)| {
                            let c = f32::from(*byte) / 255.0;
                            weight
                                * if c <= 0.04045 {
                                    c / 12.92
                                } else {
                                    ((c + 0.055) / 1.055).powf(2.4)
                                }
                        })
                        .sum()
                };
                let foreground =
                    crate::theme::parse_color(&theme.terminal_colors.foreground).unwrap();
                let foreground_luma = luminance(&foreground.to_array()[..3]);
                for seconds in [0.0, 19.0, 83.0] {
                    values[4] = if shader.animated {
                        seconds * shader.speed
                    } else {
                        0.0
                    };
                    let pixels = render_sample_with_values(&device, &queue, &resources, values);
                    for pixel in pixels.chunks_exact(4) {
                        assert_eq!(pixel[3], 255);
                        let contrast = (foreground_luma + 0.05) / (luminance(&pixel[..3]) + 0.05);
                        assert!(
                            contrast
                                >= if theme.id.starts_with("showcase-") {
                                    4.5
                                } else {
                                    7.0
                                },
                            "{} text contrast {contrast}",
                            theme.id
                        );
                    }
                    if seconds > 0.0 && shader.animated {
                        assert_ne!(pixels, still, "{} should animate", theme.id);
                    } else {
                        assert_eq!(pixels, still, "{} should stay still", theme.id);
                    }
                }
            }
            assert_eq!(count, expected);
            assert!(
                device.pop_error_scope().await.is_none(),
                "Pane Focus GPU validation"
            );
        });
    }

    /// Writes `<preset>.png` (480x270, strength 0.65) for docs and the website.
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "requires a graphics adapter; set BUTTONSCLI_SHADER_GALLERY_DIR"]
    fn render_shader_gallery() {
        let directory = std::path::PathBuf::from(
            std::env::var_os("BUTTONSCLI_SHADER_GALLERY_DIR").expect("output directory"),
        );
        std::fs::create_dir_all(&directory).unwrap();
        let (width, height) = (480_u32, 270_u32);
        let row = (width * 4).div_ceil(256) * 256;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            use wgpu::util::DeviceExt;
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await
                .expect("native graphics adapter");
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor::default(), None)
                .await
                .unwrap();
            let resources = Resources::new(&device, wgpu::TextureFormat::Rgba8Unorm);
            for (index, (id, _)) in ShaderBackground::PRESETS.iter().enumerate() {
                // Dark terminal base with the editor's pink/cyan/gold colors.
                let values: [f32; 36] = [
                    0.0,
                    0.0,
                    width as f32,
                    height as f32,
                    3.0,
                    0.65,
                    1.0,
                    index as f32,
                    0.03,
                    0.03,
                    0.06,
                    1.0,
                    1.0,
                    0.28,
                    0.48,
                    1.0,
                    0.21,
                    0.86,
                    1.0,
                    1.0,
                    1.0,
                    0.78,
                    0.34,
                    1.0,
                    3.0,
                    0.0,
                    0.0,
                    0.0,
                    0.49,
                    0.36,
                    1.0,
                    1.0,
                    0.24,
                    0.86,
                    0.52,
                    1.0,
                ];
                let bytes: Vec<u8> = values.into_iter().flat_map(f32::to_le_bytes).collect();
                let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: &bytes,
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &resources.layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform.as_entire_binding(),
                    }],
                });
                let size = wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                };
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                });
                let view = texture.create_view(&Default::default());
                let readback = device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: (row * height) as u64,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let mut encoder = device.create_command_encoder(&Default::default());
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    pass.set_pipeline(&resources.pipeline);
                    pass.set_bind_group(0, &bindings, &[]);
                    pass.draw(0..3, 0..1);
                }
                encoder.copy_texture_to_buffer(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyBufferInfo {
                        buffer: &readback,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(row),
                            rows_per_image: Some(height),
                        },
                    },
                    size,
                );
                queue.submit([encoder.finish()]);
                readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
                device.poll(wgpu::Maintain::Wait);
                let padded = readback.slice(..).get_mapped_range().to_vec();
                let pixels: Vec<u8> = padded
                    .chunks(row as usize)
                    .flat_map(|line| line[..(width * 4) as usize].to_vec())
                    .collect();
                image::save_buffer(
                    directory.join(format!("{index:02}-{id}.png")),
                    &pixels,
                    width,
                    height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        });
    }

    #[test]
    #[ignore = "requires a graphics adapter"]
    fn generated_style_bodies_render_real_pixels_on_the_native_adapter() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        // Bodies in the style the shader writer is asked for.
        let bodies = [
            "let s = stars(p * 12.0, t); return mix(c1, c2, 0.5 + 0.5 * sin(p.y * 6.0 + t)) + vec3<f32>(s);",
            "let n = cloud(p * 2.5 + vec2<f32>(t * 0.1, 0.0)); return palette(n + t * 0.05) * smoothstep(0.2, 0.8, n);",
            "let g = fract(p * 4.0) - 0.5; let d = length(g); return mix(c3, c1, step(d, 0.25 + 0.05 * sin(t)));",
            "return hsv(fract(atan2(p.y, p.x) / 6.28318 + t * 0.05), 0.6, 0.5);",
            "let w = noise(p * 3.0 + t * 0.2); return mix(c1, c2, w) * 0.8 + c3 * pow(w, 3.0);",
        ];
        runtime.block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await
                .expect("native graphics adapter");
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor::default(), None)
                .await
                .unwrap();
            device.push_error_scope(wgpu::ErrorFilter::Validation);
            for body in bodies {
                let custom = crate::theme::CustomShader {
                    expression: body.into(),
                    variables: crate::theme::CustomShader::default().variables,
                    ..Default::default()
                };
                let source =
                    custom_source(&custom).unwrap_or_else(|error| panic!("{body}: {error}"));
                let mut resources = Resources::new(&device, wgpu::TextureFormat::Rgba8Unorm);
                resources.pipeline = pipeline(
                    &device,
                    wgpu::TextureFormat::Rgba8Unorm,
                    &resources.layout,
                    &source,
                );
                let still = render_sample_with_knob(&device, &queue, &resources, 0.0, 0.0, 3.0);
                assert!(
                    still.chunks_exact(4).any(|pixel| pixel != &still[..4]),
                    "{body} renders a flat color"
                );
            }
            assert!(
                device.pop_error_scope().await.is_none(),
                "generated-style shaders failed GPU validation"
            );
        });
    }

    #[test]
    #[ignore = "requires a graphics adapter"]
    fn background_shader_pipeline_validates_on_the_native_adapter() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await
                .expect("native graphics adapter");
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor::default(), None)
                .await
                .unwrap();
            device.push_error_scope(wgpu::ErrorFilter::Validation);
            let resources = Resources::new(&device, wgpu::TextureFormat::Rgba8Unorm);
            let mut frames = Vec::new();
            for index in 0..ShaderBackground::PRESETS.len() {
                let preset = index as f32;
                let still = render_sample(&device, &queue, &resources, preset, 0.0);
                assert_eq!(
                    still,
                    render_sample(&device, &queue, &resources, preset, 0.0),
                    "still frames are deterministic"
                );
                assert_ne!(
                    still,
                    render_sample(&device, &queue, &resources, preset, 1.0),
                    "motion changes the actual GPU pixels"
                );
                assert!(still.chunks_exact(4).all(|pixel| pixel[3] == 255));
                assert!(
                    still.chunks_exact(4).any(|pixel| pixel != &still[..4]),
                    "background varies across the pane"
                );
                frames.push(still);
            }
            for (index, frame) in frames.iter().enumerate() {
                for other in &frames[index + 1..] {
                    assert_ne!(frame, other, "presets produce distinct GPU pixels");
                }
            }
            let mut custom_resources = Resources::new(&device, wgpu::TextureFormat::Rgba8Unorm);
            let source = custom_source(&crate::theme::CustomShader::default()).unwrap();
            custom_resources.pipeline = pipeline(
                &device,
                wgpu::TextureFormat::Rgba8Unorm,
                &custom_resources.layout,
                &source,
            );
            let first = render_sample_with_knob(&device, &queue, &custom_resources, 0.0, 0.0, 3.0);
            assert_ne!(
                first,
                render_sample_with_knob(&device, &queue, &custom_resources, 0.0, 0.0, 8.0),
                "custom variable changes actual GPU pixels"
            );
            assert_ne!(
                first,
                render_sample_with_knob(&device, &queue, &custom_resources, 0.0, 1.0, 3.0),
                "custom shader animates"
            );
            assert_eq!(
                first,
                render_sample_with_knob(&device, &queue, &custom_resources, 0.0, 0.0, 3.0)
            );
            assert!(
                device.pop_error_scope().await.is_none(),
                "shader background pipeline validation failed"
            );
        });
    }
}
