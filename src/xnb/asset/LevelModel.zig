const std = @import("std");
const zm = @import("matrix");

const rh = @import("../reader_helpers.zig");

const Xnb = @import("../Xnb.zig");
const XnbAsset = @import("../asset.zig").XnbAsset;
const XnbAssetReadError = @import("../asset.zig").XnbAssetReadError;
const Model = @import("Model.zig");
const BiTreeModel = @import("BiTreeModel.zig");
const AnimationClip = @import("AnimationClip.zig");
const Color = @import("Color.zig");
const VertexBuffer = @import("VertexBuffer.zig");
const IndexBuffer = @import("IndexBuffer.zig");
const VertexDeclaration = @import("VertexDeclaration.zig");

const LevelModel = @This();

model: BiTreeModel,
animated_parts: []AnimatedLevelPart,
lights: []Light,
effect_storages: []EffectStorage,
physics_entity_storages: []PhysicsEntityStorage,
liquids: []Liquid,
force_fields: []ForceField,
collision_meshes: []TriangleMesh,
camera_collision_mesh: ?TriangleMesh,
trigger_areas: []TriggerArea,
locators: []Locator,
nav_mesh: NavMesh,

pub fn initFromReader(reader: *std.Io.Reader, type_readers: []const Xnb.TypeReader, gpa: std.mem.Allocator) XnbAssetReadError!LevelModel {
    var model_asset = try XnbAsset.initTypeFromReader(reader, .bi_tree_model, type_readers, gpa);
    errdefer model_asset.deinit(gpa);

    const num_animated_parts: usize = @intCast(try rh.readI32(reader, .little));
    var animated_parts = try std.ArrayList(AnimatedLevelPart).initCapacity(gpa, num_animated_parts);
    errdefer {
        for (animated_parts.items) |*part| {
            part.deinit(gpa);
        }
        animated_parts.deinit(gpa);
    }
    for (0..num_animated_parts) |_| {
        const part = try AnimatedLevelPart.initFromReader(reader, type_readers, gpa);
        animated_parts.appendAssumeCapacity(part);
    }

    const num_lights: usize = @intCast(try rh.readI32(reader, .little));
    var lights = try std.ArrayList(Light).initCapacity(gpa, num_lights);
    errdefer {
        for (lights.items) |*light| {
            light.deinit(gpa);
        }
        lights.deinit(gpa);
    }
    for (0..num_lights) |_| {
        const light = try Light.initFromReader(reader, gpa);
        lights.appendAssumeCapacity(light);
    }

    const num_effect_storages: usize = @intCast(try rh.readI32(reader, .little));
    var effect_storages = try std.ArrayList(EffectStorage).initCapacity(gpa, num_effect_storages);
    errdefer {
        for (effect_storages.items) |*effect| {
            effect.deinit(gpa);
        }
        effect_storages.deinit(gpa);
    }
    for (0..num_effect_storages) |_| {
        const effect = try EffectStorage.initFromReader(reader, gpa);
        effect_storages.appendAssumeCapacity(effect);
    }

    const num_physics_entity_storages: usize = @intCast(try rh.readI32(reader, .little));
    var physics_entity_storages = try std.ArrayList(PhysicsEntityStorage).initCapacity(gpa, num_physics_entity_storages);
    errdefer {
        for (physics_entity_storages.items) |*ent| {
            ent.deinit(gpa);
        }
        physics_entity_storages.deinit(gpa);
    }
    for (0..num_physics_entity_storages) |_| {
        const ent = try PhysicsEntityStorage.initFromReader(reader, gpa);
        physics_entity_storages.appendAssumeCapacity(ent);
    }

    const num_liquids: usize = @intCast(try rh.readI32(reader, .little));
    var liquids = try std.ArrayList(Liquid).initCapacity(gpa, num_liquids);
    errdefer {
        for (liquids.items) |*liquid| {
            liquid.deinit();
        }
        liquids.deinit(gpa);
    }
    for (0..num_liquids) |_| {
        const liquid = try Liquid.initFromReader(reader);
        liquids.appendAssumeCapacity(liquid);
    }

    const num_force_fields: usize = @intCast(try rh.readI32(reader, .little));
    var force_fields = try std.ArrayList(ForceField).initCapacity(gpa, num_force_fields);
    errdefer {
        for (force_fields.items) |*field| {
            field.deinit(gpa);
        }
        force_fields.deinit(gpa);
    }
    for (0..num_force_fields) |_| {
        const field = try ForceField.initFromReader(reader, type_readers, gpa);
        force_fields.appendAssumeCapacity(field);
    }

    const max_collision_meshes = 10;
    var collision_meshes = try std.ArrayList(TriangleMesh).initCapacity(gpa, max_collision_meshes);
    errdefer {
        for (collision_meshes.items) |*mesh| {
            mesh.deinit(gpa);
        }
        collision_meshes.deinit(gpa);
    }
    for (0..max_collision_meshes) |_| {
        const exists = try rh.readBool(reader);
        if (!exists) {
            continue;
        }

        const mesh = try TriangleMesh.initFromReader(reader, type_readers, gpa);
        collision_meshes.appendAssumeCapacity(mesh);
    }

    var camera_mesh: ?TriangleMesh = null;
    errdefer if (camera_mesh) |*mesh| {
        mesh.deinit(gpa);
    };
    const has_camera_mesh = try rh.readBool(reader);
    if (has_camera_mesh) {
        camera_mesh = try TriangleMesh.initFromReader(reader, type_readers, gpa);
    }

    const num_trigger_areas: usize = @intCast(try rh.readI32(reader, .little));
    var trigger_areas = try std.ArrayList(TriggerArea).initCapacity(gpa, num_trigger_areas);
    errdefer {
        for (trigger_areas.items) |*area| {
            area.deinit(gpa);
        }
        trigger_areas.deinit(gpa);
    }
    for (0..num_trigger_areas) |_| {
        const area = try TriggerArea.initFromReader(reader, gpa);
        trigger_areas.appendAssumeCapacity(area);
    }

    const num_locators: u32 = @intCast(try rh.readI32(reader, .little));
    var locators = try std.ArrayList(Locator).initCapacity(gpa, num_locators);
    errdefer {
        for (locators.items) |*locator| {
            locator.deinit(gpa);
        }
        locators.deinit(gpa);
    }
    for (0..num_locators) |_| {
        const locator = try Locator.initFromReader(reader, gpa);
        locators.appendAssumeCapacity(locator);
    }

    var nav_mesh = try NavMesh.initFromReader(reader, gpa);
    errdefer nav_mesh.deinit(gpa);

    return LevelModel{
        .model = model_asset.bi_tree_model,
        .animated_parts = try animated_parts.toOwnedSlice(gpa),
        .lights = try lights.toOwnedSlice(gpa),
        .effect_storages = try effect_storages.toOwnedSlice(gpa),
        .physics_entity_storages = try physics_entity_storages.toOwnedSlice(gpa),
        .liquids = try liquids.toOwnedSlice(gpa),
        .force_fields = try force_fields.toOwnedSlice(gpa),
        .collision_meshes = try collision_meshes.toOwnedSlice(gpa),
        .camera_collision_mesh = camera_mesh,
        .trigger_areas = try trigger_areas.toOwnedSlice(gpa),
        .locators = try locators.toOwnedSlice(gpa),
        .nav_mesh = nav_mesh,
    };
}

