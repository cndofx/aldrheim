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

fn run(gpa: std.mem.Allocator, magicka_path: []const u8) !void {
    std.debug.print("magicka path: {s}\n", .{magicka_path});

    // const source_path = try std.fmt.allocPrint(gpa, "{s}/Content/Levels/Textures/Surface/Nature/Ground/dirt00_0.xnb", .{magicka_path});
    // const source_path = try std.fmt.allocPrint(gpa, "{s}/Content/Models/Bosses/assatur/assatur_0.xnb", .{magicka_path});
    const source_path = try std.fmt.allocPrint(gpa, "{s}/Content/UI/Menu/CampaignMap.xnb", .{magicka_path});
    defer gpa.free(source_path);
    var source_xnb = try Xnb.initFromFile(gpa, source_path);
    defer source_xnb.deinit(gpa);
    var source_content = try source_xnb.parseContent(gpa);
    defer source_content.deinit(gpa);
    const source_texture = source_content.primary_asset.texture_2d;
    std.debug.print("texture width: {}, height: {}\n", .{ source_texture.width, source_texture.height });

    // basic sdl init

    try sdl.hints.set(.app_id, "cndofx.Aldrheim");
    try sdl.hints.set(.app_name, "Aldrheim");

    const sdl_init_flags = sdl.InitFlags{ .events = true, .video = true };
    try sdl.init(sdl_init_flags);
    defer sdl.quit(sdl_init_flags);

    const window = try sdl.video.Window.init("Aldrheim", 1280, 720, .{ .resizable = true });
    defer window.deinit();

    const device = try sdl.gpu.Device.init(.{ .spirv = true, .dxil = true, .metal_lib = true }, runtime_safety, null);
    defer device.deinit();
    try device.claimWindow(window);
    defer device.releaseWindow(window);

    // create gpu pipeline

    const vert_shader = try loadShader(device, @embedFile("TexturedQuad.vert"), "main", .vertex, 0);
    defer device.releaseShader(vert_shader);
    const frag_shader = try loadShader(device, @embedFile("TexturedQuad.frag"), "main", .fragment, 1);
    defer device.releaseShader(frag_shader);

    const pipeline_create_info = sdl.gpu.GraphicsPipelineCreateInfo{
        .target_info = sdl.gpu.GraphicsPipelineTargetInfo{
            .color_target_descriptions = &.{
                sdl.gpu.ColorTargetDescription{
                    .format = device.getSwapchainTextureFormat(window),
                },
            },
        },
        .vertex_input_state = sdl.gpu.VertexInputState{
            .vertex_buffer_descriptions = &.{
                sdl.gpu.VertexBufferDescription{
                    .slot = 0,
                    .input_rate = .vertex,
                    .pitch = @sizeOf(Vertex),
                },
            },
            .vertex_attributes = &.{
                sdl.gpu.VertexAttribute{
                    .buffer_slot = 0,
                    .format = .f32x3,
                    .location = 0,
                    .offset = 0,
                },
                sdl.gpu.VertexAttribute{
                    .buffer_slot = 0,
                    .format = .f32x2,
                    .location = 1,
                    .offset = @sizeOf(f32) * 3,
                },
            },
        },
        .primitive_type = .triangle_list,
        .vertex_shader = vert_shader,
        .fragment_shader = frag_shader,
    };

    const pipeline = try device.createGraphicsPipeline(pipeline_create_info);
    defer device.releaseGraphicsPipeline(pipeline);

    // create gpu buffers

    // const vertex_buffer_create_info = sdl.gpu.BufferCreateInfo{
    //     .usage = .{ .vertex = true },
    //     .size = @sizeOf(Vertex) * 4,
    //     .props = .{
    //         .name = "Quad Vertex Buffer!",
    //     },
    // };
    // const vertex_buffer = try device.createBuffer(vertex_buffer_create_info);
    // defer device.releaseBuffer(vertex_buffer);

    // const index_buffer_create_info = sdl.gpu.BufferCreateInfo{
    //     .usage = .{ .index = true },
    //     .size = @sizeOf(u16) * 6,
    //     .props = .{
    //         .name = "Quad Index Buffer!",
    //     },
    // };
    // const index_buffer = try device.createBuffer(index_buffer_create_info);
    // defer device.releaseBuffer(index_buffer);

    const vertex_data = [4]Vertex{
        Vertex{
            .x = -1,
            .y = 1,
            .z = 0,
            .u = 0,
            .v = 0,
        },
        Vertex{
            .x = 1,
            .y = 1,
            .z = 0,
            .u = 1,
            .v = 0,
        },
        Vertex{
            .x = 1,
            .y = -1,
            .z = 0,
            .u = 1,
            .v = 1,
        },
        Vertex{
            .x = -1,
            .y = -1,
            .z = 0,
            .u = 0,
            .v = 1,
        },
    };
    const vertex_data_bytes: []const u8 = @as([*]const u8, @ptrCast(&vertex_data))[0 .. @sizeOf(Vertex) * vertex_data.len];

    const index_data = [6]u16{
        0, 1, 2, 0, 2, 3,
    };
    const index_data_bytes: []const u8 = @as([*]const u8, @ptrCast(&index_data))[0 .. @sizeOf(u16) * index_data.len];

    const vertex_buffer = try uploadBuffer(device, vertex_data_bytes, .{ .vertex = true });
    defer device.releaseBuffer(vertex_buffer);

    const index_buffer = try uploadBuffer(device, index_data_bytes, .{ .index = true });
    defer device.releaseBuffer(index_buffer);

    // create gpu texture

    // if (device.textureSupportsFormat(try dirt_texture.format.toSdlTextureFormat(), .two_dimensional, .{ .sampler = true }) == false) {
    //     return error.UnsupportedGpuTextureFormat;
    // }

    // const texture_create_info = sdl.gpu.TextureCreateInfo{
    //     .texture_type = .two_dimensional,
    //     .format = try dirt_texture.format.toSdlTextureFormat(),
    //     .usage = .{ .sampler = true },
    //     .width = dirt_texture.width,
    //     .height = dirt_texture.height,
    //     .layer_count_or_depth = 1,
    //     .num_levels = 1,
    //     .props = .{
    //         .name = "Quad Texture!",
    //     },
    // };
    // const texture = try device.createTexture(texture_create_info);
    // defer device.releaseTexture(texture);

    // {
    //     const tansfer_buffer_create_info = sdl.gpu.TransferBufferCreateInfo{
    //         .usage = .upload,
    //         .size = @intCast(dirt_texture.mips[0].len),
    //     };
    //     const texture_transfer_buffer = try device.createTransferBuffer(tansfer_buffer_create_info);
    //     defer device.releaseTransferBuffer(texture_transfer_buffer);

    //     const texture_transfer_ptr = try device.mapTransferBuffer(texture_transfer_buffer, false);
    //     @memcpy(texture_transfer_ptr, dirt_texture.mips[0]);
    //     device.unmapTransferBuffer(texture_transfer_buffer);

    //     const upload_command_buffer = try device.acquireCommandBuffer();
    //     const copy_pass = upload_command_buffer.beginCopyPass();

    //     const texture_transfer_info = sdl.gpu.TextureTransferInfo{
    //         .transfer_buffer = texture_transfer_buffer,
    //         .offset = 0,
    //     };
    //     const destination_region = sdl.gpu.TextureRegion{
    //         .texture = texture,
    //         .width = dirt_texture.width,
    //         .height = dirt_texture.height,
    //         .depth = 1,
    //     };
    //     copy_pass.uploadToTexture(texture_transfer_info, destination_region, false);

    //     copy_pass.end();
    //     try upload_command_buffer.submit();
    // }

    const texture = try uploadTexture2d(
        device,
        source_texture.mips[0],
        source_texture.width,
        source_texture.height,
        try source_texture.format.toSdlTextureFormat(),
        .{ .sampler = true },
    );
    defer device.releaseTexture(texture);

    const sampler_create_info = sdl.gpu.SamplerCreateInfo{
        .min_filter = .nearest,
        .mag_filter = .nearest,
        .mipmap_mode = .nearest,
        .address_mode_u = .clamp_to_edge,
        .address_mode_v = .clamp_to_edge,
        .address_mode_w = .clamp_to_edge,
    };
    const sampler = try device.createSampler(sampler_create_info);
    defer device.releaseSampler(sampler);

    // main loop

    var running = true;
    while (running) {
        const command_buffer = try device.acquireCommandBuffer();
        const swapchain_texture = try command_buffer.waitAndAcquireSwapchainTexture(window);
        const target_texture = swapchain_texture.texture.?;

        const color_target_infos = [1]sdl.gpu.ColorTargetInfo{
            sdl.gpu.ColorTargetInfo{
                .texture = target_texture,
                .clear_color = sdl.pixels.FColor{
                    .r = 0.1,
                    .g = 0.2,
                    .b = 0.3,
                    .a = 1.0,
                },
                .load = .clear,
                .store = .store,
            },
        };

        const render_pass = command_buffer.beginRenderPass(&color_target_infos, null);
        render_pass.bindGraphicsPipeline(pipeline);
        const vertex_buffer_bindings = [1]sdl.gpu.BufferBinding{
            sdl.gpu.BufferBinding{
                .buffer = vertex_buffer,
                .offset = 0,
            },
        };
        render_pass.bindVertexBuffers(0, &vertex_buffer_bindings);
        const index_buffer_binding = sdl.gpu.BufferBinding{
            .buffer = index_buffer,
            .offset = 0,
        };
        render_pass.bindIndexBuffer(index_buffer_binding, .indices_16bit);
        const sampler_bindings = [1]sdl.gpu.TextureSamplerBinding{
            sdl.gpu.TextureSamplerBinding{
                .sampler = sampler,
                .texture = texture,
            },
        };
        render_pass.bindFragmentSamplers(0, &sampler_bindings);
        render_pass.drawIndexedPrimitives(index_data.len, 1, 0, 0, 0);
        render_pass.end();

        try command_buffer.submit();

        while (sdl.events.poll()) |event| {
            switch (event) {
                .quit => running = false,
                else => {},
            }
        }
    }
}

