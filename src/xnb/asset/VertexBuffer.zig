const std = @import("std");

const rh = @import("../reader_helpers.zig");

const VertexBuffer = @This();

data: []u8,

pub fn initFromReader(reader: *std.Io.Reader, gpa: std.mem.Allocator) !VertexBuffer {
    const size = try rh.readU32(reader, .little);
    const data = try gpa.alloc(u8, size);
    errdefer gpa.free(data);
    try reader.readSliceAll(data);

    return VertexBuffer{
        .data = data,
    };
}

pub fn deinit(self: *VertexBuffer, gpa: std.mem.Allocator) void {
    gpa.free(self.data);
    self.* = undefined;
}
