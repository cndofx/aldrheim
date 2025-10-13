struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) vertex_color: vec3<f32>,
    @location(2) diffuse_color_0: vec3<f32>,
    @location(3) tex_coords_0: vec2<f32>,
    @location(4) tex_coords_1: vec2<f32>,
};

struct MvpUniform {
    model: mat4x4<f32>,
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
};

struct EffectUniform {
    vertex_layout: VertexLayout,
    diffuse_color_0_r: f32,
    diffuse_color_0_g: f32,
    diffuse_color_0_b: f32,
    diffuse_texture_0_alpha_enabled: i32,
    diffuse_texture_1_alpha_enabled: i32,
    vertex_color_enabled: i32,
    has_material_1: i32,
    m0_alpha_mask_enabled: i32,
}

struct VertexLayout {
    stride: u32,
    position: i32,
    normal: i32,
    tangent_0: i32,
    tangent_1: i32,
    color: i32,
    tex_coords_0: i32,
    tex_coords_1: i32,
}

@group(0) @binding(0)
var<storage, read> vertex_buffer: array<u32>;

@group(1) @binding(0)
var<uniform> effect: EffectUniform;

var<push_constant> mvp: mat4x4<f32>;

// TODO: assumes 4 byte alignment
fn read_vec3(byte_offset: u32) -> vec3<f32> {
    var offset = byte_offset / 4;
    var x = bitcast<f32>(vertex_buffer[offset + 0]);
    var y = bitcast<f32>(vertex_buffer[offset + 1]);
    var z = bitcast<f32>(vertex_buffer[offset + 2]);
    return vec3<f32>(x, y, z);
}

// TODO: assumes 4 byte alignment
fn read_vec2(byte_offset: u32) -> vec2<f32> {
    var offset = byte_offset / 4;
    var x = bitcast<f32>(vertex_buffer[offset + 0]);
    var y = bitcast<f32>(vertex_buffer[offset + 1]);
    return vec2<f32>(x, y);
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var base_offset = effect.vertex_layout.stride * vertex_index;

    var position = read_vec3(base_offset + bitcast<u32>(effect.vertex_layout.position));
    var normal = read_vec3(base_offset + bitcast<u32>(effect.vertex_layout.normal));
    var vertex_color = vec3<f32>(1.0);
    if effect.vertex_color_enabled != 0 {
        vertex_color = read_vec3(base_offset + bitcast<u32>(effect.vertex_layout.color));
    }
    var tex_coords_0 = read_vec2(base_offset + bitcast<u32>(effect.vertex_layout.tex_coords_0));
    var tex_coords_1 = vec2<f32>(0.0);
    if effect.has_material_1 != 0 {
        tex_coords_1 = read_vec2(base_offset + bitcast<u32>(effect.vertex_layout.tex_coords_1));
    }

    var out: VertexOutput;
    out.clip_position = mvp * vec4<f32>(position, 1.0);
    out.normal = normal;
    out.vertex_color = vertex_color;
    out.tex_coords_0 = tex_coords_0;
    out.tex_coords_1 = tex_coords_1;
    out.diffuse_color_0 = vec3<f32>(effect.diffuse_color_0_r, effect.diffuse_color_0_g, effect.diffuse_color_0_b);

    return out;
}

@group(2) @binding(0)
var diffuse_texture_0: texture_2d<f32>;
@group(2) @binding(1)
var diffuse_texture_1: texture_2d<f32>;
@group(2) @binding(2)
var texture_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var diffuse_0 = textureSample(diffuse_texture_0, texture_sampler, in.tex_coords_0) * vec4<f32>(in.diffuse_color_0, 1.0);
    if effect.diffuse_texture_0_alpha_enabled == 0 {
        diffuse_0.a = 1.0;
    }

    var diffuse_1 = vec4<f32>(0.0);
    if effect.has_material_1 != 0 {
        diffuse_1 = textureSample(diffuse_texture_1, texture_sampler, in.tex_coords_1);
        if effect.diffuse_texture_1_alpha_enabled == 0 {
            diffuse_1.a = 1.0;
        }
    }
    
    // TODO: i have no idea if this is correct
    if effect.m0_alpha_mask_enabled != 0 {
        diffuse_1.a *= diffuse_0.a;
        diffuse_0.a = 1.0;
    }
    var diffuse_mixed = mix(diffuse_0, diffuse_1, diffuse_1.a);

    // TODO: proper blending instead of clipping transparent pixels
    if diffuse_mixed.a < 0.1 {
        discard;
    }

    return diffuse_mixed * vec4<f32>(in.vertex_color, 1.0);
}