pub fn deinit(self: *LevelModel, gpa: std.mem.Allocator) void {
    self.model.deinit(gpa);

    for (self.animated_parts) |*part| {
        part.deinit(gpa);
    }
    gpa.free(self.animated_parts);

    for (self.lights) |*light| {
        light.deinit(gpa);
    }
    gpa.free(self.lights);

    for (self.effect_storages) |*effect| {
        effect.deinit(gpa);
    }
    gpa.free(self.effect_storages);

    for (self.physics_entity_storages) |*ent| {
        ent.deinit(gpa);
    }
    gpa.free(self.physics_entity_storages);

    for (self.liquids) |*liquid| {
        liquid.deinit();
    }
    gpa.free(self.liquids);

    for (self.force_fields) |*field| {
        field.deinit(gpa);
    }
    gpa.free(self.force_fields);

    for (self.collision_meshes) |*mesh| {
        mesh.deinit(gpa);
    }
    gpa.free(self.collision_meshes);

    if (self.camera_collision_mesh) |*mesh| {
        mesh.deinit(gpa);
    }

    for (self.trigger_areas) |*trigger| {
        trigger.deinit(gpa);
    }
    gpa.free(self.trigger_areas);

    for (self.locators) |*locator| {
        locator.deinit(gpa);
    }
    gpa.free(self.locators);

    self.nav_mesh.deinit(gpa);

    self.* = undefined;
}

