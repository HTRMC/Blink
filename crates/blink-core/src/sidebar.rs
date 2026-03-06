use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wgpu::util::DeviceExt;

use crate::font_atlas::FontAtlas;
use crate::icon_atlas::{Icon, IconAtlas};

// ---- GPU types (same layout as renderer.rs) ----

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

impl GlyphInstance {
    const ATTRIBS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        1 => Float32x2,
        2 => Float32x2,
        3 => Float32x2,
        4 => Float32x2,
        5 => Float32x4,
    ];
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct TextUniforms {
    viewport_size: [f32; 2],
    _pad: [f32; 2],
}

const MAX_INSTANCES: u64 = 8192;

const QUAD_VERTICES: [QuadVertex; 4] = [
    QuadVertex { position: [0.0, 0.0] },
    QuadVertex { position: [1.0, 0.0] },
    QuadVertex { position: [0.0, 1.0] },
    QuadVertex { position: [1.0, 1.0] },
];

const QUAD_INDICES: [u16; 6] = [0, 1, 2, 2, 1, 3];

// ---- Data types from JS ----

#[derive(Deserialize)]
pub struct SidebarEntry {
    pub name: String,
    pub depth: u32,
    pub is_dir: bool,
    pub expanded: bool,
    pub is_last: Vec<bool>,
}

// ---- Layout constants ----

const INDENT_WIDTH: f32 = 16.0;
const BASE_INDENT: f32 = 12.0;
const CHEVRON_CENTER: f32 = 7.0;
const PADDING_Y: f32 = 3.0;
const CHEVRON_SPACE: f32 = 14.0;

// ---- SidebarRenderer ----

#[wasm_bindgen]
pub struct SidebarRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,

    bg_pipeline: wgpu::RenderPipeline,
    bg_vertex_buffer: wgpu::Buffer,

    text_pipeline: wgpu::RenderPipeline,
    text_bind_group: wgpu::BindGroup,
    text_uniform_buffer: wgpu::Buffer,
    text_quad_vb: wgpu::Buffer,
    text_quad_ib: wgpu::Buffer,
    text_instance_buffer: wgpu::Buffer,
    text_instance_count: u32,

    atlas: FontAtlas,
    icon_atlas: IconAtlas,
    icon_bind_group: wgpu::BindGroup,
    icon_instance_buffer: wgpu::Buffer,
    icon_instance_count: u32,

    scroll_y: f32,
    viewport_width: f32,
    viewport_height: f32,
    hover_index: i32,
    guides_visible: bool,
}

#[wasm_bindgen]
impl SidebarRenderer {
    pub async fn create(
        canvas_id: &str,
        font_data: &[u8],
        device_pixel_ratio: f32,
    ) -> Result<SidebarRenderer, JsValue> {
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
            .map_err(|e| JsValue::from_str(&format!("Surface error: {e}")))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("No GPU adapter")?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Sidebar Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|e| JsValue::from_str(&format!("Device error: {e}")))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .iter()
            .find(|f| !f.is_srgb())
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

        // Font atlas
        let font_size = 12.0 * device_pixel_ratio;
        let atlas = FontAtlas::new(font_data, font_size);

        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Sidebar Atlas"),
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
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Icon atlas
        let icon_atlas = IconAtlas::new(device_pixel_ratio);

