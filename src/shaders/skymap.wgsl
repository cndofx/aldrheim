struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

struct SkymapUniform {
    texture_w: f32,
    texture_h: f32,
    target_w: f32,
    target_h: f32,
    color_r: f32,
    color_g: f32,
    color_b: f32,
};

@group(0) @binding(0)
var <uniform> uniform: SkymapUniform;

@group(1) @binding(0)
var texture: texture_2d<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var position = vec2<f32>(0.0);
    var tex_coords = vec2<f32>(0.0);
    if vertex_index == 0 {
        position = vec2<f32>(-1.0, -3.0);
        tex_coords = vec2<f32>(0.0, 2.0);
    } else if vertex_index == 1 {
        position = vec2<f32>(3.0, 1.0);
        tex_coords = vec2<f32>(2.0, 0.0);
    } else { // vertex_index == 2
        position = vec2<f32>(-1.0, 1.0);
        tex_coords = vec2<f32>(0.0, 0.0);
    }

    var texture_aspect = uniform.texture_w / uniform.texture_h;
    var target_aspect = uniform.target_w / uniform.target_h;
    var horizontal_scale = target_aspect / texture_aspect;
    tex_coords *= vec2<f32>(horizontal_scale, 1.0);

    var out: VertexOutput;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.tex_coords = tex_coords;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var sample = textureSample(texture, texture_sampler, in.tex_coords);
    var color = sample.rgb * vec3<f32>(uniform.color_r, uniform.color_g, uniform.color_b);
    return vec4<f32>(color, 1.0);
}
