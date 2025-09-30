const builtin = @import("builtin");
const std = @import("std");

const Endian = std.builtin.Endian;
const native_endian = builtin.cpu.arch.endian();

pub fn u16FromBytes(bytes: [2]u8, endian: Endian) u16 {
    var num: u16 = @bitCast(bytes);
    if (native_endian != endian) {
        num = @byteSwap(num);
    }
    return num;
}

pub fn u32FromBytes(bytes: [4]u8, endian: Endian) u32 {
    var num: u32 = @bitCast(bytes);
    if (native_endian != endian) {
        num = @byteSwap(num);
    }
    return num;
}

pub fn i32FromBytes(bytes: [4]u8, endian: Endian) i32 {
    var num: i32 = @bitCast(bytes);
    if (native_endian != endian) {
        num = @byteSwap(num);
    }
    return num;
}

pub fn u64FromBytes(bytes: [8]u8, endian: Endian) u64 {
    var num: u64 = @bitCast(bytes);
    if (native_endian != endian) {
        num = @byteSwap(num);
    }
    return num;
}

pub fn bytesFromU16(value: u16, endian: Endian) [2]u8 {
    var v = value;
    if (native_endian != endian) {
        v = @byteSwap(v);
    }
    return @bitCast(v);
}

pub fn bytesFromU32(value: u32, endian: Endian) [4]u8 {
    var v = value;
    if (native_endian != endian) {
        v = @byteSwap(v);
    }
    return @bitCast(v);
}

pub fn readU8(reader: *std.Io.Reader) !u8 {
    var buf: [1]u8 = undefined;
    try reader.readSliceAll(&buf);
    return buf[0];
}

pub fn readU16(reader: *std.Io.Reader, endian: Endian) !u16 {
    var buf: [2]u8 = undefined;
    try reader.readSliceAll(&buf);
    return u16FromBytes(buf, endian);
}

pub fn readU32(reader: *std.Io.Reader, endian: Endian) !u32 {
    var buf: [4]u8 = undefined;
    try reader.readSliceAll(&buf);
    return u32FromBytes(buf, endian);
}

pub fn readI32(reader: *std.Io.Reader, endian: Endian) !i32 {
    var buf: [4]u8 = undefined;
    try reader.readSliceAll(&buf);
    return i32FromBytes(buf, endian);
}

pub fn read7BitEncodedI32(reader: *std.Io.Reader) !i32 {
    var result: i32 = 0;
    var bits_read: u5 = 0;

    while (true) {
        const byte: i32 = @intCast(try readU8(reader));
        result |= (byte & 0x7F) << bits_read;
        bits_read += 7;

        if (byte & 0x80 == 0) {
            break;
        }
    }

    return result;
}

/// result must be freed
pub fn read7BitLengthString(reader: *std.Io.Reader, gpa: std.mem.Allocator) ![]u8 {
    const len = try read7BitEncodedI32(reader);
    const s = try gpa.alloc(u8, @intCast(len));
    try reader.readSliceAll(s);
    return s;
}
