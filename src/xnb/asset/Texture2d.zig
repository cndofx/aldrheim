const std = @import("std");

const rh = @import("../reader_helpers.zig");

const decodeBc1 = @import("../bcn/bc1.zig").decodeBc1;
const decodeBc3 = @import("../bcn/bc3.zig").decodeBc3;

const Texture2d = @This();

format: u32,
width: u32,
height: u32,
mips: [][]u8,

pub fn initFromReader(reader: *std.Io.Reader, gpa: std.mem.Allocator) !Texture2d {
    const format = try rh.readU32(reader, .little);
    const width = try rh.readU32(reader, .little);
    const height = try rh.readU32(reader, .little);
    const mip_count = try rh.readU32(reader, .little);

    var mips = try std.ArrayList([]u8).initCapacity(gpa, mip_count);
    errdefer {
        for (mips.items) |mip| {
            gpa.free(mip);
        }
        mips.deinit(gpa);
    }
    for (0..mip_count) |_| {
        const size = try rh.readU32(reader, .little);
        const mip = try gpa.alloc(u8, size);
        errdefer gpa.free(mip);
        try reader.readSliceAll(mip);
        mips.appendAssumeCapacity(mip);
    }

    return Texture2d{
        .format = format,
        .width = width,
        .height = height,
        .mips = try mips.toOwnedSlice(gpa),
    };
}

pub fn deinit(self: *Texture2d, gpa: std.mem.Allocator) void {
    for (self.mips) |mip| {
        gpa.free(mip);
    }
    gpa.free(self.mips);
    self.* = undefined;
}

pub fn decode(self: Texture2d, gpa: std.mem.Allocator, mip_index: usize) ![]u32 {
    const compressed_pixels = self.mips[mip_index];
    const decompressed_pixels = try gpa.alloc(u32, self.width * self.height);
    switch (self.format) {
        28 => try decodeBc1(compressed_pixels, self.width, self.height, decompressed_pixels),
        32 => try decodeBc3(compressed_pixels, self.width, self.height, decompressed_pixels),
        else => return error.UnsupportedTextureFormat,
    }
    return decompressed_pixels;
}
