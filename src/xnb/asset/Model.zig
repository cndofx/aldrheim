const std = @import("std");
const zm = @import("matrix");

const rh = @import("../reader_helpers.zig");

const Xnb = @import("../Xnb.zig");
const XnbAsset = @import("../asset.zig").XnbAsset;
const XnbAssetReadError = @import("../asset.zig").XnbAssetReadError;
const VertexDeclaration = @import("VertexDeclaration.zig");
const VertexBuffer = @import("VertexBuffer.zig");
const IndexBuffer = @import("IndexBuffer.zig");

const Model = @This();

bones: []Bone,
bones_hierarchy: []BoneHierarchy,
vertex_decls: []VertexDeclaration,
meshes: []Mesh,
root_bone_ref: u32,
tag: u8,

pub fn initFromReader(reader: *std.Io.Reader, type_readers: []const Xnb.TypeReader, gpa: std.mem.Allocator) XnbAssetReadError!Model {
    const num_bones = try rh.readU32(reader, .little);

    const bones = try gpa.alloc(Bone, num_bones);
    errdefer gpa.free(bones);
    for (0..num_bones) |i| {
        bones[i] = try Bone.initFromReader(reader, type_readers, gpa);
    }

    const bones_hierarchy = try gpa.alloc(BoneHierarchy, num_bones);
    errdefer gpa.free(bones_hierarchy);
    for (0..num_bones) |i| {
        bones_hierarchy[i] = try BoneHierarchy.initFromReader(reader, num_bones, gpa);
    }

    const num_vertex_decls = try rh.readU32(reader, .little);
    const vertex_decls = try gpa.alloc(VertexDeclaration, num_vertex_decls);
    errdefer gpa.free(vertex_decls);
    for (0..num_vertex_decls) |i| {
        const asset = try XnbAsset.initFromReader(reader, type_readers, gpa);
        if (asset != .vertex_declaration) {
            return XnbAssetReadError.UnexpectedAssetType;
        }
        vertex_decls[i] = asset.vertex_declaration;
    }

    const num_meshes = try rh.readU32(reader, .little);
    const meshes = try gpa.alloc(Mesh, num_meshes);
    errdefer gpa.free(meshes);
    for (0..num_meshes) |i| {
        meshes[i] = try Mesh.initFromReader(reader, type_readers, gpa);
    }

    const root_bone_ref = try readBoneRef(reader, num_bones);
    const tag = try rh.readU8(reader);

    return Model{
        .bones = bones,
        .bones_hierarchy = bones_hierarchy,
        .vertex_decls = vertex_decls,
        .meshes = meshes,
        .root_bone_ref = root_bone_ref,
        .tag = tag,
    };
}

pub fn deinit(self: *Model, gpa: std.mem.Allocator) void {
    for (self.bones) |*bone| {
        bone.deinit(gpa);
    }
    gpa.free(self.bones);

    for (self.bones_hierarchy) |*hierarchy| {
        hierarchy.deinit(gpa);
    }
    gpa.free(self.bones_hierarchy);

    for (self.vertex_decls) |*decl| {
        decl.deinit(gpa);
    }
    gpa.free(self.vertex_decls);

    for (self.meshes) |*mesh| {
        mesh.deinit(gpa);
    }
    gpa.free(self.meshes);

    self.* = undefined;
}

pub const Bone = struct {
    name: []u8,
    transform: zm.Mat4x4,

    pub fn initFromReader(reader: *std.Io.Reader, type_readers: []const Xnb.TypeReader, gpa: std.mem.Allocator) XnbAssetReadError!Bone {
        const name_asset = try XnbAsset.initFromReader(reader, type_readers, gpa);
        if (name_asset != .string) {
            return XnbAssetReadError.UnexpectedAssetType;
        }
        const name = name_asset.string;
        errdefer gpa.free(name);

        const transform = try rh.readMat4x4(reader);

        return Bone{
            .name = name,
            .transform = transform,
        };
    }

    pub fn deinit(self: *Bone, gpa: std.mem.Allocator) void {
        gpa.free(self.name);
        self.* = undefined;
    }
};

