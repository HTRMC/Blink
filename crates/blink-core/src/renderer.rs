use wasm_bindgen::prelude::*;
use wgpu::util::DeviceExt;
use crate::buffer::TextBuffer;
use crate::editor::Cursor;

/// Vertex for the background quad.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x4,
    ];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
}

impl Renderer {
    pub async fn new(canvas_id: &str) -> Result<Self, JsValue> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        // Get the canvas element
        let window = web_sys::window().ok_or("No window")?;
        let document = window.document().ok_or("No document")?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or("Canvas not found")?;
        let canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into()
            .map_err(|_| "Element is not a canvas")?;

        let width = canvas.client_width() as u32;
        let height = canvas.client_height() as u32;
        canvas.set_width(width);
        canvas.set_height(height);

        // Create surface from canvas
        let surface_target = wgpu::SurfaceTarget::Canvas(canvas);
        let surface = instance
            .create_surface(surface_target)
            .map_err(|e| JsValue::from_str(&format!("Failed to create surface: {e}")))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("No suitable GPU adapter found")?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Blink Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                memory_hints: wgpu::MemoryHints::Performance,
            }, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to get device: {e}")))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blink Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Blink Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Blink Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Initial fullscreen quad (editor background)
        let vertices = Self::create_background_vertices(width, height);
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        log::info!("Blink renderer initialized ({}x{})", width, height);

        Ok(Renderer {
            device,
            queue,
            surface,
            surface_config,
            pipeline,
            vertex_buffer,
        })
    }

    fn create_background_vertices(_width: u32, _height: u32) -> Vec<Vertex> {
        // Editor background - dark theme (#1e1e2e style)
        let bg_color = [0.118, 0.118, 0.180, 1.0];
        // Gutter/line-number area - slightly lighter
        let gutter_color = [0.145, 0.145, 0.210, 1.0];

        let gutter_width = -0.85; // ~7.5% of screen for gutter

        vec![
            // Gutter background (left strip) - two triangles
            Vertex { position: [-1.0, -1.0], color: gutter_color },
            Vertex { position: [gutter_width, -1.0], color: gutter_color },
            Vertex { position: [gutter_width, 1.0], color: gutter_color },
            Vertex { position: [-1.0, -1.0], color: gutter_color },
            Vertex { position: [gutter_width, 1.0], color: gutter_color },
            Vertex { position: [-1.0, 1.0], color: gutter_color },
            // Main editor background - two triangles
            Vertex { position: [gutter_width, -1.0], color: bg_color },
            Vertex { position: [1.0, -1.0], color: bg_color },
            Vertex { position: [1.0, 1.0], color: bg_color },
            Vertex { position: [gutter_width, -1.0], color: bg_color },
            Vertex { position: [1.0, 1.0], color: bg_color },
            Vertex { position: [gutter_width, 1.0], color: bg_color },
        ]
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);

            let vertices = Self::create_background_vertices(width, height);
            self.queue.write_buffer(
                &self.vertex_buffer,
                0,
                bytemuck::cast_slice(&vertices),
            );
        }
    }

    pub fn render(&mut self, _buffer: &TextBuffer, _cursor: &Cursor, _scroll_y: f32) {
        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(e) => {
                log::warn!("Failed to get surface texture: {:?}", e);
                return;
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.118,
                            g: 0.118,
                            b: 0.180,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..12, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
