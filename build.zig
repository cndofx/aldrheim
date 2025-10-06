const std = @import("std");

pub fn build(b: *std.Build) !void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const force_llvm = b.option(bool, "llvm", "Force LLVM") orelse false;

    const sdl3_dep = b.dependency("sdl3", .{
        .target = target,
        .optimize = optimize,
    });

    const matrix_dep = b.dependency("matrix", .{
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
        .use_llvm = if (force_llvm) true else null,
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/main.zig"),
            .target = target,
            .optimize = optimize,
            .imports = &.{
                .{ .name = "c", .module = translate_c_mod },
                .{ .name = "sdl3", .module = sdl3_dep.module("sdl3") },
                .{ .name = "matrix", .module = matrix_dep.module("zig_matrix") },
            },
        }),
    });
    b.installArtifact(exe);

    const shader_target = blk: {
        switch (target.result.os.tag) {
            .linux => break :blk "spirv",
            .windows => break :blk "dxil",
            .macos => break :blk "metallib",
            else => return error.UnsupportedOs,
        }
    };
    const shader_target_is_spirv = std.mem.eql(u8, shader_target, "spirv");
    for (shader_sources) |source| {
        const input_path = try std.fmt.allocPrint(b.allocator, "src/shaders/{s}", .{source.path});

        if (source.vertex_entry) |vertex_entry| {
            const output_path = try std.fmt.allocPrint(b.allocator, "{s}.vert", .{source.path[0 .. source.path.len - 6]});
            const compile_command = b.addSystemCommand(&.{"slangc"});
            compile_command.addFileArg(b.path(input_path));
            compile_command.addArg("-target");
            compile_command.addArg(shader_target);
            if (shader_target_is_spirv) {
                compile_command.addArg("-emit-spirv-via-glsl");
                compile_command.addArg("-profile");
                compile_command.addArg("spirv_1_0");
            }
            compile_command.addArg("-entry");
            compile_command.addArg(vertex_entry);
            compile_command.addArg("-o");
            const output = compile_command.addOutputFileArg(output_path);
            exe.root_module.addAnonymousImport(output_path, .{
                .root_source_file = output,
            });
        }

        if (source.fragment_entry) |fragment_entry| {
            const output_path = try std.fmt.allocPrint(b.allocator, "{s}.frag", .{source.path[0 .. source.path.len - 6]});
            const compile_command = b.addSystemCommand(&.{"slangc"});
            compile_command.addFileArg(b.path(input_path));
            compile_command.addArg("-target");
            compile_command.addArg(shader_target);
            if (shader_target_is_spirv) {
                compile_command.addArg("-emit-spirv-via-glsl");
                compile_command.addArg("-profile");
                compile_command.addArg("spirv_1_0");
            }
            compile_command.addArg("-entry");
            compile_command.addArg(fragment_entry);
            compile_command.addArg("-o");
            const output = compile_command.addOutputFileArg(output_path);
            exe.root_module.addAnonymousImport(output_path, .{
                .root_source_file = output,
            });
        }
    }

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

const ShaderSource = struct {
    path: []const u8,
    vertex_entry: ?[]const u8 = null,
    fragment_entry: ?[]const u8 = null,
};

const shader_sources = [_]ShaderSource{ShaderSource{
    .path = "TexturedQuad.slang",
    .vertex_entry = "VertexMain",
    .fragment_entry = "FragmentMain",
}};
