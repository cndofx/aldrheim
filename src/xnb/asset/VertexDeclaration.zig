const std = @import("std");

const rh = @import("../reader_helpers.zig");

const VertexDeclaration = @This();

elements: []Element,

pub fn initFromReader(reader: *std.Io.Reader, gpa: std.mem.Allocator) !VertexDeclaration {
    const num_elements = try rh.readU32(reader, .little);
    const elements = try gpa.alloc(Element, num_elements);
    errdefer gpa.free(elements);
    for (0..num_elements) |i| {
        elements[i] = try Element.initFromReader(reader);
    }

    return VertexDeclaration{
        .elements = elements,
    };
}

pub fn deinit(self: *VertexDeclaration, gpa: std.mem.Allocator) void {
    gpa.free(self.elements);
    self.* = undefined;
}

pub const Element = struct {
    stream: u16,
    offset: u16,
    format: Format,
    method: Method,
    usage: Usage,
    usage_index: u8,

    pub fn initFromReader(reader: *std.Io.Reader) !Element {
        const stream = try rh.readU16(reader, .little);
        const offset = try rh.readU16(reader, .little);
        const format: Format = @enumFromInt(try rh.readU8(reader));
        const method: Method = @enumFromInt(try rh.readU8(reader));
        const usage: Usage = @enumFromInt(try rh.readU8(reader));
        const usage_index = try rh.readU8(reader);

        return Element{
            .stream = stream,
            .offset = offset,
            .format = format,
            .method = method,
            .usage = usage,
            .usage_index = usage_index,
        };
    }

    // TODO: i dont remember where these came from
    pub const Format = enum(u8) {
        single,
        vec2,
        vec3,
        vec4,
        color,
        byte4,
        short2,
        short4,
        normalized_short_2,
        normalized_short_4,
        rgb32,
        rgba64,
        uint40,
        normalized40,
        half_vector_2,
        half_vector_4,
    };

    // TODO: i dont remember where these came from
    pub const Method = enum(u8) {
        default,
        uv = 4,
        lookup = 5,
        lookup_presampled,
    };

    // TODO: i dont remember where these came from
    pub const Usage = enum(u8) {
        position,
        blend_weight,
        blend_indices,
        normal,
        point_size,
        texture_coordinate,
        tangent,
        binormal,
        tesselate_factor,
        color = 10,
        fog,
        depth,
        sample,
    };
};
