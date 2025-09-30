// ported from https://github.com/UniversalGameExtraction/texture2ddecoder

const std = @import("std");

const rh = @import("../reader_helpers.zig");

pub fn decodeWithBlockDecoder(
    data: []const u8,
    width: usize,
    height: usize,
    out: []u32,
    comptime block_width: comptime_int,
    comptime block_height: comptime_int,
    comptime raw_block_size: comptime_int,
    comptime blockDecoder: fn ([]const u8, []u32) void,
) !void {
    const block_size = block_width * block_height;
    const num_blocks_x = try std.math.divCeil(usize, width, block_width);
    const num_blocks_y = try std.math.divCeil(usize, height, block_height);

    if (data.len < num_blocks_x * num_blocks_y * raw_block_size) {
        return error.InputDataTooSmall;
    }

    if (out.len < width * height) {
        return error.OutputDataTooSmall;
    }

    var buffer: [block_size]u32 = undefined;
    @memset(&buffer, (Color{ .r = 0, .g = 0, .b = 0, .a = 255 }).toU32Rgba());

    var data_offset: usize = 0;
    for (0..num_blocks_y) |by| {
        for (0..num_blocks_x) |bx| {
            blockDecoder(data[data_offset..], &buffer);
            copyBlockBuffer(bx, by, width, height, block_width, block_height, &buffer, out);
            data_offset += raw_block_size;
        }
    }
}

pub fn copyBlockBuffer(bx: usize, by: usize, w: usize, h: usize, bw: usize, bh: usize, in: []const u32, out: []u32) void {
    const x = bw * bx;
    const copy_width = if (bw * (bx + 1) > w) w - bw * bx else bw;

    const y0 = by * bh;
    const copy_height = if (bh * (by + 1) > h) h - y0 else bh;
    var buffer_offset: usize = 0;

    for (y0..y0 + copy_height) |y| {
        const image_offset = y * w + x;
        @memcpy(
            out[image_offset .. image_offset + copy_width],
            in[buffer_offset .. buffer_offset + copy_width],
        );
        buffer_offset += bw;
    }
}

pub const Color = struct {
    r: u8,
    g: u8,
    b: u8,
    a: u8,

    pub fn toU32Bgra(self: Color) u32 {
        return rh.u32FromBytes(.{ self.b, self.g, self.r, self.a }, .little);
    }

    pub fn toU32Rgba(self: Color) u32 {
        return rh.u32FromBytes(.{ self.r, self.g, self.b, self.a }, .little);
    }

    pub fn fromRgb565Le(rgb: u16) Color {
        const r: u8 = @truncate((rgb >> 8 & 0xF8) | (rgb >> 13));
        const g: u8 = @truncate((rgb >> 3 & 0xFC) | (rgb >> 9 & 3));
        const b: u8 = @truncate((rgb << 3) | (rgb >> 2 & 7));
        return Color{
            .r = r,
            .g = g,
            .b = b,
            .a = 255,
        };
    }
};
