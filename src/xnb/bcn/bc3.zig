const std = @import("std");

const rh = @import("../reader_helpers.zig");

const Color = @import("common.zig").Color;
const decodeWithBlockDecoder = @import("common.zig").decodeWithBlockDecoder;
const decodeBc1Block = @import("bc1.zig").decodeBc1Block;
const copyBlockBuffer = @import("common.zig").copyBlockBuffer;

pub fn decodeBc3(data: []const u8, width: usize, height: usize, out: []u32) !void {
    try decodeWithBlockDecoder(data, width, height, out, 4, 4, 16, decodeBc3Block);
}

pub fn decodeBc3Block(data: []const u8, out: []u32) void {
    decodeBc1Block(data[8..], out);
    decodeBc3Alpha(data, out, 3);
}

pub fn decodeBc3Alpha(data: []const u8, out: []u32, channel: u2) void {
    var a: [8]u16 = [_]u16{ data[0], data[1], 0, 0, 0, 0, 0, 0 };
    if (a[0] > a[1]) {
        a[2] = (a[0] * 6 + a[1]) / 7;
        a[3] = (a[0] * 5 + a[1] * 2) / 7;
        a[4] = (a[0] * 4 + a[1] * 3) / 7;
        a[5] = (a[0] * 3 + a[1] * 4) / 7;
        a[6] = (a[0] * 2 + a[1] * 5) / 7;
        a[7] = (a[0] + a[1] * 6) / 7;
    } else {
        a[2] = (a[0] * 4 + a[1]) / 5;
        a[3] = (a[0] * 3 + a[1] * 2) / 5;
        a[4] = (a[0] * 2 + a[1] * 3) / 5;
        a[5] = (a[0] + a[1] * 4) / 5;
        a[6] = 0;
        a[7] = 255;
    }

    var d: usize = @intCast(rh.u64FromBytes(data[0..8].*, .little) >> 16);

    const channel_shift = @as(u5, channel) * 8;
    const channel_mask: u32 = 0xFFFFFFFF ^ (@as(u32, 0xFF) << channel_shift);
    for (0..out.len) |p| {
        out[p] = (out[p] & channel_mask) | @as(u32, @intCast(a[d & 7])) << channel_shift;
        d >>= 3;
    }
}
