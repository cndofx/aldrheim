const std = @import("std");

const rh = @import("../reader_helpers.zig");

const Color = @import("../bcn/common.zig").Color;
const decodeBc1 = @import("../bcn/bc1.zig").decodeBc1;
const decodeBc3 = @import("../bcn/bc3.zig").decodeBc3;

const Texture2d = @This();

format: PixelFormat,
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
        .format = @enumFromInt(format),
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

/// returns rgba8 pixels
pub fn decode(self: Texture2d, gpa: std.mem.Allocator, mip_index: usize) ![]u32 {
    const source = self.mips[mip_index];
    return try decodePixels(gpa, source, self.width, self.height, self.format);
}

/// returns rgba8 pixels
pub fn decodePixels(gpa: std.mem.Allocator, source: []const u8, width: usize, height: usize, format: PixelFormat) ![]u32 {
    const dest = try gpa.alloc(u32, width * height);
    errdefer gpa.free(dest);
    switch (format) {
        .color => {
            for (0..height) |y| {
                for (0..width) |x| {
                    const index = y * width + x;
                    const b = source[index * 4 + 0];
                    const g = source[index * 4 + 1];
                    const r = source[index * 4 + 2];
                    const a = source[index * 4 + 3];
                    const pixel = (Color{ .r = r, .g = g, .b = b, .a = a }).toU32Rgba();
                    dest[index] = pixel;
                }
            }
        },
        .bc1 => try decodeBc1(source, width, height, dest),
        .bc3 => try decodeBc3(source, width, height, dest),
        else => {
            std.debug.print("error: unsupported texture format: {}\n", .{format});
            return error.UnsupportedTextureFormat;
        },
    }
    return dest;
}

pub const PixelFormat = enum(u32) {
    /// bgra8?
    color = 1,
    bc1 = 28,
    bc3 = 32,
    _,

    pub fn stride(self: PixelFormat) !usize {
        switch (self) {
            .color => return 4,
            else => {
                std.debug.print("error: unsupported texture format: {}\n", .{self});
                return error.UnsupportedTextureFormat;
            },
        }
    }
};