pub const AnimatedLevelPart = struct {
    name: []u8,
    affect_shields: bool,
    model: Model,
    mesh_settings: std.StringHashMap(MeshSetting),
    liquids: []Liquid,
    // locators: std.StringHashMap(Locator),
    locators: []Locator,
    animation_duration: f32,
    animation: AnimationClip.Channel,
    effect_storages: []EffectStorage,
    light_refs: []LightRef,
    collision: ?Collision,
    children: []AnimatedLevelPart,

    pub const Collision = struct {
        material: Material,
        mesh: TriangleMesh,

        pub const Material = enum(u8) {
            generic,
            gravel,
            grass,
            wood,
            snow,
            stone,
            mud,
            reflect,
            water,
            lava,
        };
    };

    pub fn initFromReader(reader: *std.Io.Reader, type_readers: []const Xnb.TypeReader, gpa: std.mem.Allocator) XnbAssetReadError!AnimatedLevelPart {
        const name = try rh.read7BitLengthString(reader, gpa);
        errdefer gpa.free(name);

        const affect_shields = try rh.readBool(reader);

        var model_asset = try XnbAsset.initTypeFromReader(reader, .model, type_readers, gpa);
        errdefer model_asset.deinit(gpa);

        const num_mesh_settings: u32 = @intCast(try rh.readI32(reader, .little));
        var mesh_settings = std.StringHashMap(MeshSetting).init(gpa);
        errdefer {
            var keys = mesh_settings.keyIterator();
            while (keys.next()) |key| {
                gpa.free(key.*);
            }
            mesh_settings.deinit();
        }
        try mesh_settings.ensureUnusedCapacity(num_mesh_settings);
        for (0..num_mesh_settings) |_| {
            const key = try rh.read7BitLengthString(reader, gpa);
            const a = try rh.readBool(reader);
            const b = try rh.readBool(reader);
            const old_kv = mesh_settings.fetchPutAssumeCapacity(key, .{ .a = a, .b = b });
            if (old_kv) |kv| {
                gpa.free(kv.key);
            }
        }

        const num_liquids: usize = @intCast(try rh.readI32(reader, .little));
        var liquids = try std.ArrayList(Liquid).initCapacity(gpa, num_liquids);
        errdefer {
            for (liquids.items) |*liquid| {
                liquid.deinit();
            }
            liquids.deinit(gpa);
        }
        for (0..num_liquids) |_| {
            const liquid = try Liquid.initFromReader(reader);
            liquids.appendAssumeCapacity(liquid);
        }

        const num_locators: u32 = @intCast(try rh.readI32(reader, .little));
        // var locators = std.StringHashMap(Locator).init(gpa);
        // errdefer {
        //     var values = locators.valueIterator();
        //     while (values.next()) |v| {
        //         v.deinit(gpa); // key is owned by locator, no need to free separately
        //     }
        //     locators.deinit();
        // }
        // try locators.ensureUnusedCapacity(num_locators);
        // for (0..num_locators) |_| {
        //     const locator = try Locator.initFromReader(reader, gpa);
        //     const old_kv = locators.fetchPutAssumeCapacity(locator.name, locator);
        //     if (old_kv) |kv| {
        //         var v = kv.value; // ???
        //         v.deinit(gpa); // key is owned by locator, no need to free separately
        //     }
        // }
        var locators = try std.ArrayList(Locator).initCapacity(gpa, num_locators);
        errdefer {
            for (locators.items) |*locator| {
                locator.deinit(gpa);
            }
            locators.deinit(gpa);
        }
        for (0..num_locators) |_| {
            const locator = try Locator.initFromReader(reader, gpa);
            locators.appendAssumeCapacity(locator);
        }

        const animation_duration = try rh.readF32(reader, .little);
        var animation = try AnimationClip.Channel.initFromReader(reader, gpa);
        errdefer animation.deinit(gpa);

        const num_effect_storages: usize = @intCast(try rh.readI32(reader, .little));
        var effect_storages = try std.ArrayList(EffectStorage).initCapacity(gpa, num_effect_storages);
        errdefer {
            for (effect_storages.items) |*effect| {
                effect.deinit(gpa);
            }
            effect_storages.deinit(gpa);
        }
        for (0..num_effect_storages) |_| {
            const effect = try EffectStorage.initFromReader(reader, gpa);
            effect_storages.appendAssumeCapacity(effect);
        }

        const num_lights: usize = @intCast(try rh.readI32(reader, .little));
        var light_refs = try std.ArrayList(LightRef).initCapacity(gpa, num_lights);
        errdefer {
            for (light_refs.items) |*light| {
                light.deinit(gpa);
            }
            light_refs.deinit(gpa);
        }
        for (0..num_lights) |_| {
            const light = try LightRef.initFromReader(reader, gpa);
            light_refs.appendAssumeCapacity(light);
        }

        var collision: ?Collision = null;
        const has_collision = try rh.readBool(reader);
        if (has_collision) {
            const material: Collision.Material = @enumFromInt(try rh.readU8(reader));

            // const reader_index: usize = @intCast(try rh.read7BitEncodedI32(reader));
            // const reader_name = type_readers[reader_index - 1].name;
            // if (std.mem.startsWith(u8, reader_name, @import("../asset.zig").list_reader_name) == false) {
            //     return XnbAssetReadError.UnexpectedAssetType;
            // }

            // const num_vertices = try rh.readU32(reader, .little);
            // const vertices = try gpa.alloc(zm.Vec3, num_vertices);
            // errdefer gpa.free(vertices);
            // for (0..num_vertices) |i| {
            //     vertices[i] = try rh.readVec3(reader);
            // }

            // const num_indices: usize = @intCast(try rh.readI32(reader, .little));
            // const indices = try gpa.alloc([3]i32, num_indices);
            // errdefer gpa.free(indices);
            // for (0..num_indices) |i| {
            //     indices[i][0] = try rh.readI32(reader, .little);
            //     indices[i][1] = try rh.readI32(reader, .little);
            //     indices[i][2] = try rh.readI32(reader, .little);
            // }

            const mesh = try TriangleMesh.initFromReader(reader, type_readers, gpa);

            collision = Collision{
                .material = material,
                .mesh = mesh,
                // .mesh = TriangleMesh{
                //     .vertices = vertices,
                //     .indices = indices,
                // },
            };
        }

        const has_navmesh = try rh.readBool(reader);
        if (has_navmesh) {
            return XnbAssetReadError.Unimplemented;
        }

        const num_children: usize = @intCast(try rh.readI32(reader, .little));
        var children = try std.ArrayList(AnimatedLevelPart).initCapacity(gpa, num_children);
        errdefer {
            for (children.items) |*child| {
                child.deinit(gpa);
            }
            children.deinit(gpa);
        }
        for (0..num_children) |_| {
            const child = try AnimatedLevelPart.initFromReader(reader, type_readers, gpa);
            children.appendAssumeCapacity(child);
        }

        return AnimatedLevelPart{
            .name = name,
            .affect_shields = affect_shields,
            .model = model_asset.model,
            .mesh_settings = mesh_settings,
            .liquids = try liquids.toOwnedSlice(gpa),
            .locators = try locators.toOwnedSlice(gpa),
            .animation_duration = animation_duration,
            .animation = animation,
            .effect_storages = try effect_storages.toOwnedSlice(gpa),
            .light_refs = try light_refs.toOwnedSlice(gpa),
            .collision = collision,
            .children = try children.toOwnedSlice(gpa),
        };
    }

    pub fn deinit(self: *AnimatedLevelPart, gpa: std.mem.Allocator) void {
        gpa.free(self.name);
        self.model.deinit(gpa);

        var mesh_settings = self.mesh_settings.iterator();
        while (mesh_settings.next()) |entry| {
            gpa.free(entry.key_ptr.*);
        }
        self.mesh_settings.deinit();

        gpa.free(self.liquids);

        // var locators = self.locators.iterator();
        // while (locators.next()) |entry| {
        //     entry.value_ptr.deinit(gpa); // key is owned by locator, no need to free separately
        // }
        // self.locators.deinit();
        for (self.locators) |*locator| {
            locator.deinit(gpa);
        }
        gpa.free(self.locators);

        self.animation.deinit(gpa);

        for (self.effect_storages) |*effect| {
            effect.deinit(gpa);
        }
        gpa.free(self.effect_storages);

        for (self.light_refs) |*light| {
            light.deinit(gpa);
        }
        gpa.free(self.light_refs);

        // if (self.collision) |collision| {
        //     gpa.free(collision.mesh.vertices);
        //     gpa.free(collision.mesh.indices);
        // }
        if (self.collision) |*collision| {
            collision.mesh.deinit(gpa);
        }

        for (self.children) |*child| {
            child.deinit(gpa);
        }
        gpa.free(self.children);

        self.* = undefined;
    }
};

