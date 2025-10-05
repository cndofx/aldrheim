const std = @import("std");

const rh = @import("../reader_helpers.zig");

const XnbAssetReadError = @import("../asset.zig").XnbAssetReadError;
const Color = @import("Color.zig");

const RenderDeferredEffect = @This();

alpha: f32,
sharpness: f32,
vertex_color_enabled: bool,
use_material_texture_for_reflectiveness: bool,
reflection_map: []u8,
material_0: Material,
material_1: ?Material,

pub fn initFromReader(reader: *std.Io.Reader, gpa: std.mem.Allocator) XnbAssetReadError!RenderDeferredEffect {
    const alpha = try rh.readF32(reader, .little);
    const sharpness = try rh.readF32(reader, .little);
    const vertex_color_enabled = try rh.readBool(reader);
    const use_material_texture_for_reflectiveness = try rh.readBool(reader);
    const reflection_map = try rh.read7BitLengthString(reader, gpa);
    errdefer gpa.free(reflection_map);

    var material_0 = try Material.initFromReader(reader, gpa);
    errdefer material_0.deinit(gpa);

    const has_material_1 = try rh.readBool(reader);
    var material_1: ?Material = null;
    if (has_material_1) {
        material_1 = try Material.initFromReader(reader, gpa);
    }
    errdefer if (material_1) |*m| {
        m.deinit(gpa);
    };

    return RenderDeferredEffect{
        .alpha = alpha,
        .sharpness = sharpness,
        .vertex_color_enabled = vertex_color_enabled,
        .use_material_texture_for_reflectiveness = use_material_texture_for_reflectiveness,
        .reflection_map = reflection_map,
        .material_0 = material_0,
        .material_1 = material_1,
    };
}

pub fn deinit(self: *RenderDeferredEffect, gpa: std.mem.Allocator) void {
    gpa.free(self.reflection_map);
    self.material_0.deinit(gpa);
    if (self.material_1) |*m| {
        m.deinit(gpa);
    }
    self.* = undefined;
}

pub const Material = struct {
    diffuse_texture_alpha_disabled: bool,
    alpha_mask_enabled: bool,
    diffuse_color: Color,
    spec_amount: f32,
    spec_power: f32,
    emissive_amount: f32,
    normal_power: f32,
    reflectiveness: f32,
    diffuse_texture: []u8,
    material_texture: []u8,
    normal_texture: []u8,

    pub fn initFromReader(reader: *std.Io.Reader, gpa: std.mem.Allocator) XnbAssetReadError!Material {
        const diffuse_texture_alpha_disabled = try rh.readBool(reader);
        const alpha_mask_enabled = try rh.readBool(reader);
        const diffuse_color = try Color.initFromReader(reader);
        const spec_amount = try rh.readF32(reader, .little);
        const spec_power = try rh.readF32(reader, .little);
        const emissive_amount = try rh.readF32(reader, .little);
        const normal_power = try rh.readF32(reader, .little);
        const reflectiveness = try rh.readF32(reader, .little);
        const diffuse_texture = try rh.read7BitLengthString(reader, gpa);
        errdefer gpa.free(diffuse_texture);
        const material_texture = try rh.read7BitLengthString(reader, gpa);
        errdefer gpa.free(material_texture);
        const normal_texture = try rh.read7BitLengthString(reader, gpa);
        errdefer gpa.free(normal_texture);

        return Material{
            .diffuse_texture_alpha_disabled = diffuse_texture_alpha_disabled,
            .alpha_mask_enabled = alpha_mask_enabled,
            .diffuse_color = diffuse_color,
            .spec_amount = spec_amount,
            .spec_power = spec_power,
            .emissive_amount = emissive_amount,
            .normal_power = normal_power,
            .reflectiveness = reflectiveness,
            .diffuse_texture = diffuse_texture,
            .material_texture = material_texture,
            .normal_texture = normal_texture,
        };
    }

    pub fn deinit(self: *Material, gpa: std.mem.Allocator) void {
        gpa.free(self.diffuse_texture);
        gpa.free(self.material_texture);
        gpa.free(self.normal_texture);
        self.* = undefined;
    }
};
