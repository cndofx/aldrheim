const std = @import("std");

const rh = @import("../reader_helpers.zig");

const Color = @import("common.zig").Color;
const decodeWithBlockDecoder = @import("common.zig").decodeWithBlockDecoder;
const copyBlockBuffer = @import("common.zig").copyBlockBuffer;

pub fn decodeBc1(data: []const u8, width: usize, height: usize, out: []u32) !void {
    try decodeWithBlockDecoder(data, width, height, out, 4, 4, 8, decodeBc1Block);
}

pub fn decodeBc1Block(data: []const u8, out: []u32) void {
    decodeBc1BlockInner(data, out, false);
}

pub fn decodeBc1aBlock(data: []const u8, out: []u32) void {
    decodeBc1BlockInner(data, out, true);
}

pub fn decodeBc1BlockInner(data: []const u8, out: []u32, use_alpha: bool) void {
    const q0 = rh.u16FromBytes(.{ data[0], data[1] }, .little);
    const q1 = rh.u16FromBytes(.{ data[2], data[3] }, .little);
    const color0 = Color.fromRgb565Le(q0);
    const color1 = Color.fromRgb565Le(q1);
    var c = [4]u32{ color0.toU32Rgba(), color1.toU32Rgba(), 0, 0 };

    const r0: u16 = @intCast(color0.r);
    const g0: u16 = @intCast(color0.g);
    const b0: u16 = @intCast(color0.b);
    const r1: u16 = @intCast(color1.r);
    const g1: u16 = @intCast(color1.g);
    const b1: u16 = @intCast(color1.b);

    if (q0 > q1) {
        c[2] = (Color{
            .r = @intCast((r0 * 2 + r1) / 3),
            .g = @intCast((g0 * 2 + g1) / 3),
            .b = @intCast((b0 * 2 + b1) / 3),
            .a = 255,
        }).toU32Rgba();
        c[3] = (Color{
            .r = @intCast((r0 + r1 * 2) / 3),
            .g = @intCast((g0 + g1 * 2) / 3),
            .b = @intCast((b0 + b1 * 2) / 3),
            .a = 255,
        }).toU32Rgba();
    } else {
        c[2] = (Color{
            .r = @intCast((r0 + r1) / 2),
            .g = @intCast((g0 + g1) / 2),
            .b = @intCast((b0 + b1) / 2),
            .a = 255,
        }).toU32Rgba();
        c[3] = (Color{
            .r = 0,
            .g = 0,
            .b = 0,
            .a = if (use_alpha) 0 else 255,
        }).toU32Rgba();
    }

    var d = rh.u32FromBytes(.{ data[4], data[5], data[6], data[7] }, .little);
    for (0..16) |i| {
        out[i] = c[d & 3];
        d >>= 2;
    }
}
