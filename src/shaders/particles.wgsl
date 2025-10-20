struct InstanceInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
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

    var pos = in.position + (camera.right.xyz * corner.x) + (camera.up.xyz * corner.y);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * model * vec4<f32>(pos, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}