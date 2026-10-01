//! Native port of the legacy analogStatic.ts overlay shader. This overlay
//! samples procedural noise only; it does not need a terminal image texture.

use std::collections::HashMap;

use eframe::{egui_wgpu, wgpu};
use egui::{Context, Id, Rect, Ui};

use crate::theme::TerminalEffects;

const FORMAT_ID: &str = "buttonscli-analog-static-format";
const MAX_BUFFERS: usize = 32;

pub(crate) fn register(ctx: &Context, format: wgpu::TextureFormat) {
    ctx.data_mut(|data| data.insert_temp(Id::new(FORMAT_ID), format));
}

pub(crate) fn paint(
    ui: &Ui,
    rect: Rect,
    effects: &TerminalEffects,
    terminal: u64,
    time: f32,
) -> bool {
    let Some(format) = ui
        .ctx()
        .data(|data| data.get_temp::<wgpu::TextureFormat>(Id::new(FORMAT_ID)))
    else {
        return false;
    };
    ui.painter().add(egui::Shape::Callback(
        egui_wgpu::Callback::new_paint_callback(
            rect,
            AnalogStatic {
                rect,
                format,
                terminal,
                frame: ui.ctx().cumulative_pass_nr(),
                values: [
                    time,
                    0.35 + effects.static_density * 1.4,
                    effects.static_amplitude,
                    effects.static_brightness,
                    effects.static_intensity,
                    effects.static_opacity,
                ],
            },
        ),
    ));
    true
}

struct AnalogStatic {
    rect: Rect,
    format: wgpu::TextureFormat,
    terminal: u64,
    frame: u64,
    values: [f32; 6],
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
            label: Some("analog static uniforms"),
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
            label: Some("analog static pipeline layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("analog static shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("analog_static.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("analog static screen overlay"),
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
                    // CSS screen blending: source + destination * (1-source).
                    // The shader premultiplies source by both legacy opacities.
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrc,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Zero,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
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

impl egui_wgpu::CallbackTrait for AnalogStatic {
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
            .expect("initialized analog static resources");
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
                label: Some("analog static pane"),
                size: 48,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("analog static pane bindings"),
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
            self.values[4],
            self.values[5],
            0.0,
            0.0,
        ];
        let bytes: Vec<u8> = values.into_iter().flat_map(f32::to_le_bytes).collect();
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

    #[test]
    #[ignore = "requires a graphics adapter"]
    fn analog_static_shader_pipeline_validates_on_the_native_adapter() {
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
            let (device, _) = adapter
                .request_device(&wgpu::DeviceDescriptor::default(), None)
                .await
                .unwrap();
            device.push_error_scope(wgpu::ErrorFilter::Validation);
            let _resources = Resources::new(&device, wgpu::TextureFormat::Bgra8Unorm);
            assert!(
                device.pop_error_scope().await.is_none(),
                "analog static pipeline validation failed"
            );
        });
    }
}