fn loadShader(device: sdl.gpu.Device, source: []const u8, entry_point: [:0]const u8, stage: sdl.gpu.ShaderStage, num_samplers: u32) !sdl.gpu.Shader {
    const format = comptime blk: {
        var f = sdl.gpu.ShaderFormatFlags{};
        if (builtin.os.tag == .linux) {
            f.spirv = true;
        } else if (builtin.os.tag == .windows) {
            f.dxil = true;
        } else if (builtin.os.tag == .macos) {
            f.metal_lib = true;
        } else {
            @compileError("unsupported os");
        }
        break :blk f;
    };

    const shader_create_info = sdl.gpu.ShaderCreateInfo{
        .code = source,
        .entry_point = entry_point,
        .stage = stage,
        .format = format,
        .num_samplers = num_samplers,
    };

    const shader = try device.createShader(shader_create_info);
    return shader;
}

fn uploadBuffer(device: sdl.gpu.Device, data: []const u8, usage: sdl.gpu.BufferUsageFlags) !sdl.gpu.Buffer {
    const buffer_create_info = sdl.gpu.BufferCreateInfo{
        .usage = usage,
        .size = @intCast(data.len),
        .props = .{
            .name = "My Buffer!",
        },
    };
    const buffer = try device.createBuffer(buffer_create_info);
    errdefer device.releaseBuffer(buffer);

    const transfer_buffer_create_info = sdl.gpu.TransferBufferCreateInfo{
        .usage = .upload,
        .size = @intCast(data.len),
    };
    const transfer_buffer = try device.createTransferBuffer(transfer_buffer_create_info);
    defer device.releaseTransferBuffer(transfer_buffer);

    const transfer_ptr = try device.mapTransferBuffer(transfer_buffer, false);
    @memcpy(transfer_ptr, data);
    device.unmapTransferBuffer(transfer_buffer);

    const upload_command_buffer = try device.acquireCommandBuffer();
    const copy_pass = upload_command_buffer.beginCopyPass();

    const source = sdl.gpu.TransferBufferLocation{
        .transfer_buffer = transfer_buffer,
        .offset = 0,
    };
    const destination = sdl.gpu.BufferRegion{
        .buffer = buffer,
        .offset = 0,
        .size = @intCast(data.len),
    };
    copy_pass.uploadToBuffer(source, destination, false);

    copy_pass.end();
    try upload_command_buffer.submit();

    return buffer;
}

