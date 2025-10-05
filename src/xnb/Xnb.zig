const std = @import("std");
const c = @import("c");

const rh = @import("reader_helpers.zig");
const XnbAsset = @import("asset.zig").XnbAsset;
const LzxDecoder = @import("lzx/LzxDecoder.zig");

const Xnb = @This();

header: Header,
data: []u8,

pub const Header = struct {
    platform: Platform,
    version: Version,
    hi_def: bool,
    compressed: bool,
    compressed_size: u32,
    uncompressed_size: u32,
};

pub const Platform = enum {
    windows,
    windows_phone,
    xbox360,
};

pub const Version = enum {
    xna_31,
    xna_40,
};

pub const Content = struct {
    type_readers: []TypeReader,
    primary_asset: XnbAsset,
    shared_assets: []XnbAsset,

    pub fn init(gpa: std.mem.Allocator, xnb: Xnb) !Content {
        return try xnb.parseContent(gpa);
    }

    pub fn deinit(self: *Content, gpa: std.mem.Allocator) void {
        for (self.type_readers) |reader| {
            gpa.free(reader.name);
        }
        gpa.free(self.type_readers);

        for (self.shared_assets) |*shared| {
            shared.deinit(gpa);
        }
        gpa.free(self.shared_assets);

        self.primary_asset.deinit(gpa);
    }
};

pub const TypeReader = struct {
    name: []const u8,
    version: i32,
};

pub fn initFromFile(gpa: std.mem.Allocator, path: []const u8) !Xnb {
    const file = try std.fs.cwd().openFile(path, .{});
    defer file.close();

    var reader_buf: [1024]u8 = undefined;
    var file_reader = file.reader(&reader_buf);
    const reader = &file_reader.interface;

    return try Xnb.initFromReader(gpa, reader);
}

pub fn initFromReader(gpa: std.mem.Allocator, reader: *std.Io.Reader) !Xnb {
    var magic: [3]u8 = undefined;
    try reader.readSliceAll(&magic);
    if (std.mem.eql(u8, &magic, "XNB") == false) {
        return error.NotAnXnbFile;
    }

    const platform = switch (try rh.readU8(reader)) {
        'w' => Platform.windows,
        'm' => Platform.windows_phone,
        'x' => Platform.xbox360,
        else => return error.UnknownPlatform,
    };

    const version = switch (try rh.readU8(reader)) {
        4 => Version.xna_31,
        5 => Version.xna_40,
        else => return error.UnknownVersion,
    };
    if (version != .xna_31) {
        return error.UnsupportedVersion;
    }

    const flags = try rh.readU8(reader);
    const hi_def = flags & 0x01 != 0;
    const compressed = flags & 0x80 != 0;

    const compressed_size = try rh.readU32(reader, .little);
    const uncompressed_size = if (compressed) try rh.readU32(reader, .little) else 0;

    const header_size: u32 = if (compressed) 14 else 10;
    const data = try gpa.alloc(u8, compressed_size - header_size);
    errdefer gpa.free(data);
    try reader.readSliceAll(data);

    return Xnb{
        .header = Header{
            .platform = platform,
            .version = version,
            .hi_def = hi_def,
            .compressed = compressed,
            .compressed_size = compressed_size,
            .uncompressed_size = uncompressed_size,
        },
        .data = data,
    };
}

pub fn deinit(self: *Xnb, gpa: std.mem.Allocator) void {
    gpa.free(self.data);
    self.* = undefined;
}

pub fn decompress(self: Xnb, gpa: std.mem.Allocator) ![]u8 {
    var fixed_reader = std.Io.Reader.fixed(self.data);
    const reader = &fixed_reader;

    var lzxd = try LzxDecoder.init(gpa, 16);
    defer lzxd.deinit(gpa);

    const decompressed = try gpa.alloc(u8, self.header.uncompressed_size);
    errdefer gpa.free(decompressed);
    var fixed_writer = std.Io.Writer.fixed(decompressed);
    const writer = &fixed_writer;

    while (reader.seek < reader.buffer.len) {
        var frame_size: u16 = 0;
        var block_size: u16 = 0;
        if (try rh.readU8(reader) == 0xFF) {
            frame_size = try rh.readU16(reader, .big);
            block_size = try rh.readU16(reader, .big);
        } else {
            reader.seek -= 1;
            block_size = try rh.readU16(reader, .big);
            frame_size = 0x8000;
        }
        if (block_size == 0 or frame_size == 0) {
            break;
        }

        const pos = reader.seek;
        try lzxd.decompress(gpa, reader, block_size, writer, frame_size);
        reader.seek = pos + block_size;
    }
    try writer.flush();

    return decompressed;
}

/// content must be freed
pub fn parseContent(self: Xnb, gpa: std.mem.Allocator) !Content {
    const data: []const u8 = if (self.header.compressed) try self.decompress(gpa) else self.data;
    defer if (self.header.compressed) {
        gpa.free(data);
    };

    var fixed_reader = std.Io.Reader.fixed(data);
    const reader = &fixed_reader;

    const type_reader_count: usize = @intCast(try rh.read7BitEncodedI32(reader));
    std.debug.print("type reader count: {}\n", .{type_reader_count});

    var type_readers = try std.ArrayList(TypeReader).initCapacity(gpa, type_reader_count);
    errdefer {
        for (type_readers.items) |tr| {
            gpa.free(tr.name);
        }
        type_readers.deinit(gpa);
    }
    for (0..type_reader_count) |_| {
        const name = try rh.read7BitLengthString(reader, gpa);
        const version = try rh.readI32(reader, .little);
        const tr = TypeReader{
            .name = name,
            .version = version,
        };
        type_readers.appendAssumeCapacity(tr);
        std.debug.print("type reader: {s}\n", .{name});
    }

    const shared_asset_count = try rh.read7BitEncodedI32(reader);
    _ = shared_asset_count;

    var primary_asset = try XnbAsset.initFromReader(reader, type_readers.items, gpa);
    errdefer primary_asset.deinit(gpa);

    return Content{
        .type_readers = try type_readers.toOwnedSlice(gpa),
        .primary_asset = primary_asset,
        .shared_assets = &.{},
    };
}
