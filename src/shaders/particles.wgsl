struct InstanceInput {
    @location(0) position: vec3<f32>,
    @location(1) lifetime: f32,
    @location(2) size: f32,
    @location(3) sprite: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) tex_coords: vec3<f32>,
    @location(2) sheet_mask: vec4<f32>,
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
    position: vec4<f32>,
    forward: vec4<f32>,
    right: vec4<f32>,
    up: vec4<f32>,
};

@group(0) @binding(0)
var <uniform> camera: CameraUniform;

@group(1) @binding(0)
var texture_sampler: sampler;
@group(1) @binding(1)
var texture_a: texture_3d<f32>;
@group(1) @binding(2)
var texture_b: texture_3d<f32>;
@group(1) @binding(3)
var texture_c: texture_3d<f32>;
@group(1) @binding(4)
var texture_d: texture_3d<f32>;

var<push_constant> model: mat4x4<f32>;

@vertex
fn vs_main(in: InstanceInput, @builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var corner = vec2<f32>(0.0);
    if vertex_index == 0 {
        corner = vec2<f32>(0.5, -0.5);
    } else if vertex_index == 1 {
        corner = vec2<f32>(-0.5, -0.5);
    } else if vertex_index == 2 {
        corner = vec2<f32>(0.5, 0.5);
    } else {
        corner = vec2<f32>(-0.5, 0.5);
    }
    var corner_pos = in.position + (camera.right.xyz * corner.x * in.size) + (camera.up.xyz * corner.y * in.size);

    // TODO: particle textures loop flipped vertically, but it looks worse when i correct them?
    // var corner_uv = corner * vec2<f32>(1.0, -1.0) + vec2<f32>(0.5);
    var corner_uv = corner + vec2<f32>(0.5);

    var sheet_index = in.sprite / 64;
    var sprite_index = in.sprite % 64;
    var sprite_y = sprite_index / 8;
    var sprite_x = sprite_index % 8;
    var sprite_uv = vec2<f32>(f32(sprite_x) / 8.0, f32(sprite_y) / 8.0) + (corner_uv / 8.0);

    var sheet_mask = vec4<f32>(
        select(0.0, 1.0, sheet_index == 0),
        select(0.0, 1.0, sheet_index == 1),
        select(0.0, 1.0, sheet_index == 2),
        select(0.0, 1.0, sheet_index == 3),
    );

    var out: VertexOutput;
    out.clip_position = camera.view_proj * model * vec4<f32>(corner_pos, 1.0);
    out.tex_coords = vec3<f32>(sprite_uv, in.lifetime);
    out.sheet_mask = sheet_mask;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var sample_a = textureSample(texture_a, texture_sampler, in.tex_coords);
    var sample_b = textureSample(texture_b, texture_sampler, in.tex_coords);
    var sample_c = textureSample(texture_c, texture_sampler, in.tex_coords);
    var sample_d = textureSample(texture_d, texture_sampler, in.tex_coords);

    var sample = sample_a * in.sheet_mask.x 
               + sample_b * in.sheet_mask.y 
               + sample_c * in.sheet_mask.z 
               + sample_d * in.sheet_mask.w;

    return sample;
}