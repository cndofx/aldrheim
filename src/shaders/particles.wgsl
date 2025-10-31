struct InstanceInput {
    @location(0) position: vec3<f32>,
    @location(1) lifetime: f32,
    @location(2) size: f32,
    @location(3) rotation: f32,
    @location(4) sprite: u32,
    @location(5) additive: i32,
    @location(6) hsv: i32,
    @location(7) colorize: i32,
    @location(8) hue_rotation: f32,
    @location(9) saturation: f32,
    @location(10) value: f32,
    @location(11) alpha: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec3<f32>,
    @location(1) sheet_index: u32,
    @location(2) additive: i32,
    @location(3) hsv: i32,
    @location(4) colorize: i32,
    @location(5) hue_rotation: f32,
    @location(6) saturation: f32,
    @location(7) value: f32,
    @location(8) alpha: f32,
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
    out.hsv = in.hsv;
    out.colorize = in.colorize;
    out.hue_rotation = in.hue_rotation;
    out.saturation = in.saturation;
    out.value = in.value;
    out.alpha = in.alpha;
    // out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var sample = textureSample(textures[in.sheet_index], texture_sampler, in.tex_coords);
    if sample.a < 0.01 {
        discard;
    }

    var color = sample.rgb;
    if in.colorize != 0 && in.hsv == 0 {
        var luminance = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
        color = vec3<f32>(luminance);
    }
    if in.colorize != 0 || in.hsv != 0 {
        color = apply_hue_saturation(color, in.hue_rotation, in.saturation, in.colorize != 0);
    }

    var alpha = sample.a * in.alpha;
    color *= alpha;
    color = 1 - exp2(color * -1.0);
    alpha = select(1.0 - alpha, 1.0, in.additive != 0);
    return vec4<f32>(color, alpha);
}

// based on https://gist.github.com/mairod/a75e7b44f68110e1576d77419d608786
fn apply_hue_saturation(color: vec3<f32>, hue_rotation: f32, saturation: f32, colorize: bool) -> vec3<f32> {
    const rgb_to_y_prime = vec3<f32>(0.299, 0.587, 0.114);
    const rgb_to_i = vec3<f32>(0.596, -0.275, -0.321);
    const rgb_to_q = vec3<f32>(0.212, -0.523, 0.311);

    const yiq_to_r = vec3<f32>(1.0, 0.956, 0.621);
    const yiq_to_g = vec3<f32>(1.0, -0.272, -0.647);
    const yiq_to_b = vec3<f32>(1.0, -1.107, 1.704);

    var y_prime = dot(color, rgb_to_y_prime);
    var i = dot(color, rgb_to_i);
    var q = dot(color, rgb_to_q);

    var hue = 0.0;
    var chroma = 0.0;
    if colorize {
        hue = hue_rotation;
        chroma = y_prime * saturation;
    } else {
        hue = atan2(q, i) + hue_rotation;
        chroma = sqrt(i * i + q * q) * saturation;
    }

    i = chroma * cos(hue);
    q = chroma * sin(hue);

    var yiq = vec3<f32>(y_prime, i, q);

    var r = dot(yiq, yiq_to_r);
    var g = dot(yiq, yiq_to_g);
    var b = dot(yiq, yiq_to_b);

    return vec3<f32>(r, g, b);
}