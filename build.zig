const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const use_llvm = b.option(bool, "llvm", "Use LLVM") orelse false;

    const sdl3_dep = b.dependency("sdl3", .{
        .target = target,
        .optimize = optimize,
    });

    const stb_image = b.addLibrary(.{
        .name = "stb_image",
        .linkage = .static,
        .root_module = b.createModule(.{
            .target = target,
            .optimize = optimize,
            .link_libc = true,
        }),
    });
    stb_image.root_module.addCSourceFile(.{
        .file = b.path("src/c/stb_image_impl.c"),
    });

    const translate_c = b.addTranslateC(.{
        .target = target,
        .optimize = optimize,
        .root_source_file = b.path("src/c/root.h"),
    });
    translate_c.addIncludePath(b.path("src/c"));
    const translate_c_mod = translate_c.createModule();
    translate_c_mod.linkLibrary(stb_image);

    const exe = b.addExecutable(.{
        .name = "aldrheim",
        .use_llvm = use_llvm,
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/main.zig"),
            .target = target,
            .optimize = optimize,
            .imports = &.{
                .{ .name = "c", .module = translate_c_mod },
                .{ .name = "sdl3", .module = sdl3_dep.module("sdl3") },
            },
        }),
    });
    b.installArtifact(exe);

    const run_step = b.step("run", "Run the app");
    const run_cmd = b.addRunArtifact(exe);
    run_step.dependOn(&run_cmd.step);
    run_cmd.step.dependOn(b.getInstallStep());
    if (b.args) |args| {
        run_cmd.addArgs(args);
    }

    const test_step = b.step("test", "Run tests");
    const exe_tests = b.addTest(.{
        .root_module = exe.root_module,
    });
    const run_exe_tests = b.addRunArtifact(exe_tests);
    test_step.dependOn(&run_exe_tests.step);
}
