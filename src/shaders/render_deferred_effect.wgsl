struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coords_0: vec2<f32>,
    @location(2) tex_coords_1: vec2<f32>,
};

struct MvpUniform {
    model: mat4x4<f32>,
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
};

struct VertexLayoutUniform {
    stride: u32,
    position: i32,
    normal: i32,
    tangent_0: i32,
    tangent_1: i32,
    color: i32,
    tex_coords_0: i32,
    tex_coords_1: i32,
    has_material_1: i32,
}

@group(0) @binding(0)
var<storage, read> vertex_buffer: array<u32>;

@group(1) @binding(0)
var<uniform> vertex_layout: VertexLayoutUniform;

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
    var base_offset = vertex_layout.stride * vertex_index;

    var position = read_vec3(base_offset + bitcast<u32>(vertex_layout.position));
    var normal = read_vec3(base_offset + bitcast<u32>(vertex_layout.normal));
    var tex_coords_0 = read_vec2(base_offset + bitcast<u32>(vertex_layout.tex_coords_0));
    var tex_coords_1 = vec2<f32>(0.0);
    if vertex_layout.has_material_1 != 0 {
        tex_coords_1 = read_vec2(base_offset + bitcast<u32>(vertex_layout.tex_coords_1));
    }

    var out: VertexOutput;
    out.clip_position = mvp * vec4<f32>(position, 1.0);
    out.normal = normal;
    out.tex_coords_0 = tex_coords_0;
    out.tex_coords_1 = tex_coords_1;

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
    return textureSample(diffuse_texture_0, texture_sampler, in.tex_coords_0);
}