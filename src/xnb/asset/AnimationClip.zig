const std = @import("std");
const zm = @import("matrix");

const rh = @import("../reader_helpers.zig");
const XnbAssetReadError = @import("../asset.zig").XnbAssetReadError;

const AnimationClip = @This();

name: []u8,
duration: f32,
channels: std.StringHashMap(Channel),

pub const Channel = struct {
    keyframes: []Keyframe,

    pub fn initFromReader(reader: *std.Io.Reader, gpa: std.mem.Allocator) XnbAssetReadError!Channel {
        const num_frames: usize = @intCast(try rh.readI32(reader, .little));
        const keyframes = try gpa.alloc(Keyframe, num_frames);
        errdefer gpa.free(keyframes);
        for (0..num_frames) |i| {
            keyframes[i] = try Keyframe.initFromReader(reader);
        }

        return Channel{
            .keyframes = keyframes,
        };
    }

    pub fn deinit(self: *Channel, gpa: std.mem.Allocator) void {
        gpa.free(self.keyframes);
        self.* = undefined;
    }
};

pub const Keyframe = struct {
    time: f32,
    pose: Pose,

    pub fn initFromReader(reader: *std.Io.Reader) XnbAssetReadError!Keyframe {
        const time = try rh.readF32(reader, .little);
        const pose = try Pose.initFromReader(reader);

        return Keyframe{
            .time = time,
            .pose = pose,
        };
    }
};

pub const Pose = struct {
    translation: zm.Vec3,
    orientation: zm.Quat,
    scale: zm.Vec3,

    pub fn initFromReader(reader: *std.Io.Reader) XnbAssetReadError!Pose {
        const translation = try rh.readVec3(reader);
        const orientation = try rh.readQuat(reader);
        const scale = try rh.readVec3(reader);

        return Pose{
            .translation = translation,
            .orientation = orientation,
            .scale = scale,
        };
    }
};