pub const MeshSetting = struct {
    a: bool,
    b: bool,
};

pub const Liquid = union(enum) {
    water,
    lava,

    pub fn initFromReader(reader: *std.Io.Reader) XnbAssetReadError!Liquid {
        _ = reader;
        return XnbAssetReadError.Unimplemented;
    }

    pub fn deinit(self: *Liquid) void {
        _ = self;
    }
};

pub const Locator = struct {
    name: []u8,
    transform: zm.Mat4x4,
    radius: f32,

    pub fn initFromReader(reader: *std.Io.Reader, gpa: std.mem.Allocator) XnbAssetReadError!Locator {
        const name = try rh.read7BitLengthString(reader, gpa);
        errdefer gpa.free(name);

        const transform = try rh.readMat4x4(reader);
        const radius = try rh.readF32(reader, .little);

        return Locator{
            .name = name,
            .transform = transform,
            .radius = radius,
        };
    }

    pub fn deinit(self: *Locator, gpa: std.mem.Allocator) void {
        gpa.free(self.name);
        self.* = undefined;
    }
};

pub const EffectStorage = struct {
    name: []u8,
    position: zm.Vec3,
    forward: zm.Vec3,
    range: f32,
    effect: []u8,

    pub fn initFromReader(reader: *std.Io.Reader, gpa: std.mem.Allocator) XnbAssetReadError!EffectStorage {
        const name = try rh.read7BitLengthString(reader, gpa);
        errdefer gpa.free(name);
        const position = try rh.readVec3(reader);
        const forward = try rh.readVec3(reader);
        const range = try rh.readF32(reader, .little);
        const effect = try rh.read7BitLengthString(reader, gpa);
        errdefer gpa.free(effect);

        return EffectStorage{
            .name = name,
            .position = position,
            .forward = forward,
            .range = range,
            .effect = effect,
        };
    }

    pub fn deinit(self: *EffectStorage, gpa: std.mem.Allocator) void {
        gpa.free(self.name);
        gpa.free(self.effect);
        self.* = undefined;
    }
};

