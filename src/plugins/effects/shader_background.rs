//! Bounded background patterns and validated resource-free color expressions.

use std::collections::HashMap;

use eframe::{egui_wgpu, wgpu};
use egui::{Context, Id, Rect, Ui};

use crate::theme::ShaderBackground;
use egui::Color32;

const FORMAT_ID: &str = "buttonscli-shader-background-format";
const MAX_BUFFERS: usize = 32;

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
    /// One use counter for every window; see [`super::make_room`].
    uses: u64,
}

impl Resources {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = uniform_layout(device);
        let pipeline = pipeline(device, format, &layout, super::shader_expression::SOURCE);
        Self {
            pipeline,
            layout,
            panes: HashMap::new(),
            custom_pipelines: HashMap::new(),
            uses: 0,
        }
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
    picture_problem_on(device, queue, custom, background, colors, animated)
}

#[cfg(not(target_arch = "wasm32"))]
fn picture_problem_on(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    custom: &crate::theme::CustomShader,
    background: &str,
    colors: &[String; 5],
    animated: bool,
) -> Option<String> {
    const WIDTH: usize = 64;
    const HEIGHT: usize = 32;
    let source = custom_source(custom).ok()?;
    let layout = uniform_layout(device);
    let pipeline = match checked_pipeline(device, wgpu::TextureFormat::Rgba8Unorm, &layout, &source)
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
    let first = render_pixels(device, queue, &layout, &pipeline, values)?;
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
        let second = render_pixels(device, queue, &layout, &pipeline, values)?;
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
#[cfg(not(target_arch = "wasm32"))]
fn render_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    pipeline: &wgpu::RenderPipeline,
    values: [f32; 36],
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
    let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout,
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
        pass.set_pipeline(pipeline);
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
        _: &mut wgpu::CommandEncoder,
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
        if let Some(source) = &self.custom_source {
            if !resources.custom_pipelines.contains_key(source) {
                super::make_room(&mut resources.custom_pipelines, MAX_BUFFERS, |(_, used)| {
                    *used
                });
                let built = checked_pipeline(device, self.format, &resources.layout, source);
                if let Err(error) = &built {
                    log::warn!("the graphics driver refused a custom shader: {error}");
                    if let Ok(mut refused) = REFUSED.lock() {
                        if refused.len() >= 16 {
                            refused.remove(0);
                        }
                        refused.push((source.clone(), error.clone()));
                    }
                }
                resources
                    .custom_pipelines
                    .insert(source.clone(), (built.ok(), now));
            }
            resources.custom_pipelines.get_mut(source).unwrap().1 = now;
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
                        render_pixels(&device, &queue, &layout, &pipeline, values).unwrap();
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
