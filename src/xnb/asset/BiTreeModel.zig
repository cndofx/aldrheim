const std = @import("std");

const rh = @import("../reader_helpers.zig");

const Xnb = @import("../Xnb.zig");
const XnbAsset = @import("../asset.zig").XnbAsset;
const XnbAssetReadError = @import("../asset.zig").XnbAssetReadError;
const VertexDeclaration = @import("VertexDeclaration.zig");
const VertexBuffer = @import("VertexBuffer.zig");
const IndexBuffer = @import("IndexBuffer.zig");
const Model = @import("Model.zig");

const BiTreeModel = @This();

trees: []BiTree,

pub fn initFromReader(reader: *std.Io.Reader, type_readers: []const Xnb.TypeReader, gpa: std.mem.Allocator) XnbAssetReadError!BiTreeModel {
    const num_trees: usize = @intCast(try rh.readI32(reader, .little));
    var trees = try std.ArrayList(BiTree).initCapacity(gpa, num_trees);
    errdefer {
        for (trees.items) |*tree| {
            tree.deinit(gpa);
        }
        trees.deinit(gpa);
    }
    for (0..num_trees) |_| {
        const tree = try BiTree.initFromReader(reader, type_readers, gpa);
        trees.appendAssumeCapacity(tree);
    }

    return BiTreeModel{
        .trees = try trees.toOwnedSlice(gpa),
    };
}

pub fn deinit(self: *BiTreeModel, gpa: std.mem.Allocator) void {
    for (self.trees) |*tree| {
        tree.deinit(gpa);
    }
    gpa.free(self.trees);
    self.* = undefined;
}

pub const BiTree = struct {
    visible: bool,
    cast_shadows: bool,
    sway: f32,
    entity_influence: f32,
    ground_level: f32,
    num_vertices: i32,
    vertex_stride: i32,
    vertex_decl: VertexDeclaration,
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    effect: XnbAsset, // TODO,
    node: BiTreeNode,

    pub fn initFromReader(reader: *std.Io.Reader, type_readers: []const Xnb.TypeReader, gpa: std.mem.Allocator) XnbAssetReadError!BiTree {
        const visible = try rh.readBool(reader);
        const cast_shadows = try rh.readBool(reader);
        const sway = try rh.readF32(reader, .little);
        const entity_influence = try rh.readF32(reader, .little);
        const ground_level = try rh.readF32(reader, .little);
        const num_vertices = try rh.readI32(reader, .little);
        const vertex_stride = try rh.readI32(reader, .little);

        var vertex_decl_asset = try XnbAsset.initTypeFromReader(reader, .vertex_declaration, type_readers, gpa);
        errdefer vertex_decl_asset.deinit(gpa);

        var vertex_buffer_asset = try XnbAsset.initTypeFromReader(reader, .vertex_buffer, type_readers, gpa);
        errdefer vertex_buffer_asset.deinit(gpa);

        var index_buffer_asset = try XnbAsset.initTypeFromReader(reader, .index_buffer, type_readers, gpa);
        errdefer index_buffer_asset.deinit(gpa);

        var effect_asset = try XnbAsset.initFromReader(reader, type_readers, gpa);
        errdefer effect_asset.deinit(gpa);

        var node = try BiTreeNode.initFromReader(reader, gpa);
        errdefer node.deinit(gpa);

        return BiTree{
            .visible = visible,
            .cast_shadows = cast_shadows,
            .sway = sway,
            .entity_influence = entity_influence,
            .ground_level = ground_level,
            .num_vertices = num_vertices,
            .vertex_stride = vertex_stride,
            .vertex_decl = vertex_decl_asset.vertex_declaration,
            .vertex_buffer = vertex_buffer_asset.vertex_buffer,
            .index_buffer = index_buffer_asset.index_buffer,
            .effect = effect_asset,
            .node = node,
        };
    }

    pub fn deinit(self: *BiTree, gpa: std.mem.Allocator) void {
        self.vertex_decl.deinit(gpa);
        self.vertex_buffer.deinit(gpa);
        self.index_buffer.deinit(gpa);
        self.effect.deinit(gpa);
        self.node.deinit(gpa);
    }
};

pub const BiTreeNode = struct {
    primitive_count: i32,
    start_index: i32,
    bounding_box: Model.BoundingBox,
    child_a: ?*BiTreeNode,
    child_b: ?*BiTreeNode,

    pub fn initFromReader(reader: *std.Io.Reader, gpa: std.mem.Allocator) XnbAssetReadError!BiTreeNode {
        const primitive_count = try rh.readI32(reader, .little);
        const start_index = try rh.readI32(reader, .little);
        const bounding_box = try Model.BoundingBox.initFromReader(reader);

        var child_a: ?*BiTreeNode = null;
        errdefer if (child_a) |a| {
            a.deinit(gpa);
            gpa.destroy(a);
        };
        const has_child_a = try rh.readBool(reader);
        if (has_child_a) {
            var a = try BiTreeNode.initFromReader(reader, gpa);
            errdefer a.deinit(gpa);
            child_a = try gpa.create(BiTreeNode);
            child_a.?.* = a;
        }

        var child_b: ?*BiTreeNode = null;
        errdefer if (child_b) |b| {
            b.deinit(gpa);
            gpa.destroy(b);
        };
        const has_child_b = try rh.readBool(reader);
        if (has_child_b) {
            var b = try BiTreeNode.initFromReader(reader, gpa);
            errdefer b.deinit(gpa);
            child_b = try gpa.create(BiTreeNode);
            child_b.?.* = b;
        }

        return BiTreeNode{
            .primitive_count = primitive_count,
            .start_index = start_index,
            .bounding_box = bounding_box,
            .child_a = child_a,
            .child_b = child_b,
        };
    }

    pub fn deinit(self: *BiTreeNode, gpa: std.mem.Allocator) void {
        if (self.child_a) |a| {
            a.deinit(gpa);
            gpa.destroy(a);
        }

        if (self.child_b) |b| {
            b.deinit(gpa);
            gpa.destroy(b);
        }

        self.* = undefined;
    }
};