pub const PhysicsEntityStorage = struct {
    transform: zm.Mat4x4,
    template: []u8,

    pub fn initFromReader(reader: *std.Io.Reader, gpa: std.mem.Allocator) XnbAssetReadError!PhysicsEntityStorage {
        const transform = try rh.readMat4x4(reader);
        const template = try rh.read7BitLengthString(reader, gpa);
        errdefer gpa.free(template);

        return PhysicsEntityStorage{
            .transform = transform,
            .template = template,
        };
    }

    pub fn deinit(self: *PhysicsEntityStorage, gpa: std.mem.Allocator) void {
        gpa.free(self.template);
        self.* = undefined;
    }
};

pub const Light = struct {
    name: []u8,
    position: zm.Vec3,
    direction: zm.Vec3,
    kind: Kind,
    variation: Variation,
    reach: f32,
    use_attenuation: bool,
    cutoff_angle: f32,
    sharpness: f32,
    diffuse_color: Color,
    ambient_color: Color,
    specular_amount: f32,
    variation_amount: f32,
    variation_speed: f32,
    shadow_map_size: i32,
    casts_shadows: bool,

    pub const Kind = enum(u8) {
        point = 0,
        directional,
        spot,
        custom = 10,
    };

    pub const Variation = enum(u8) {
        none = 0,
        sine,
        flicker,
        candle,
        strobe,
    };

    pub fn initFromReader(reader: *std.Io.Reader, gpa: std.mem.Allocator) XnbAssetReadError!Light {
        const name = try rh.read7BitLengthString(reader, gpa);
        errdefer gpa.free(name);
        const position = try rh.readVec3(reader);
        const direction = try rh.readVec3(reader);
        const kind: Kind = @enumFromInt(try rh.readI32(reader, .little));
        const variation: Variation = @enumFromInt(try rh.readI32(reader, .little));
        const reach = try rh.readF32(reader, .little);
        const use_attenuation = try rh.readBool(reader);
        const cutoff_angle = try rh.readF32(reader, .little);
        const sharpness = try rh.readF32(reader, .little);
        const diffuse_color = try Color.initFromReader(reader);
        const ambient_color = try Color.initFromReader(reader);
        const specular_amount = try rh.readF32(reader, .little);
        const variation_speed = try rh.readF32(reader, .little);
        const variation_amount = try rh.readF32(reader, .little);
        const shadow_map_size = try rh.readI32(reader, .little);
        const casts_shadows = try rh.readBool(reader);

        return Light{
            .name = name,
            .position = position,
            .direction = direction,
            .kind = kind,
            .variation = variation,
            .reach = reach,
            .use_attenuation = use_attenuation,
            .cutoff_angle = cutoff_angle,
            .sharpness = sharpness,
            .diffuse_color = diffuse_color,
            .ambient_color = ambient_color,
            .specular_amount = specular_amount,
            .variation_amount = variation_amount,
            .variation_speed = variation_speed,
            .shadow_map_size = shadow_map_size,
            .casts_shadows = casts_shadows,
        };
    }

    pub fn deinit(self: *Light, gpa: std.mem.Allocator) void {
        gpa.free(self.name);
        self.* = undefined;
    }
};

