const builtin = @import("builtin");
const std = @import("std");
const c = @import("c");

const Xnb = @import("xnb/Xnb.zig");

pub const runtime_safety = switch (builtin.mode) {
    .Debug, .ReleaseSafe => true,
    .ReleaseFast, .ReleaseSmall => false,
};

pub fn main() !void {
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

    if (args.len != 2) {
        std.debug.print("usage: aldrheim <xnb_path>\n", .{});
        std.process.exit(1);
    }
    const in_path = args[1];

    var xnb = try Xnb.initFromFile(gpa, in_path);
    defer xnb.deinit(gpa);

    // // TODO: temp
    // {
    //     const decompressed = try xnb.decompress(gpa);
    //     defer gpa.free(decompressed);

    //     const out_path = try std.fmt.allocPrint(gpa, "{s}.decompressed", .{in_path});
    //     defer gpa.free(out_path);
    //     var out_file = try std.fs.cwd().createFile(out_path, .{});
    //     defer out_file.close();

    //     var out_writer = out_file.writer(&.{});
    //     const writer = &out_writer.interface;
    //     try writer.writeAll(decompressed);
    //     try writer.flush();
    // }

    var content = try xnb.parseContent(gpa);
    defer content.deinit(gpa);

    if (content.primary_asset == .texture_2d) {
        const texture = content.primary_asset.texture_2d;

        const pixels = try texture.decode(gpa, 0);
        defer gpa.free(pixels);

        const out_path = try std.fmt.allocPrint(gpa, "{s}.png\x00", .{in_path});
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

    std.debug.print("{}\n", .{xnb.header});
}