pub const BoneHierarchy = struct {
    parent_ref: u32,
    children_refs: []u32,

    pub fn initFromReader(reader: *std.Io.Reader, num_bones: u32, gpa: std.mem.Allocator) XnbAssetReadError!BoneHierarchy {
        const parent_ref = try readBoneRef(reader, num_bones);
        const num_children = try rh.readU32(reader, .little);
        const children_refs = try gpa.alloc(u32, num_children);
        errdefer gpa.free(children_refs);
        for (0..num_children) |i| {
            children_refs[i] = try readBoneRef(reader, num_bones);
        }

        return BoneHierarchy{
            .parent_ref = parent_ref,
            .children_refs = children_refs,
        };
    }

    pub fn deinit(self: *BoneHierarchy, gpa: std.mem.Allocator) void {
        gpa.free(self.children_refs);
        self.* = undefined;
    }
};

pub const Mesh = struct {
    name: []u8,
    parent_bone_ref: u32,
    bounds: BoundingSphere,
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    parts: []Part,
    tag: u8,

    pub fn initFromReader(reader: *std.Io.Reader, type_readers: []const Xnb.TypeReader, gpa: std.mem.Allocator) XnbAssetReadError!Mesh {
        var name_asset = try XnbAsset.initFromReader(reader, type_readers, gpa);
        errdefer name_asset.deinit(gpa);
        if (name_asset != .string) {
            return XnbAssetReadError.UnexpectedAssetType;
        }
        const name = name_asset.string;

        const parent_bone_ref = try readBoneRef(reader, 0);
        const bounds = try BoundingSphere.initFromReader(reader);

        var vertex_buffer_asset = try XnbAsset.initFromReader(reader, type_readers, gpa);
        errdefer vertex_buffer_asset.deinit(gpa);
        if (vertex_buffer_asset != .vertex_buffer) {
            return XnbAssetReadError.UnexpectedAssetType;
        }
        const vertex_buffer = vertex_buffer_asset.vertex_buffer;

        var index_buffer_asset = try XnbAsset.initFromReader(reader, type_readers, gpa);
        errdefer index_buffer_asset.deinit(gpa);
        if (index_buffer_asset != .index_buffer) {
            return XnbAssetReadError.UnexpectedAssetType;
        }
        const index_buffer = index_buffer_asset.index_buffer;

        const tag = try rh.readU8(reader);

        const num_parts = try rh.readU32(reader, .little);
        const parts = try gpa.alloc(Part, num_parts);
        errdefer gpa.free(parts);
        for (0..num_parts) |i| {
            parts[i] = try Part.initFromReader(reader);
        }

        return Mesh{
            .name = name,
            .parent_bone_ref = parent_bone_ref,
            .bounds = bounds,
            .vertex_buffer = vertex_buffer,
            .index_buffer = index_buffer,
            .parts = parts,
            .tag = tag,
        };
    }

    pub fn deinit(self: *Mesh, gpa: std.mem.Allocator) void {
        gpa.free(self.name);
        gpa.free(self.parts);
        self.vertex_buffer.deinit(gpa);
        self.index_buffer.deinit(gpa);
        self.* = undefined;
    }

    pub const Part = struct {
        stream_offset: u32,
        base_vertex: u32,
        vertex_count: u32,
        start_index: u32,
        primitive_count: u32,
        vertex_decl_index: u32,
        tag: u8,
        shared_content_material_index: i32,

        pub fn initFromReader(reader: *std.Io.Reader) !Part {
            const stream_offset = try rh.readU32(reader, .little);
            const base_vertex = try rh.readU32(reader, .little);
            const vertex_count = try rh.readU32(reader, .little);
            const start_index = try rh.readU32(reader, .little);
            const primitive_count = try rh.readU32(reader, .little);
            const vertex_decl_index = try rh.readU32(reader, .little);
            const tag = try rh.readU8(reader);
            const shared_content_material_index = try rh.read7BitEncodedI32(reader);

            return Part{
                .stream_offset = stream_offset,
                .base_vertex = base_vertex,
                .vertex_count = vertex_count,
                .start_index = start_index,
                .primitive_count = primitive_count,
                .vertex_decl_index = vertex_decl_index,
                .tag = tag,
                .shared_content_material_index = shared_content_material_index,
            };
        }
    };
};

pub const BoundingSphere = struct {
    center: zm.Vec3,
    radius: f32,

    pub fn initFromReader(reader: *std.Io.Reader) XnbAssetReadError!BoundingSphere {
        const center = try rh.readVec3(reader);
        const radius = try rh.readF32(reader, .little);

        return BoundingSphere{
            .center = center,
            .radius = radius,
        };
    }
};

fn readBoneRef(reader: *std.Io.Reader, num_bones: u32) XnbAssetReadError!u32 {
    if (num_bones <= 255) {
        return @as(u32, try rh.readU8(reader));
    } else {
        return try rh.readU32(reader, .little);
    }
}