pub const LightRef = struct {
    name: []u8,
    transform: zm.Mat4x4,

    pub fn initFromReader(reader: *std.Io.Reader, gpa: std.mem.Allocator) XnbAssetReadError!LightRef {
        const name = try rh.read7BitLengthString(reader, gpa);
        errdefer gpa.free(name);
        const transform = try rh.readMat4x4(reader);

        return LightRef{
            .name = name,
            .transform = transform,
        };
    }

    pub fn deinit(self: *LightRef, gpa: std.mem.Allocator) void {
        gpa.free(self.name);
        self.* = undefined;
    }
};

pub const TriangleMesh = struct {
    vertices: []zm.Vec3,
    indices: [][3]i32,

    pub fn initFromReader(reader: *std.Io.Reader, type_readers: []const Xnb.TypeReader, gpa: std.mem.Allocator) XnbAssetReadError!TriangleMesh {
        const reader_index: usize = @intCast(try rh.read7BitEncodedI32(reader));
        const reader_name = type_readers[reader_index - 1].name;
        if (std.mem.startsWith(u8, reader_name, @import("../asset.zig").list_reader_name) == false) {
            return XnbAssetReadError.UnexpectedAssetType;
        }

        const num_vertices = try rh.readU32(reader, .little);
        const vertices = try gpa.alloc(zm.Vec3, num_vertices);
        errdefer gpa.free(vertices);
        for (0..num_vertices) |i| {
            vertices[i] = try rh.readVec3(reader);
        }

        const num_indices: usize = @intCast(try rh.readI32(reader, .little));
        const indices = try gpa.alloc([3]i32, num_indices);
        errdefer gpa.free(indices);
        for (0..num_indices) |i| {
            indices[i][0] = try rh.readI32(reader, .little);
            indices[i][1] = try rh.readI32(reader, .little);
            indices[i][2] = try rh.readI32(reader, .little);
        }

        return TriangleMesh{
            .vertices = vertices,
            .indices = indices,
        };
    }

    pub fn deinit(self: *TriangleMesh, gpa: std.mem.Allocator) void {
        gpa.free(self.vertices);
        gpa.free(self.indices);
        self.* = undefined;
    }
};

