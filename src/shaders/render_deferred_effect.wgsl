struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) vertex_color: vec4<f32>,
    @location(2) tex_coords_0: vec2<f32>,
    @location(3) tex_coords_1: vec2<f32>,
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
    position: vec4<f32>,
    forward: vec4<f32>,
    right: vec4<f32>,
    up: vec4<f32>,
};

struct EffectUniform {
    vertex_layout_stride: u32,
    vertex_layout_position: i32,
    vertex_layout_normal: i32,
    vertex_layout_tangent_0: i32,
    vertex_layout_tangent_1: i32,
    vertex_layout_color: i32,
    vertex_layout_tex_coords_0: i32,
    vertex_layout_tex_coords_1: i32,

    sharpness: f32,
    vertex_color_enabled: i32,

    m0_diffuse_color_r: f32,
    m0_diffuse_color_g: f32,
    m0_diffuse_color_b: f32,
    m0_diffuse_texture_enabled: i32,
    m0_diffuse_texture_alpha_enabled: i32,
    m0_alpha_mask_enabled: i32,
    m1_enabled: i32,
    m1_diffuse_color_r: f32,
    m1_diffuse_color_g: f32,
    m1_diffuse_color_b: f32,
    m1_diffuse_texture_enabled: i32,
    m1_diffuse_texture_alpha_enabled: i32,
    m1_alpha_mask_enabled: i32, // always opposite of m0_alpha_mask_enabled? 
};

@group(0) @binding(0)
var <uniform> camera: CameraUniform;

@group(1) @binding(0)
var<storage, read> vertex_buffer: array<u32>;

@group(2) @binding(0)
var<uniform> effect: EffectUniform;

var<push_constant> model: mat4x4<f32>;

fn read_vec4(byte_offset: u32) -> vec4<f32> {
    var offset = byte_offset / 4;
    var x = bitcast<f32>(vertex_buffer[offset + 0]);
    var y = bitcast<f32>(vertex_buffer[offset + 1]);
    var z = bitcast<f32>(vertex_buffer[offset + 2]);
    var w = bitcast<f32>(vertex_buffer[offset + 3]);
    return vec4<f32>(x, y, z, w);
}

fn read_vec3(byte_offset: u32) -> vec3<f32> {
    var offset = byte_offset / 4;
    var x = bitcast<f32>(vertex_buffer[offset + 0]);
    var y = bitcast<f32>(vertex_buffer[offset + 1]);
    var z = bitcast<f32>(vertex_buffer[offset + 2]);
    return vec3<f32>(x, y, z);
}

fn read_vec2(byte_offset: u32) -> vec2<f32> {
    var offset = byte_offset / 4;
    var x = bitcast<f32>(vertex_buffer[offset + 0]);
    var y = bitcast<f32>(vertex_buffer[offset + 1]);
    return vec2<f32>(x, y);
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var base_offset = effect.vertex_layout_stride * vertex_index;

    var position = read_vec3(base_offset + bitcast<u32>(effect.vertex_layout_position));
    var normal = read_vec3(base_offset + bitcast<u32>(effect.vertex_layout_normal));
    var vertex_color = vec4<f32>(1.0);
    if effect.vertex_color_enabled != 0 {
        vertex_color = read_vec4(base_offset + bitcast<u32>(effect.vertex_layout_color));
    }
    var tex_coords_0 = read_vec2(base_offset + bitcast<u32>(effect.vertex_layout_tex_coords_0));
    var tex_coords_1 = vec2<f32>(0.0);
    if effect.m1_enabled != 0 {
        tex_coords_1 = read_vec2(base_offset + bitcast<u32>(effect.vertex_layout_tex_coords_1));
    }

    var out: VertexOutput;
    out.clip_position = camera.view_proj * model * vec4<f32>(position, 1.0);
    out.normal = normal;
    out.vertex_color = vertex_color;
    out.tex_coords_0 = tex_coords_0;
    out.tex_coords_1 = tex_coords_1;

    return out;
}