fn uploadTexture2d(
    device: sdl.gpu.Device,
    data: []const u8,
    width: u32,
    height: u32,
    format: sdl.gpu.TextureFormat,
    usage: sdl.gpu.TextureUsageFlags,
) !sdl.gpu.Texture {
    if (device.textureSupportsFormat(format, .two_dimensional, usage) == false) {
        return error.UnsupportedGpuTextureFormat;
    }

    const texture_create_info = sdl.gpu.TextureCreateInfo{
        .texture_type = .two_dimensional,
        .width = width,
        .height = height,
        .format = format,
        .usage = usage,
        .layer_count_or_depth = 1,
        .num_levels = 1,
        .props = .{
            .name = "My Texture!",
        },
    };
    const texture = try device.createTexture(texture_create_info);
    errdefer device.releaseTexture(texture);

    const tansfer_buffer_create_info = sdl.gpu.TransferBufferCreateInfo{
        .usage = .upload,
        .size = @intCast(data.len),
    };
    const transfer_buffer = try device.createTransferBuffer(tansfer_buffer_create_info);
    defer device.releaseTransferBuffer(transfer_buffer);

    const transfer_ptr = try device.mapTransferBuffer(transfer_buffer, false);
    @memcpy(transfer_ptr, data);
    device.unmapTransferBuffer(transfer_buffer);

    const upload_command_buffer = try device.acquireCommandBuffer();
    const copy_pass = upload_command_buffer.beginCopyPass();

    const transfer_info = sdl.gpu.TextureTransferInfo{
        .transfer_buffer = transfer_buffer,
        .offset = 0,
    };
    const destination_region = sdl.gpu.TextureRegion{
        .texture = texture,
        .width = width,
        .height = height,
        .depth = 1,
    };
    copy_pass.uploadToTexture(transfer_info, destination_region, false);

    copy_pass.end();
    try upload_command_buffer.submit();

    return texture;
}

const Vertex = extern struct {
    x: f32,
    y: f32,
    z: f32,
    u: f32,
    v: f32,
};
