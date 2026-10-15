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
    mut colors: [Color32; 4],
) -> Option<egui::PaintCallback> {
    if ui.input(|input| input.viewport().minimized.unwrap_or(false)) {
        return None;
    }
    let format = ui
        .ctx()
        .data(|data| data.get_temp::<wgpu::TextureFormat>(Id::new(FORMAT_ID)))?;
    if let Some(palette) = &shader.colors {
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
            frame: ui.ctx().cumulative_pass_nr(),
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
    frame: u64,
    values: [f32; 4],
    colors: [Color32; 4],
    knobs: [f32; 4],
    custom_source: Option<String>,
}

struct PaneUniform {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    last_frame: u64,
}

struct Resources {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    panes: HashMap<u64, PaneUniform>,
    custom_pipelines: HashMap<String, (wgpu::RenderPipeline, u64)>,
}

impl Resources {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        });
        let pipeline = pipeline(device, format, &layout, super::shader_expression::SOURCE);
        Self {
            pipeline,
            layout,
            panes: HashMap::new(),
            custom_pipelines: HashMap::new(),
        }
    }
}

fn custom_source(custom: &crate::theme::CustomShader) -> Result<String, String> {
    super::shader_expression::program(custom)
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
        if let Some(source) = &self.custom_source {
            if !resources.custom_pipelines.contains_key(source) {
                if resources.custom_pipelines.len() >= MAX_BUFFERS {
                    if let Some(oldest) = resources
                        .custom_pipelines
                        .iter()
                        .min_by_key(|(_, (_, frame))| *frame)
                        .map(|(source, _)| source.clone())
                    {
                        resources.custom_pipelines.remove(&oldest);
                    }
                }
                resources.custom_pipelines.insert(
                    source.clone(),
                    (
                        pipeline(device, self.format, &resources.layout, source),
                        self.frame,
                    ),
                );
            }
            resources.custom_pipelines.get_mut(source).unwrap().1 = self.frame;
        }
        if !resources.panes.contains_key(&self.terminal) {
            // Only tiny uniform buffers are cached, with a hard bound even
            // after many tab-close/reopen cycles. No pane image is retained.
            if resources.panes.len() >= MAX_BUFFERS {
                if let Some(oldest) = resources
                    .panes
                    .iter()
                    .min_by_key(|(_, pane)| pane.last_frame)
                    .map(|(id, _)| *id)
                {
                    resources.panes.remove(&oldest);
                }
            }
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("shader background pane"),
                size: 112,
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
                    last_frame: self.frame,
                },
            );
        }
        let pane = resources
            .panes
            .get_mut(&self.terminal)
            .expect("initialized pane uniforms");
        pane.last_frame = self.frame;
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
        let bytes: Vec<u8> = values
            .into_iter()
            .chain(self.colors.into_iter().flat_map(|color| {
                [
                    color.r() as f32 / 255.0,
                    color.g() as f32 / 255.0,
                    color.b() as f32 / 255.0,
                    1.0,
                ]
            }))
            .chain(self.knobs)
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
        let selected = self
            .custom_source
            .as_ref()
            .and_then(|source| resources.custom_pipelines.get(source))
            .map_or(&resources.pipeline, |(pipeline, _)| pipeline);
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
                        callback(ui, ui.max_rect(), &shader, 1, 0.0, [Color32::BLACK; 4]).is_none()
                    );
                });
            });
        }
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
        use wgpu::util::DeviceExt;
        let mut values: [f32; 28] = [
            0.0, 0.0, 64.0, 32.0, time, 0.6, 1.0, preset, 0.02, 0.03, 0.05, 1.0, 0.1, 0.7, 0.5,
            1.0, 0.4, 0.1, 0.8, 1.0, 0.2, 0.3, 0.6, 1.0, 3.0, 0.0, 0.0, 0.0,
        ];
        values[24] = knob;
        let bytes: Vec<u8> = values.into_iter().flat_map(f32::to_le_bytes).collect();
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("test background"),
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
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 64,
                height: 32,
                depth_or_array_layers: 1,
            },
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
            size: 8192,
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
                    bytes_per_row: Some(256),
                    rows_per_image: Some(32),
                },
            },
            wgpu::Extent3d {
                width: 64,
                height: 32,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        let (sender, receiver) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                sender.send(result).unwrap();
            });
        device.poll(wgpu::Maintain::Wait);
        receiver
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
            .unwrap();
        let output = readback.slice(..).get_mapped_range().to_vec();
        readback.unmap();
        output
    }

    /// Writes `<preset>.png` (480x270, max strength) for docs and the website.
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
                let values: [f32; 28] = [
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