@group(3) @binding(0)
var diffuse_texture_0: texture_2d<f32>;
@group(3) @binding(1)
var diffuse_texture_1: texture_2d<f32>;
@group(3) @binding(2)
var texture_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var sharpness = effect.sharpness + 1;
    var half_sharpness = sharpness * 0.5;

    var diffuse_0 = vec4<f32>(1.0);
    if effect.m0_diffuse_texture_enabled != 0 {
        diffuse_0 = textureSample(diffuse_texture_0, texture_sampler, in.tex_coords_0);
        if effect.m0_diffuse_texture_alpha_enabled == 0 {
            diffuse_0.a = 1.0;
        }
    }
    diffuse_0 *= vec4<f32>(effect.m0_diffuse_color_r, effect.m0_diffuse_color_g, effect.m0_diffuse_color_b, 1.0);

    var diffuse_1 = vec4<f32>(0.0);
    if effect.m1_enabled != 0 {
        diffuse_1 = vec4<f32>(1.0);
        if effect.m1_diffuse_texture_enabled != 0 {
            diffuse_1 = textureSample(diffuse_texture_1, texture_sampler, in.tex_coords_1);
            if effect.m1_diffuse_texture_alpha_enabled == 0 {
                diffuse_1.a = 1.0;
            }
        }
        diffuse_1 *= vec4<f32>(effect.m1_diffuse_color_r, effect.m1_diffuse_color_g, effect.m1_diffuse_color_b, 1.0);
    }

    var blend_factor_0 = 0.0;
    var blend_factor_1 = 0.0;

    if effect.m0_alpha_mask_enabled != 0 {
        if effect.m1_alpha_mask_enabled != 0 {
            diffuse_1.a = 1.0 - diffuse_1.a;
            blend_factor_0 = max(diffuse_0.a, diffuse_1.a);
            diffuse_0.a = 1.0;
            diffuse_1.a = 1.0;
        } else {
            blend_factor_0 = diffuse_0.a;
            diffuse_0.a = 1.0;
        }
        blend_factor_1 = diffuse_1.a;
        diffuse_1.a = diffuse_0.a;
    } else {
        if effect.m1_alpha_mask_enabled != 0 {
            blend_factor_0 = 1.0 - diffuse_1.a;
            blend_factor_1 = 1.0;
        } else {
            blend_factor_1 = diffuse_1.a;
            blend_factor_0 = in.vertex_color.a;
        }
        diffuse_1.a = diffuse_0.a;
    }

    // TODO: approximate this curve with smoothstep or similar?
    var curve_in = in.vertex_color.a * 0.05 + 0.25;
    curve_in = saturate(curve_in * blend_factor_0 + (1.0 - in.vertex_color.a));

    var curve_center = curve_in - 0.5;
    var curve_out = 0.0;
    if curve_center >= 0.0 {
        var x = curve_in * 2.0;
        var y = curve_center * 2.0;
        var scaled_x = x * (x * (-0.5) + 0.55) - 0.4;
        curve_out = y * scaled_x + 0.1;
    } else {
        var x = curve_in * 2.0;
        var scaled_x = x * (x * 1.1 - 0.8) - 0.45;
        curve_out = x * scaled_x + 0.8;
    }
    curve_out = saturate(curve_out * 12.5);

    var sharpened = saturate(sharpness * curve_out - half_sharpness);
    var weight_1 = blend_factor_1 * sharpened;
    var weight_0 = 1.0 - weight_1;
    var diffuse_blended = diffuse_0.rgb * weight_0 + diffuse_1.rgb * weight_1;
    
    var output_alpha = diffuse_0.a * diffuse_1.a + weight_1;
    if output_alpha < 0.5 {
        discard;
    }

    // // TODO: skipped normal and material map stuff

    var output_0 = vec4<f32>(0.0);
    var output_1 = vec4<f32>(0.0);

    output_0 = vec4<f32>(diffuse_blended * in.vertex_color.rgb, 1.0);

    return output_0;
}
