const builtin = @import("builtin");
const std = @import("std");
const c = @import("c");

const sdl = @import("sdl3");

const Xnb = @import("xnb/Xnb.zig");
const Texture2d = @import("xnb/asset/Texture2d.zig");

pub const runtime_safety = switch (builtin.mode) {
    .Debug, .ReleaseSafe => true,
    .ReleaseFast, .ReleaseSmall => false,
};

pub fn main() !u8 {
    const stack_trace_frames = if (builtin.mode == .Debug) 16 else 0;
    var debug_allocator: std.heap.DebugAllocator(.{ .stack_trace_frames = stack_trace_frames }) = .init;
    const gpa = if (runtime_safety)
        debug_allocator.allocator()
    else
        std.heap.c_allocator;
    defer if (runtime_safety) {
        _ = debug_allocator.deinit();
    };

    const args = try std.process.argsAlloc(gpa);
    defer std.process.argsFree(gpa, args);

    const usage = "usage:\n  aldrheim [path_to_magicka_install]\n    or\n  aldrheim extract [path_to_xnb]\n";
    if (args.len < 2) {
        std.debug.print("{s}", .{usage});
        return 1;
    } else if (args.len == 2) {
        try run(gpa, args[1]);
    } else if (args.len == 3) {
        if (std.mem.eql(u8, args[1], "extract") == false) {
            std.debug.print("{s}", .{usage});
            return 1;
        }
        try extractXnb(gpa, args[2]);
    } else if (args.len > 3) {
        std.debug.print("{s}", .{usage});
        return 1;
    } else {
        unreachable;
    }

    return 0;
}

fn run(gpa: std.mem.Allocator, magicka_path: []const u8) !void {
    _ = gpa;

    std.debug.print("magicka path: {s}\n", .{magicka_path});

    try sdl.hints.set(.app_id, "cndofx.Aldrheim");
    try sdl.hints.set(.app_name, "Aldrheim");

    const sdl_init_flags = sdl.InitFlags{ .events = true, .video = true };
    try sdl.init(sdl_init_flags);
    defer sdl.quit(sdl_init_flags);

    const window = try sdl.video.Window.init("Aldrheim", 1280, 720, .{ .resizable = false });
    defer window.deinit();

    var running = true;
    while (running) {
        const surface = try window.getSurface();
        try surface.fillRect(null, surface.mapRgb(128, 30, 255));
        try window.updateSurface();

        while (sdl.events.poll()) |event| {
            switch (event) {
                .quit => running = false,
                else => {},
            }
        }
    }
}

fn extractXnb(gpa: std.mem.Allocator, path: []const u8) !void {
    var xnb = try Xnb.initFromFile(gpa, path);
    defer xnb.deinit(gpa);

    const decompressed = if (xnb.header.compressed) try xnb.decompress(gpa) else xnb.data;
    defer if (xnb.header.compressed) {
        gpa.free(decompressed);
    };

    var content = try Xnb.parseContentFrom(decompressed, gpa);
    defer content.deinit(gpa);

    // dump decompressed
    {
        const out_path = try std.fmt.allocPrint(gpa, "{s}.decompressed", .{path});
        defer gpa.free(out_path);
        var out_file = try std.fs.cwd().createFile(out_path, .{});
        defer out_file.close();

        var out_writer = out_file.writer(&.{});
        const writer = &out_writer.interface;
        try writer.writeAll(decompressed);
        try writer.flush();
    }

    // dump png
    if (content.primary_asset == .texture_2d) {
        const texture = content.primary_asset.texture_2d;

        const pixels = try texture.decode(gpa, 0);
        defer gpa.free(pixels);

        const out_path = try std.fmt.allocPrint(gpa, "{s}.png\x00", .{path});
        defer gpa.free(out_path);

        if (c.stbi_write_png(
            @ptrCast(out_path),
            @intCast(texture.width),
            @intCast(texture.height),
            4,
            @ptrCast(pixels),
            @intCast(4 * texture.width),
        ) == 0) {
            return error.StbWritePngFailed;
        }
    }

    // dump png slices of 3d texture
    if (content.primary_asset == .texture_3d) {
        const texture = content.primary_asset.texture_3d;
        std.debug.print("3d width: {}, height: {}, depth: {}\n", .{ texture.width, texture.height, texture.depth });
        const slice_stride = texture.width * texture.height * 4;
        for (0..texture.depth) |z| {
            const slice_start = z * slice_stride;
            const slice = texture.mips[0][slice_start .. slice_start + slice_stride];
            const pixels = try Texture2d.decodePixels(gpa, slice, texture.width, texture.height, texture.format);
            defer gpa.free(pixels);

            const out_path = try std.fmt.allocPrint(gpa, "{s}-depth{}.png\x00", .{ path, z });
            defer gpa.free(out_path);

            if (c.stbi_write_png(
                @ptrCast(out_path),
                @intCast(texture.width),
                @intCast(texture.height),
                4,
                @ptrCast(pixels),
                @intCast(4 * texture.width),
            ) == 0) {
                return error.StbWritePngFailed;
            }
        }
    }

    std.debug.print("{}\n", .{xnb.header});
}
