const std = @import("std");

const rh = @import("../reader_helpers.zig");

const Color = @This();

r: f32,
g: f32,
b: f32,

pub fn initFromReader(reader: *std.Io.Reader) !Color {
    const r = try rh.readF32(reader, .little);
    const g = try rh.readF32(reader, .little);
    const b = try rh.readF32(reader, .little);

    return Color{
        .r = r,
        .g = g,
        .b = b,
    };
}
