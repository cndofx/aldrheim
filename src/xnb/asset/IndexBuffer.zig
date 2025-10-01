const std = @import("std");

const rh = @import("../reader_helpers.zig");

const IndexBuffer = @This();

is_16_bit: bool,
data: []u8,

pub fn initFromReader(reader: *std.Io.Reader, gpa: std.mem.Allocator) !IndexBuffer {
    const is_16_bit = try rh.readBool(reader);
    const size = try rh.readU32(reader, .little);
    const data = try gpa.alloc(u8, size);
    errdefer gpa.free(data);
    try reader.readSliceAll(data);

    return IndexBuffer{
        .is_16_bit = is_16_bit,
        .data = data,
    };
}

pub fn deinit(self: *IndexBuffer, gpa: std.mem.Allocator) void {
    gpa.free(self.data);
    self.* = undefined;
}
