const std = @import("std");

const rh = @import("reader_helpers.zig");
const Xnb = @import("Xnb.zig");

const Texture2DAsset = @import("asset/Texture2dAsset.zig");

pub const texture_2d_reader_name = "Microsoft.Xna.Framework.Content.Texture2DReader";

pub const XnbAssetKind = @typeInfo(XnbAsset).@"union".tag_type.?;
pub const XnbAssetMap = std.StaticStringMap(XnbAssetKind);
pub const xnb_asset_map = XnbAssetMap.initComptime(.{
    .{ texture_2d_reader_name, .texture_2d },
});

pub const XnbAsset = union(enum) {
    none,
    texture_2d: Texture2DAsset,

    pub fn initFromReader(reader: *std.Io.Reader, type_readers: []const Xnb.TypeReader, gpa: std.mem.Allocator) !XnbAsset {
        const type_id = try rh.read7BitEncodedI32(reader);
        if (type_id == 0) {
            return XnbAsset.none;
        }

        const type_reader = type_readers[@intCast(type_id - 1)];

        var split = std.mem.splitScalar(u8, type_reader.name, ',');
        const name = split.next().?;

        const asset_kind = xnb_asset_map.get(name).?;
        std.debug.print("asset_kind: {any}\n", .{asset_kind});

        switch (asset_kind) {
            .none => return .none,
            .texture_2d => return .{ .texture_2d = try .initFromReader(reader, gpa) },
        }

        return error.Unimplemented;
    }

    pub fn deinit(self: *XnbAsset, gpa: std.mem.Allocator) void {
        switch (self.*) {
            .none => {},
            .texture_2d => self.texture_2d.deinit(gpa),
        }
    }
};
