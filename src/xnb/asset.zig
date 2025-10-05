const std = @import("std");

const rh = @import("reader_helpers.zig");
const Xnb = @import("Xnb.zig");

const Texture2D = @import("asset/Texture2d.zig");
const Model = @import("asset/Model.zig");
const VertexDeclaration = @import("asset/VertexDeclaration.zig");
const VertexBuffer = @import("asset/VertexBuffer.zig");
const IndexBuffer = @import("asset/IndexBuffer.zig");
const BiTreeModel = @import("asset/BiTreeModel.zig");
const LevelModel = @import("asset/LevelModel.zig");
const RenderDeferredEffect = @import("asset/RenderDeferredEffect.zig");

pub const string_reader_name = "Microsoft.Xna.Framework.Content.StringReader";
pub const list_reader_name = "Microsoft.Xna.Framework.Content.ListReader"; // handled on site
pub const texture_2d_reader_name = "Microsoft.Xna.Framework.Content.Texture2DReader";
pub const model_reader_name = "Microsoft.Xna.Framework.Content.ModelReader";
pub const vertex_declaration_reader_name = "Microsoft.Xna.Framework.Content.VertexDeclarationReader";
pub const vertex_buffer_reader_name = "Microsoft.Xna.Framework.Content.VertexBufferReader";
pub const index_buffer_reader_name = "Microsoft.Xna.Framework.Content.IndexBufferReader";

pub const bi_tree_model_reader_name = "PolygonHead.Pipeline.BiTreeModelReader";
pub const render_deferred_effect_reader_name = "PolygonHead.Pipeline.RenderDeferredEffectReader";

pub const level_model_reader_name = "Magicka.ContentReaders.LevelModelReader";

pub const XnbAssetReadError = error{ UnexpectedAssetType, Unimplemented } || std.Io.Reader.Error || std.mem.Allocator.Error;

pub const XnbAssetKind = @typeInfo(XnbAsset).@"union".tag_type.?;
pub const XnbAssetMap = std.StaticStringMap(XnbAssetKind);
pub const xnb_asset_map = XnbAssetMap.initComptime(.{
    .{ string_reader_name, .string },
    .{ texture_2d_reader_name, .texture_2d },
    .{ model_reader_name, .model },
    .{ vertex_declaration_reader_name, .vertex_declaration },
    .{ vertex_buffer_reader_name, .vertex_buffer },
    .{ index_buffer_reader_name, .index_buffer },
    .{ bi_tree_model_reader_name, .bi_tree_model },
    .{ render_deferred_effect_reader_name, .render_deferred_effect },
    .{ level_model_reader_name, .level_model },
});

pub const XnbAsset = union(enum) {
    none,
    string: []u8,
    texture_2d: Texture2D,
    model: Model,
    vertex_declaration: VertexDeclaration,
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    bi_tree_model: BiTreeModel,
    render_deferred_effect: RenderDeferredEffect,
    level_model: LevelModel,

    pub fn initFromReader(reader: *std.Io.Reader, type_readers: []const Xnb.TypeReader, gpa: std.mem.Allocator) XnbAssetReadError!XnbAsset {
        const type_id = try rh.read7BitEncodedI32(reader);
        if (type_id == 0) {
            return XnbAsset.none;
        }

        const type_reader = type_readers[@intCast(type_id - 1)];

        var split = std.mem.splitScalar(u8, type_reader.name, ',');
        const name = split.next().?;

        const asset_kind = xnb_asset_map.get(name);
        if (asset_kind != null) {
            std.debug.print("asset_kind: {any}\n", .{asset_kind});
        } else {
            std.debug.print("error: no reader implementation for {s}\n", .{name});
            return XnbAssetReadError.Unimplemented;
        }

        switch (asset_kind.?) {
            .none => return .none,
            .string => return .{ .string = try rh.read7BitLengthString(reader, gpa) },
            .texture_2d => return .{ .texture_2d = try .initFromReader(reader, gpa) },
            .model => return .{ .model = try .initFromReader(reader, type_readers, gpa) },
            .vertex_declaration => return .{ .vertex_declaration = try .initFromReader(reader, gpa) },
            .vertex_buffer => return .{ .vertex_buffer = try .initFromReader(reader, gpa) },
            .index_buffer => return .{ .index_buffer = try .initFromReader(reader, gpa) },
            .bi_tree_model => return .{ .bi_tree_model = try .initFromReader(reader, type_readers, gpa) },
            .render_deferred_effect => return .{ .render_deferred_effect = try .initFromReader(reader, gpa) },
            .level_model => return .{ .level_model = try .initFromReader(reader, type_readers, gpa) },
        }

        return error.Unimplemented;
    }

    pub fn initTypeFromReader(reader: *std.Io.Reader, kind: XnbAssetKind, type_readers: []const Xnb.TypeReader, gpa: std.mem.Allocator) XnbAssetReadError!XnbAsset {
        var asset = try XnbAsset.initFromReader(reader, type_readers, gpa);
        errdefer asset.deinit(gpa);
        if (asset != kind) {
            return XnbAssetReadError.UnexpectedAssetType;
        }
        return asset;
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
            .bi_tree_model => self.bi_tree_model.deinit(gpa),
            .render_deferred_effect => self.render_deferred_effect.deinit(gpa),
            .level_model => self.level_model.deinit(gpa),
        }
    }
};
