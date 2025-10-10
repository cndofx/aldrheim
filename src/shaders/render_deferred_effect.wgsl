struct CameraUniform {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords_0: vec2<f32>,
    @location(3) tex_coords_1: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coords_0: vec2<f32>,
    @location(2) tex_coords_1: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.projection * camera.view * vec4<f32>(in.position, 1.0);
    out.normal = in.normal;
    out.tex_coords_0 = in.tex_coords_0;
    out.tex_coords_1 = in.tex_coords_1;
    return out;
}

// @group(0) @binding(0)
// var texture: texture_2d<f32>;
// @group(0) @binding(1)
// var texture_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // return textureSample(texture, texture_sampler, in.tex_coords);
    // return vec4<f32>(in.tex_coords, 1.0, 1.0);
    return vec4<f32>(in.normal, 1.0);
}