        let icon_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Sidebar Icon Atlas"),
            size: wgpu::Extent3d {
                width: icon_atlas.texture_width,
                height: icon_atlas.texture_height,
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
                texture: &icon_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &icon_atlas.texture_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(icon_atlas.texture_width),
                rows_per_image: Some(icon_atlas.texture_height),
            },
            wgpu::Extent3d {
                width: icon_atlas.texture_width,
                height: icon_atlas.texture_height,
                depth_or_array_layers: 1,
            },
        );

        let icon_view = icon_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let icon_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sidebar Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // BG pipeline
        let bg_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let bg_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sidebar BG"),
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

        let bg_vertices = Self::create_bg_vertices(width, height);
        let bg_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sidebar BG VB"),
            contents: bytemuck::cast_slice(&bg_vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // Text pipeline
        let text_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
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
            label: None,
            contents: bytemuck::cast_slice(&[TextUniforms {
                viewport_size: [width as f32, height as f32],
                _pad: [0.0, 0.0],
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let text_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
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
                label: None,
                bind_group_layouts: &[&text_bind_group_layout],
                push_constant_ranges: &[],
            });

        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sidebar Text"),
            layout: Some(&text_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_text"),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                    },
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
            label: None,
            contents: bytemuck::cast_slice(&QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let text_quad_ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let text_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: MAX_INSTANCES * std::mem::size_of::<GlyphInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Icon bind group (same layout, different texture)
        let icon_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Icon Bind Group"),
            layout: &text_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: text_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&icon_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&icon_sampler),
                },
            ],
        });

        let icon_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Icon Instance Buffer"),
            size: 1024 * std::mem::size_of::<GlyphInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(SidebarRenderer {
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
            icon_atlas,
            icon_bind_group,
            icon_instance_buffer,
            icon_instance_count: 0,
            scroll_y: 0.0,
            viewport_width: width as f32,
            viewport_height: height as f32,
            hover_index: -1,
            guides_visible: false,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.viewport_width = width as f32;
        self.viewport_height = height as f32;
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);

        let bg_vertices = Self::create_bg_vertices(width, height);
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

    pub fn set_hover(&mut self, index: i32) {
        self.hover_index = index;
    }

    pub fn set_guides_visible(&mut self, visible: bool) {
        self.guides_visible = visible;
    }

    pub fn set_scroll(&mut self, scroll_y: f32) {
        self.scroll_y = scroll_y;
    }

    pub fn row_height(&self) -> f32 {
        self.atlas.line_height + PADDING_Y * 2.0
    }

    /// Hit-test: returns the entry index at the given pixel coordinate, or -1.
    pub fn hit_test(&self, _x: f32, y: f32) -> i32 {
        let row_h = self.row_height();
        let index = ((y + self.scroll_y) / row_h) as i32;
        if index < 0 { -1 } else { index }
    }

    /// Render the sidebar file tree. `entries_js` is a JS array of SidebarEntry objects.
    pub fn render(&mut self, entries_js: JsValue) {
        let entries: Vec<SidebarEntry> = match serde_wasm_bindgen::from_value(entries_js) {
            Ok(e) => e,
            Err(err) => {
                log::error!("SidebarRenderer: failed to deserialize entries: {:?}", err);
                return;
            }
        };
        log::info!("SidebarRenderer: rendering {} entries, viewport {}x{}", entries.len(), self.viewport_width, self.viewport_height);

        let (text_instances, icon_instances) = self.build_instances(&entries);
        let instance_count = (text_instances.len() as u64).min(MAX_INSTANCES) as u32;
        self.text_instance_count = instance_count;

        if instance_count > 0 {
            self.queue.write_buffer(
                &self.text_instance_buffer,
                0,
                bytemuck::cast_slice(&text_instances[..instance_count as usize]),
            );
        }

        let icon_count = (icon_instances.len() as u64).min(1024) as u32;
        self.icon_instance_count = icon_count;
        log::info!("SidebarRenderer: text={}, icons={}", instance_count, icon_count);
        web_sys::console::log_1(&format!("DIRECT: text={}, icons={}", instance_count, icon_count).into());

        if icon_count > 0 {
            self.queue.write_buffer(
                &self.icon_instance_buffer,
                0,
                bytemuck::cast_slice(&icon_instances[..icon_count as usize]),
            );
        }

        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => return,
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.078,
                            g: 0.078,
                            b: 0.078,
                            a: 1.0,
                        }), // #141414
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // BG
            pass.set_pipeline(&self.bg_pipeline);
            pass.set_vertex_buffer(0, self.bg_vertex_buffer.slice(..));
            pass.draw(0..6, 0..1);

            // Text
            if self.text_instance_count > 0 {
                pass.set_pipeline(&self.text_pipeline);
                pass.set_bind_group(0, &self.text_bind_group, &[]);
                pass.set_vertex_buffer(0, self.text_quad_vb.slice(..));
                pass.set_vertex_buffer(1, self.text_instance_buffer.slice(..));
                pass.set_index_buffer(self.text_quad_ib.slice(..), wgpu::IndexFormat::Uint16);
                pass.draw_indexed(0..6, 0, 0..self.text_instance_count);
            }

            // Icons
            if self.icon_instance_count > 0 {
                pass.set_bind_group(0, &self.icon_bind_group, &[]);
                pass.set_vertex_buffer(1, self.icon_instance_buffer.slice(..));
                pass.draw_indexed(0..6, 0, 0..self.icon_instance_count);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    fn create_bg_vertices(_width: u32, _height: u32) -> Vec<Vertex> {
        let bg = [0.078, 0.078, 0.078, 1.0]; // #141414
        vec![
            Vertex { position: [-1.0, -1.0], color: bg },
            Vertex { position: [1.0, -1.0], color: bg },
            Vertex { position: [1.0, 1.0], color: bg },
            Vertex { position: [-1.0, -1.0], color: bg },
            Vertex { position: [1.0, 1.0], color: bg },
            Vertex { position: [-1.0, 1.0], color: bg },
        ]
    }

    fn build_instances(&self, entries: &[SidebarEntry]) -> (Vec<GlyphInstance>, Vec<GlyphInstance>) {
        let mut text_instances = Vec::new();
        let mut icon_instances = Vec::new();
        let solid = self.atlas.solid_uv();
        let row_h = self.row_height();
        let text_color = [0.525, 0.525, 0.525, 1.0]; // #868686
        let hover_bg = [1.0, 1.0, 1.0, 0.05];
        let guide_color = [0.137, 0.137, 0.137, 1.0]; // #232323
        let chevron_color = [0.533, 0.533, 0.533, 1.0]; // #888

        let visible_start = (self.scroll_y / row_h) as usize;
        let visible_count = (self.viewport_height / row_h) as usize + 2;

        for (i, entry) in entries
            .iter()
            .enumerate()
            .skip(visible_start)
            .take(visible_count)
        {
            let row_y = i as f32 * row_h - self.scroll_y;
            let indent = BASE_INDENT + entry.depth as f32 * INDENT_WIDTH;

            // Hover highlight
            if i as i32 == self.hover_index {
                text_instances.push(GlyphInstance {
                    glyph_pos: [0.0, row_y],
                    glyph_size: [self.viewport_width, row_h],
                    uv_origin: [solid[0], solid[1]],
                    uv_size: [solid[2], solid[3]],
                    color: hover_bg,
                });
            }

            // Hierarchy guide lines
            if self.guides_visible {
                for (d, &active) in entry.is_last.iter().enumerate() {
                    if !active {
                        let guide_x = BASE_INDENT + d as f32 * INDENT_WIDTH + CHEVRON_CENTER;
                        text_instances.push(GlyphInstance {
                            glyph_pos: [guide_x, row_y],
                            glyph_size: [1.0, row_h],
                            uv_origin: [solid[0], solid[1]],
                            uv_size: [solid[2], solid[3]],
                            color: guide_color,
                        });
                    }
                }
            }

            let baseline_y = row_y + PADDING_Y + self.atlas.ascent;

            // Chevron icon for directories
            if entry.is_dir {
                let icon = if entry.expanded { Icon::ChevronDown } else { Icon::ChevronRight };
                if let Some(info) = self.icon_atlas.get(icon) {
                    let icon_x = indent + (CHEVRON_SPACE - info.width) / 2.0;
                    let icon_y = row_y + (row_h - info.height) / 2.0;
                    icon_instances.push(GlyphInstance {
                        glyph_pos: [icon_x, icon_y],
                        glyph_size: [info.width, info.height],
                        uv_origin: [info.uv_x, info.uv_y],
                        uv_size: [info.uv_w, info.uv_h],
                        color: chevron_color,
                    });
                }
            }

            // File/folder name (proportional positioning)
            let mut cursor_x = indent + CHEVRON_SPACE;
            for ch in entry.name.chars() {
                if let Some(glyph) = self.atlas.glyphs.get(&ch) {
                    if glyph.width > 0.0 && glyph.height > 0.0 {
                        text_instances.push(GlyphInstance {
                            glyph_pos: [
                                (cursor_x + glyph.offset_x).round(),
                                (baseline_y - glyph.offset_y - glyph.height).round(),
                            ],
                            glyph_size: [glyph.width, glyph.height],
                            uv_origin: [glyph.uv_x, glyph.uv_y],
                            uv_size: [glyph.uv_w, glyph.uv_h],
                            color: text_color,
                        });
                    }
                    cursor_x += glyph.advance_width;
                }
            }
        }

        (text_instances, icon_instances)
    }
}
