struct InstanceInput {
    @location(0) position: vec3<f32>,
    @location(1) lifetime: f32,
    @location(2) size: f32,
    @location(3) rotation: f32,
    @location(4) sprite: u32,
    @location(5) additive: i32,
    @location(6) alpha: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec3<f32>,
    @location(1) sheet_index: u32,
    @location(2) additive: i32,
    @location(3) alpha: f32,
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
var textures: binding_array<texture_3d<f32>>;

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

    var corner_uv = corner * vec2<f32>(1.0, -1.0) + vec2<f32>(0.5);

    var sin_r = sin(in.rotation);
    var cos_r = cos(in.rotation);
    var rotated_corner = vec2<f32>(
        corner.x *        cos_r + corner.y * sin_r,
        corner.x * -1.0 * sin_r + corner.y * cos_r,
    );

    var aligned_corner = in.position 
                       + (camera.right.xyz * rotated_corner.x * in.size) 
                       + (camera.up.xyz    * rotated_corner.y * in.size);

    var sheet_index = in.sprite / 64;
    var sprite_index = in.sprite % 64;
    var sprite_y = sprite_index / 8;
    var sprite_x = sprite_index % 8;
    var sprite_uv = vec2<f32>(f32(sprite_x) / 8.0, f32(sprite_y) / 8.0) + (corner_uv / 8.0);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(aligned_corner, 1.0);
    out.tex_coords = vec3<f32>(sprite_uv, in.lifetime);
    out.sheet_index = sheet_index;
    out.additive = in.additive;
    out.alpha = in.alpha;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var sample = textureSample(textures[in.sheet_index], texture_sampler, in.tex_coords);
    
    if sample.a < 0.01 {
        discard;
    }

    var alpha = sample.a * in.alpha;

    var color = sample.rgb * alpha;
    color = 1 - exp2(color * -1.0);

    // 1.0 if additive, else 1.0 - a
    alpha = select(1.0 - alpha, 1.0, in.additive != 0);

    return vec4<f32>(color, alpha);
}