pub const ForceField = struct {
    color: Color,
    width: f32,
    alpha_power: f32,
    alpha_falloff_power: f32,
    max_radius: f32,
    ripple_distortion: f32,
    map_distortion: f32,
    vertex_color_enabled: bool,
    displacement_map: []u8,
    ttl: f32,
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    vertex_declaration: VertexDeclaration,
    vertex_stride: i32,
    num_vertices: i32,
    primitive_count: i32,

    pub fn initFromReader(reader: *std.Io.Reader, type_readers: []const Xnb.TypeReader, gpa: std.mem.Allocator) XnbAssetReadError!ForceField {
        const color = try Color.initFromReader(reader);
        const width = try rh.readF32(reader, .little);
        const alpha_power = try rh.readF32(reader, .little);
        const alpha_falloff_power = try rh.readF32(reader, .little);
        const max_radius = try rh.readF32(reader, .little);
        const ripple_distortion = try rh.readF32(reader, .little);
        const map_distortion = try rh.readF32(reader, .little);
        const vertex_color_enabled = try rh.readBool(reader);
        const displacement_map = try rh.read7BitLengthString(reader, gpa);
        errdefer gpa.free(displacement_map);
        const ttl = try rh.readF32(reader, .little);
        var vertex_buffer_asset = try XnbAsset.initTypeFromReader(reader, .vertex_buffer, type_readers, gpa);
        errdefer vertex_buffer_asset.deinit(gpa);
        var index_buffer_asset = try XnbAsset.initTypeFromReader(reader, .index_buffer, type_readers, gpa);
        errdefer index_buffer_asset.deinit(gpa);
        var vertex_declaration_asset = try XnbAsset.initTypeFromReader(reader, .vertex_declaration, type_readers, gpa);
        errdefer vertex_declaration_asset.deinit(gpa);
        const vertex_stride = try rh.readI32(reader, .little);
        const num_vertices = try rh.readI32(reader, .little);
        const primitive_count = try rh.readI32(reader, .little);

        return ForceField{
            .color = color,
            .width = width,
            .alpha_power = alpha_power,
            .alpha_falloff_power = alpha_falloff_power,
            .max_radius = max_radius,
            .ripple_distortion = ripple_distortion,
            .map_distortion = map_distortion,
            .vertex_color_enabled = vertex_color_enabled,
            .displacement_map = displacement_map,
            .ttl = ttl,
            .vertex_buffer = vertex_buffer_asset.vertex_buffer,
            .index_buffer = index_buffer_asset.index_buffer,
            .vertex_declaration = vertex_declaration_asset.vertex_declaration,
            .vertex_stride = vertex_stride,
            .num_vertices = num_vertices,
            .primitive_count = primitive_count,
        };
    }

    pub fn deinit(self: *ForceField, gpa: std.mem.Allocator) void {
        gpa.free(self.displacement_map);
        self.vertex_buffer.deinit(gpa);
        self.index_buffer.deinit(gpa);
        self.vertex_declaration.deinit(gpa);
        self.* = undefined;
    }
};

