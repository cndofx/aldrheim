const std = @import("std");

const rh = @import("reader_helpers.zig");
const Xnb = @import("Xnb.zig");

const Texture2D = @import("asset/Texture2d.zig");
const Model = @import("asset/Model.zig");
const VertexDeclaration = @import("asset/VertexDeclaration.zig");
const VertexBuffer = @import("asset/VertexBuffer.zig");
const IndexBuffer = @import("asset/IndexBuffer.zig");

pub const string_reader_name = "Microsoft.Xna.Framework.Content.StringReader";
pub const texture_2d_reader_name = "Microsoft.Xna.Framework.Content.Texture2DReader";
pub const model_reader_name = "Microsoft.Xna.Framework.Content.ModelReader";
pub const vertex_declaration_reader_name = "Microsoft.Xna.Framework.Content.VertexDeclarationReader";
pub const vertex_buffer_reader_name = "Microsoft.Xna.Framework.Content.VertexBufferReader";
pub const index_buffer_reader_name = "Microsoft.Xna.Framework.Content.IndexBufferReader";

pub const XnbAssetReadError = error{UnexpectedAssetType} || std.Io.Reader.Error || std.mem.Allocator.Error;

pub const XnbAssetKind = @typeInfo(XnbAsset).@"union".tag_type.?;
pub const XnbAssetMap = std.StaticStringMap(XnbAssetKind);
pub const xnb_asset_map = XnbAssetMap.initComptime(.{
    .{ string_reader_name, .string },
    .{ texture_2d_reader_name, .texture_2d },
    .{ model_reader_name, .model },
    .{ vertex_declaration_reader_name, .vertex_declaration },
    .{ vertex_buffer_reader_name, .vertex_buffer },
    .{ index_buffer_reader_name, .index_buffer },
});

pub const XnbAsset = union(enum) {
    none,
    string: []u8,
    texture_2d: Texture2D,
    model: Model,
    vertex_declaration: VertexDeclaration,
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,

    pub fn initFromReader(reader: *std.Io.Reader, type_readers: []const Xnb.TypeReader, gpa: std.mem.Allocator) XnbAssetReadError!XnbAsset {
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
            .string => return .{ .string = try rh.read7BitLengthString(reader, gpa) },
            .texture_2d => return .{ .texture_2d = try .initFromReader(reader, gpa) },
            .model => return .{ .model = try .initFromReader(reader, type_readers, gpa) },
            .vertex_declaration => return .{ .vertex_declaration = try .initFromReader(reader, gpa) },
            .vertex_buffer => return .{ .vertex_buffer = try .initFromReader(reader, gpa) },
            .index_buffer => return .{ .index_buffer = try .initFromReader(reader, gpa) },
        }

        return error.Unimplemented;
    }

    pub fn deinit(self: *XnbAsset, gpa: std.mem.Allocator) void {
        switch (self.*) {
            .none => {},
            .string => gpa.free(self.string),
            .texture_2d => self.texture_2d.deinit(gpa),
            .model => self.model.deinit(gpa),
            .vertex_declaration => self.vertex_declaration.deinit(gpa),
            .vertex_buffer => self.vertex_buffer.deinit(gpa),
            .index_buffer => self.index_buffer.deinit(gpa),
        }
    }
};
