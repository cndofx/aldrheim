const builtin = @import("builtin");
const std = @import("std");
const zm = @import("matrix");

const ReaderError = std.Io.Reader.Error;
const AllocatorReaderError = std.mem.Allocator.Error || std.Io.Reader.Error;

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

pub fn f32FromBytes(bytes: [4]u8, endian: Endian) f32 {
    var num: i32 = @bitCast(bytes);
    if (native_endian != endian) {
        num = @byteSwap(num);
    }
    return @bitCast(num);
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

pub fn readU8(reader: *std.Io.Reader) ReaderError!u8 {
    var buf: [1]u8 = undefined;
    try reader.readSliceAll(&buf);
    return buf[0];
}

pub fn readU16(reader: *std.Io.Reader, endian: Endian) ReaderError!u16 {
    var buf: [2]u8 = undefined;
    try reader.readSliceAll(&buf);
    return u16FromBytes(buf, endian);
}

pub fn readU32(reader: *std.Io.Reader, endian: Endian) ReaderError!u32 {
    var buf: [4]u8 = undefined;
    try reader.readSliceAll(&buf);
    return u32FromBytes(buf, endian);
}

pub fn readI32(reader: *std.Io.Reader, endian: Endian) ReaderError!i32 {
    var buf: [4]u8 = undefined;
    try reader.readSliceAll(&buf);
    return i32FromBytes(buf, endian);
}

pub fn readF32(reader: *std.Io.Reader, endian: Endian) ReaderError!f32 {
    var buf: [4]u8 = undefined;
    try reader.readSliceAll(&buf);
    return f32FromBytes(buf, endian);
}

pub fn read7BitEncodedI32(reader: *std.Io.Reader) ReaderError!i32 {
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
pub fn read7BitLengthString(reader: *std.Io.Reader, gpa: std.mem.Allocator) AllocatorReaderError![]u8 {
    const len = try read7BitEncodedI32(reader);
    const s = try gpa.alloc(u8, @intCast(len));
    errdefer gpa.free(s);
    try reader.readSliceAll(s);
    return s;
}

pub fn readVec3(reader: *std.Io.Reader) ReaderError!zm.Vec3 {
    const x = try readF32(reader, .little);
    const y = try readF32(reader, .little);
    const z = try readF32(reader, .little);
    return zm.Vec3.init(x, y, z);
}

pub fn readQuat(reader: *std.Io.Reader) ReaderError!zm.Quat {
    const x = try readF32(reader, .little);
    const y = try readF32(reader, .little);
    const z = try readF32(reader, .little);
    const w = try readF32(reader, .little);
    return zm.Quat.init(x, y, z, w);
}

pub fn readMat4x4(reader: *std.Io.Reader) ReaderError!zm.Mat4x4 {
    const m11 = try readF32(reader, .little);
    const m12 = try readF32(reader, .little);
    const m13 = try readF32(reader, .little);
    const m14 = try readF32(reader, .little);

    const m21 = try readF32(reader, .little);
    const m22 = try readF32(reader, .little);
    const m23 = try readF32(reader, .little);
    const m24 = try readF32(reader, .little);

    const m31 = try readF32(reader, .little);
    const m32 = try readF32(reader, .little);
    const m33 = try readF32(reader, .little);
    const m34 = try readF32(reader, .little);

    const m41 = try readF32(reader, .little);
    const m42 = try readF32(reader, .little);
    const m43 = try readF32(reader, .little);
    const m44 = try readF32(reader, .little);

    const mat = zm.Mat4x4.fromSlice(&.{
        m11, m12, m13, m14,
        m21, m22, m23, m24,
        m31, m32, m33, m34,
        m41, m42, m43, m44,
    });

    return mat;
}

pub fn readBool(reader: *std.Io.Reader) ReaderError!bool {
    const v = try readU8(reader);
    return v != 0;
}
