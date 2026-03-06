// ---- Background pipeline ----

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

// ---- Text pipeline ----

struct TextUniforms {
    viewport_size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> text_uniforms: TextUniforms;
@group(0) @binding(1) var atlas_texture: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

struct TextVertexInput {
    @location(0) vertex_pos: vec2<f32>,
    @location(1) glyph_pos: vec2<f32>,
    @location(2) glyph_size: vec2<f32>,
    @location(3) uv_origin: vec2<f32>,
    @location(4) uv_size: vec2<f32>,
    @location(5) color: vec4<f32>,
};

struct TextVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_text(in: TextVertexInput) -> TextVertexOutput {
    var out: TextVertexOutput;

    let pixel_pos = in.glyph_pos + in.vertex_pos * in.glyph_size;
    let clip_pos = vec2<f32>(
        (pixel_pos.x / text_uniforms.viewport_size.x) * 2.0 - 1.0,
        1.0 - (pixel_pos.y / text_uniforms.viewport_size.y) * 2.0,
    );

    out.clip_position = vec4<f32>(clip_pos, 0.0, 1.0);
    out.uv = in.uv_origin + in.vertex_pos * in.uv_size;
    out.color = in.color;
    return out;
}

@fragment
fn fs_text(in: TextVertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(atlas_texture, atlas_sampler, in.uv).r;
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
