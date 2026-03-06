use wasm_bindgen::prelude::*;
use wgpu::util::DeviceExt;

use crate::buffer::TextBuffer;
use crate::editor::Cursor;
use crate::font_atlas::FontAtlas;

// ---- Background pipeline types ----

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

// ---- Text pipeline types ----

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct QuadVertex {
    position: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GlyphInstance {
    glyph_pos: [f32; 2],
    glyph_size: [f32; 2],
    uv_origin: [f32; 2],
    uv_size: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct TextUniforms {
    viewport_size: [f32; 2],
    _pad: [f32; 2],
}

const MAX_INSTANCES: u64 = 16384;

const QUAD_VERTICES: [QuadVertex; 4] = [
    QuadVertex { position: [0.0, 0.0] },
    QuadVertex { position: [1.0, 0.0] },
    QuadVertex { position: [0.0, 1.0] },
    QuadVertex { position: [1.0, 1.0] },
];

const QUAD_INDICES: [u16; 6] = [0, 1, 2, 2, 1, 3];

impl GlyphInstance {
    const ATTRIBS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        1 => Float32x2,
        2 => Float32x2,
        3 => Float32x2,
        4 => Float32x2,
        5 => Float32x4,
    ];
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,

    // Background
    bg_pipeline: wgpu::RenderPipeline,
    bg_vertex_buffer: wgpu::Buffer,

    // Text
    text_pipeline: wgpu::RenderPipeline,
    text_bind_group: wgpu::BindGroup,
    text_uniform_buffer: wgpu::Buffer,
    text_quad_vb: wgpu::Buffer,
    text_quad_ib: wgpu::Buffer,
    text_instance_buffer: wgpu::Buffer,
    text_instance_count: u32,

    // Font
    atlas: FontAtlas,
    gutter_width: f32,
}

impl Renderer {
    pub async fn new(canvas_id: &str, font_data: &[u8]) -> Result<Self, JsValue> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        let window = web_sys::window().ok_or("No window")?;
        let document = window.document().ok_or("No document")?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or("Canvas not found")?;
        let canvas: web_sys::HtmlCanvasElement =
            canvas.dyn_into().map_err(|_| "Element is not a canvas")?;

        let width = canvas.client_width().max(1) as u32;
        let height = canvas.client_height().max(1) as u32;
        canvas.set_width(width);
        canvas.set_height(height);

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
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Blink Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
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

        // ---- Font atlas ----

        let font_size = 14.0;
        let atlas = FontAtlas::new(font_data, font_size);
        let gutter_width = (atlas.cell_width * 5.0 + 16.0).ceil();

        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Glyph Atlas"),
            size: wgpu::Extent3d {
                width: atlas.texture_width,
                height: atlas.texture_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas.texture_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(atlas.texture_width),
                rows_per_image: Some(atlas.texture_height),
            },
            wgpu::Extent3d {
                width: atlas.texture_width,
                height: atlas.texture_height,
                depth_or_array_layers: 1,
            },
        );

        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // ---- Shader ----

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blink Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // ---- Background pipeline ----

        let bg_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("BG Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let bg_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("BG Pipeline"),
            layout: Some(&bg_pipeline_layout),
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
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let bg_vertices = Self::create_background_vertices(width, height, gutter_width);
        let bg_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BG Vertex Buffer"),
            contents: bytemuck::cast_slice(&bg_vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // ---- Text pipeline ----

        let text_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Text BG Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
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
            });

        let text_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Text Uniforms"),
            contents: bytemuck::cast_slice(&[TextUniforms {
                viewport_size: [width as f32, height as f32],
                _pad: [0.0, 0.0],
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let text_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Text Bind Group"),
            layout: &text_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: text_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                },
            ],
        });

        let text_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Text Pipeline Layout"),
                bind_group_layouts: &[&text_bind_group_layout],
                push_constant_ranges: &[],
            });

        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Text Pipeline"),
            layout: Some(&text_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_text"),
                buffers: &[
                    // Slot 0: quad vertices (per-vertex)
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                    },
                    // Slot 1: glyph instances (per-instance)
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<GlyphInstance>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &GlyphInstance::ATTRIBS,
                    },
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_text"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let text_quad_vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad VB"),
            contents: bytemuck::cast_slice(&QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let text_quad_ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad IB"),
            contents: bytemuck::cast_slice(&QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let text_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Glyph Instances"),
            size: MAX_INSTANCES * std::mem::size_of::<GlyphInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        log::info!(
            "Blink renderer initialized ({}x{}, atlas {}x{}, cell {:.1}x{:.1})",
            width,
            height,
            atlas.texture_width,
            atlas.texture_height,
            atlas.cell_width,
            atlas.line_height,
        );

        Ok(Renderer {
            device,
            queue,
            surface,
            surface_config,
            bg_pipeline,
            bg_vertex_buffer,
            text_pipeline,
            text_bind_group,
            text_uniform_buffer,
            text_quad_vb,
            text_quad_ib,
            text_instance_buffer,
            text_instance_count: 0,
            atlas,
            gutter_width,
        })
    }

    fn create_background_vertices(width: u32, height: u32, gutter_px: f32) -> Vec<Vertex> {
        let bg_color = [0.118, 0.118, 0.180, 1.0];
        let gutter_color = [0.145, 0.145, 0.210, 1.0];

        // Convert gutter pixel width to clip space
        let gutter_clip = (gutter_px / width as f32) * 2.0 - 1.0;

        vec![
            // Gutter
            Vertex { position: [-1.0, -1.0], color: gutter_color },
            Vertex { position: [gutter_clip, -1.0], color: gutter_color },
            Vertex { position: [gutter_clip, 1.0], color: gutter_color },
            Vertex { position: [-1.0, -1.0], color: gutter_color },
            Vertex { position: [gutter_clip, 1.0], color: gutter_color },
            Vertex { position: [-1.0, 1.0], color: gutter_color },
            // Editor background
            Vertex { position: [gutter_clip, -1.0], color: bg_color },
            Vertex { position: [1.0, -1.0], color: bg_color },
            Vertex { position: [1.0, 1.0], color: bg_color },
            Vertex { position: [gutter_clip, -1.0], color: bg_color },
            Vertex { position: [1.0, 1.0], color: bg_color },
            Vertex { position: [gutter_clip, 1.0], color: bg_color },
        ]
    }

    fn build_glyph_instances(
        &self,
        buffer: &TextBuffer,
        cursor: &Cursor,
        scroll_y: f32,
        selection: Option<(usize, usize)>,
        total_content_height: f32,
    ) -> Vec<GlyphInstance> {
        let mut instances = Vec::new();
        let lines = buffer.lines();
        let padding = 8.0;
        let text_start_x = self.gutter_width + padding;

        let line_num_color = [0.42, 0.44, 0.53, 1.0];
        let text_color = [0.80, 0.84, 0.96, 1.0];
        let cursor_color = [0.80, 0.84, 0.96, 0.9];
        let current_line_color = [1.0, 1.0, 1.0, 0.04];
        let selection_color = [0.34, 0.42, 0.68, 0.45];

        let visible_start = (scroll_y / self.atlas.line_height) as usize;
        let visible_count =
            (self.surface_config.height as f32 / self.atlas.line_height) as usize + 2;
        let solid = self.atlas.solid_uv();

        // Track byte offset at start of each line for selection rendering
        let mut line_byte_offset: usize = 0;

        for (i, line) in lines
            .iter()
            .enumerate()
            .skip(visible_start)
            .take(visible_count)
        {
            // Compute the byte offset for this line
            line_byte_offset = buffer.line_start_offset(i);
            let line_end_offset = line_byte_offset + line.len();
            let row_y = i as f32 * self.atlas.line_height - scroll_y;
            let baseline_y = row_y + self.atlas.ascent;

            // Current line highlight (only when no selection)
            if i == cursor.line && selection.is_none() {
                instances.push(GlyphInstance {
                    glyph_pos: [self.gutter_width, row_y],
                    glyph_size: [self.surface_config.width as f32 - self.gutter_width, self.atlas.line_height],
                    uv_origin: [solid[0], solid[1]],
                    uv_size: [solid[2], solid[3]],
                    color: current_line_color,
                });
            }

            // Selection highlight for this line
            if let Some((sel_start, sel_end)) = selection {
                if sel_start < line_end_offset + 1 && sel_end > line_byte_offset {
                    let col_start = if sel_start > line_byte_offset {
                        sel_start - line_byte_offset
                    } else {
                        0
                    };
                    let col_end = if sel_end < line_end_offset {
                        sel_end - line_byte_offset
                    } else {
                        // Extend selection to include the newline character visually
                        line.len() + if sel_end > line_end_offset { 1 } else { 0 }
                    };

                    if col_end > col_start {
                        let sel_x = text_start_x + col_start as f32 * self.atlas.cell_width;
                        let sel_w = (col_end - col_start) as f32 * self.atlas.cell_width;
                        instances.push(GlyphInstance {
                            glyph_pos: [sel_x, row_y],
                            glyph_size: [sel_w, self.atlas.line_height],
                            uv_origin: [solid[0], solid[1]],
                            uv_size: [solid[2], solid[3]],
                            color: selection_color,
                        });
                    }
                }
            }

            // Line number
            let line_num = format!("{:>4}", i + 1);
            for (j, ch) in line_num.chars().enumerate() {
                if let Some(glyph) = self.atlas.glyphs.get(&ch) {
                    if glyph.width > 0.0 && glyph.height > 0.0 {
                        instances.push(GlyphInstance {
                            glyph_pos: [
                                padding + j as f32 * self.atlas.cell_width + glyph.offset_x,
                                baseline_y - glyph.offset_y - glyph.height,
                            ],
                            glyph_size: [glyph.width, glyph.height],
                            uv_origin: [glyph.uv_x, glyph.uv_y],
                            uv_size: [glyph.uv_w, glyph.uv_h],
                            color: if i == cursor.line {
                                text_color
                            } else {
                                line_num_color
                            },
                        });
                    }
                }
            }

            // Text content
            for (j, ch) in line.chars().enumerate() {
                if let Some(glyph) = self.atlas.glyphs.get(&ch) {
                    if glyph.width > 0.0 && glyph.height > 0.0 {
                        instances.push(GlyphInstance {
                            glyph_pos: [
                                text_start_x + j as f32 * self.atlas.cell_width + glyph.offset_x,
                                baseline_y - glyph.offset_y - glyph.height,
                            ],
                            glyph_size: [glyph.width, glyph.height],
                            uv_origin: [glyph.uv_x, glyph.uv_y],
                            uv_size: [glyph.uv_w, glyph.uv_h],
                            color: text_color,
                        });
                    }
                }
            }
        }

        // Cursor (thin vertical bar)
        let cursor_row_y = cursor.line as f32 * self.atlas.line_height - scroll_y;
        instances.push(GlyphInstance {
            glyph_pos: [
                text_start_x + cursor.col as f32 * self.atlas.cell_width,
                cursor_row_y,
            ],
            glyph_size: [2.0, self.atlas.line_height],
            uv_origin: [solid[0], solid[1]],
            uv_size: [solid[2], solid[3]],
            color: cursor_color,
        });

        // Scrollbar
        let viewport_h = self.surface_config.height as f32;
        let viewport_w = self.surface_config.width as f32;
        if total_content_height > viewport_h {
            let scrollbar_width = 8.0;
            let scrollbar_x = viewport_w - scrollbar_width;

            // Track background
            let track_color = [1.0, 1.0, 1.0, 0.03];
            instances.push(GlyphInstance {
                glyph_pos: [scrollbar_x, 0.0],
                glyph_size: [scrollbar_width, viewport_h],
                uv_origin: [solid[0], solid[1]],
                uv_size: [solid[2], solid[3]],
                color: track_color,
            });

            // Thumb
            let thumb_ratio = viewport_h / total_content_height;
            let thumb_h = (thumb_ratio * viewport_h).max(20.0);
            let scroll_ratio = scroll_y / (total_content_height - viewport_h);
            let thumb_y = scroll_ratio * (viewport_h - thumb_h);
            let thumb_color = [1.0, 1.0, 1.0, 0.15];
            instances.push(GlyphInstance {
                glyph_pos: [scrollbar_x, thumb_y],
                glyph_size: [scrollbar_width, thumb_h],
                uv_origin: [solid[0], solid[1]],
                uv_size: [solid[2], solid[3]],
                color: thumb_color,
            });
        }

        instances
    }

    pub fn cell_width(&self) -> f32 {
        self.atlas.cell_width
    }

    pub fn line_height(&self) -> f32 {
        self.atlas.line_height
    }

    pub fn gutter_width(&self) -> f32 {
        self.gutter_width
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);

        let bg_vertices = Self::create_background_vertices(width, height, self.gutter_width);
        self.queue.write_buffer(
            &self.bg_vertex_buffer,
            0,
            bytemuck::cast_slice(&bg_vertices),
        );

        self.queue.write_buffer(
            &self.text_uniform_buffer,
            0,
            bytemuck::cast_slice(&[TextUniforms {
                viewport_size: [width as f32, height as f32],
                _pad: [0.0, 0.0],
            }]),
        );
    }

    pub fn render(
        &mut self,
        buffer: &TextBuffer,
        cursor: &Cursor,
        scroll_y: f32,
        selection: Option<(usize, usize)>,
    ) {
        let total_content_height = buffer.line_count() as f32 * self.atlas.line_height;
        let instances =
            self.build_glyph_instances(buffer, cursor, scroll_y, selection, total_content_height);
        let instance_count = (instances.len() as u64).min(MAX_INSTANCES) as u32;
        self.text_instance_count = instance_count;

        if instance_count > 0 {
            self.queue.write_buffer(
                &self.text_instance_buffer,
                0,
                bytemuck::cast_slice(&instances[..instance_count as usize]),
            );
        }

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
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

            // Background
            pass.set_pipeline(&self.bg_pipeline);
            pass.set_vertex_buffer(0, self.bg_vertex_buffer.slice(..));
            pass.draw(0..12, 0..1);

            // Text
            if self.text_instance_count > 0 {
                pass.set_pipeline(&self.text_pipeline);
                pass.set_bind_group(0, &self.text_bind_group, &[]);
                pass.set_vertex_buffer(0, self.text_quad_vb.slice(..));
                pass.set_vertex_buffer(1, self.text_instance_buffer.slice(..));
                pass.set_index_buffer(self.text_quad_ib.slice(..), wgpu::IndexFormat::Uint16);
                pass.draw_indexed(0..6, 0, 0..self.text_instance_count);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