pub const TriggerArea = struct {
    name: []u8,
    position: zm.Vec3,
    side_lengths: zm.Vec3,
    orientation: zm.Quat,

    pub fn initFromReader(reader: *std.Io.Reader, gpa: std.mem.Allocator) XnbAssetReadError!TriggerArea {
        const name = try rh.read7BitLengthString(reader, gpa);
        errdefer gpa.free(name);
        const position = try rh.readVec3(reader);
        const side_lengths = try rh.readVec3(reader);
        const orientation = try rh.readQuat(reader);

        return TriggerArea{
            .name = name,
            .position = position,
            .side_lengths = side_lengths,
            .orientation = orientation,
        };
    }

    pub fn deinit(self: *TriggerArea, gpa: std.mem.Allocator) void {
        gpa.free(self.name);
        self.* = undefined;
    }
};

pub const NavMesh = struct {
    vertices: []zm.Vec3,
    triangles: []Triangle,

    pub fn initFromReader(reader: *std.Io.Reader, gpa: std.mem.Allocator) XnbAssetReadError!NavMesh {
        const num_vertices = try rh.readU16(reader, .little);
        const vertices = try gpa.alloc(zm.Vec3, num_vertices);
        errdefer gpa.free(vertices);
        for (0..num_vertices) |i| {
            const vertex = try rh.readVec3(reader);
            vertices[i] = vertex;
        }

        const num_triangles = try rh.readU16(reader, .little);
        const triangles = try gpa.alloc(Triangle, num_triangles);
        errdefer gpa.free(triangles);
        for (0..num_triangles) |i| {
            const triangle = try Triangle.initFromReader(reader);
            triangles[i] = triangle;
        }

        return NavMesh{
            .vertices = vertices,
            .triangles = triangles,
        };
    }

    pub fn deinit(self: *NavMesh, gpa: std.mem.Allocator) void {
        gpa.free(self.vertices);
        gpa.free(self.triangles);
        self.* = undefined;
    }

    pub const Triangle = struct {
        vertex_a: u16,
        vertex_b: u16,
        vertex_c: u16,
        neighbor_a: u16,
        neighbor_b: u16,
        neighbor_c: u16,
        cost_ab: f32,
        cost_bc: f32,
        cost_ca: f32,
        properties: MovementProperties,

        pub fn initFromReader(reader: *std.Io.Reader) XnbAssetReadError!Triangle {
            const vertex_a = try rh.readU16(reader, .little);
            const vertex_b = try rh.readU16(reader, .little);
            const vertex_c = try rh.readU16(reader, .little);
            const neighbor_a = try rh.readU16(reader, .little);
            const neighbor_b = try rh.readU16(reader, .little);
            const neighbor_c = try rh.readU16(reader, .little);
            const cost_ab = try rh.readF32(reader, .little);
            const cost_bc = try rh.readF32(reader, .little);
            const cost_ca = try rh.readF32(reader, .little);
            const properties = MovementProperties.fromU8(try rh.readU8(reader));

            return Triangle{
                .vertex_a = vertex_a,
                .vertex_b = vertex_b,
                .vertex_c = vertex_c,
                .neighbor_a = neighbor_a,
                .neighbor_b = neighbor_b,
                .neighbor_c = neighbor_c,
                .cost_ab = cost_ab,
                .cost_bc = cost_bc,
                .cost_ca = cost_ca,
                .properties = properties,
            };
        }
    };

    pub const MovementProperties = packed struct(u8) {
        water: bool = false,
        jump: bool = false,
        fly: bool = false,
        dynamic: bool = false,
        _padding: u4 = 0,

        pub fn fromU8(v: u8) MovementProperties {
            const water = (v & 1) != 0;
            const jump = (v & 2) != 0;
            const fly = (v & 4) != 0;
            const dynamic = (v & 128) != 0;

            return MovementProperties{
                .water = water,
                .jump = jump,
                .fly = fly,
                .dynamic = dynamic,
            };
        }
    };
};
