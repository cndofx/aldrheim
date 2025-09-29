const std = @import("std");

const rh = @import("../reader_helpers.zig");

const BitBuffer = @This();

buffer: u32 = 0,
bits_left: u5 = 0,
reader: *std.Io.Reader,

pub fn clear(self: *BitBuffer) void {
    self.buffer = 0;
    self.bits_left = 0;
}

pub fn ensureBits(self: *BitBuffer, bits: u5) !void {
    while (self.bits_left < bits) {
        const lo = @as(u32, try rh.readU8(self.reader));
        const hi = @as(u32, try rh.readU8(self.reader));

        self.buffer |= (((hi << 8) | lo) << (32 - 16 - self.bits_left));
        self.bits_left += 16;
    }
}

pub fn peekBits(self: BitBuffer, bits: u5) u32 {
    const shift: u5 = @intCast(32 - @as(u8, bits));
    return self.buffer >> shift;
}

pub fn removeBits(self: *BitBuffer, bits: u5) void {
    self.buffer <<= bits;
    self.bits_left -= bits;
}

pub fn readBits(self: *BitBuffer, bits: u5) !u32 {
    var ret: u32 = 0;

    if (bits > 0) {
        try self.ensureBits(bits);
        ret = self.peekBits(bits);
        self.removeBits(bits);
    }

    return ret;
}
