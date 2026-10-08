//! Fixed, loop-free backgrounds; no imported source reaches the GPU.

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
    colors: [Color32; 4],
) -> Option<egui::PaintCallback> {
    if ui.input(|input| input.viewport().minimized.unwrap_or(false)) {
        return None;
    }
    let format = ui
        .ctx()
        .data(|data| data.get_temp::<wgpu::TextureFormat>(Id::new(FORMAT_ID)))?;
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shader background pipeline layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader background shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader_background.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
        });
        Self {
            pipeline,
            layout,
            panes: HashMap::new(),
        }
    }
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
                size: 96,
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
        pass.set_pipeline(&resources.pipeline);
        pass.set_bind_group(0, &pane.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_sample(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        resources: &Resources,
        preset: f32,
        time: f32,
    ) -> Vec<u8> {
        use wgpu::util::DeviceExt;
        let values: [f32; 24] = [
            0.0, 0.0, 64.0, 32.0, time, 0.6, 1.0, preset, 0.02, 0.03, 0.05, 1.0, 0.1, 0.7, 0.5,
            1.0, 0.4, 0.1, 0.8, 1.0, 0.2, 0.3, 0.6, 1.0,
        ];
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
            assert!(
                device.pop_error_scope().await.is_none(),
                "shader background pipeline validation failed"
            );
        });
    }
}
