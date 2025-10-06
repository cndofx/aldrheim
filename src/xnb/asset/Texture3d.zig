const std = @import("std");

const rh = @import("../reader_helpers.zig");

const PixelFormat = @import("Texture2d.zig").PixelFormat;

const Texture3d = @This();

format: PixelFormat,
width: u32,
height: u32,
depth: u32,
mips: [][]u8,

pub fn initFromReader(reader: *std.Io.Reader, gpa: std.mem.Allocator) !Texture3d {
    const format = try rh.readU32(reader, .little);
    const width = try rh.readU32(reader, .little);
    const height = try rh.readU32(reader, .little);
    const depth = try rh.readU32(reader, .little);
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

    return Texture3d{
        .format = @enumFromInt(format),
        .width = width,
        .height = height,
        .depth = depth,
        .mips = try mips.toOwnedSlice(gpa),
    };
}

pub fn deinit(self: *Texture3d, gpa: std.mem.Allocator) void {
    for (self.mips) |mip| {
        gpa.free(mip);
    }
    gpa.free(self.mips);
    self.* = undefined;
}
