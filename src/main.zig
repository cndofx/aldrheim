const builtin = @import("builtin");
const std = @import("std");
const c = @import("c");

const Xnb = @import("xnb/Xnb.zig");

pub const runtime_safety = switch (builtin.mode) {
    .Debug, .ReleaseSafe => true,
    .ReleaseFast, .ReleaseSmall => false,
};

pub fn main() !void {
    var debug_allocator: std.heap.DebugAllocator(.{}) = .init;
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

    var xnb = try Xnb.initFromFile(gpa, args[1]);
    defer xnb.deinit(gpa);

    var content = try xnb.parseContent(gpa);
    defer content.deinit();

    std.debug.print("{}\n", .{xnb.header});
}
