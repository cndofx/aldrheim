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

pub fn getIndexCount(self: IndexBuffer) usize {
    const size: usize = if (self.is_16_bit) 2 else 4;
    return self.data.len / size;
}

pub fn getIndex(self: IndexBuffer, position: usize) u32 {
    const size: usize = if (self.is_16_bit) 2 else 4;
    const i = position * size;
    if (self.is_16_bit) {
        var buf: [2]u8 = undefined; // ugly
        @memcpy(&buf, self.data[i .. i + size]);
        return @as(u32, rh.u16FromBytes(buf, .little));
    } else {
        var buf: [4]u8 = undefined;
        @memcpy(&buf, self.data[i .. i + size]);
        return rh.u32FromBytes(buf, .little);
    }
}
