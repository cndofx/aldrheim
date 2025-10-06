const std = @import("std");

const vk = @import("vulkan");
const sdl = @import("sdl3");

const runtime_safety = @import("main.zig").runtime_safety;

const target_api_version: u32 = @bitCast(vk.API_VERSION_1_3);
const required_instance_extensions = [_][*:0]const u8{};
const required_device_extensions = [_][*:0]const u8{
    vk.extensions.khr_swapchain.name,
};
const validation_layers = [_][*:0]const u8{
    "VK_LAYER_KHRONOS_validation",
};

const Renderer = @This();

vkb: *vk.BaseWrapper,
vki: *vk.InstanceWrapper,
vkd: *vk.DeviceWrapper,
instance: vk.Instance,
physical_device: vk.PhysicalDevice,
graphics_queue_family_index: u32,
present_queue_family_index: u32,
device: vk.Device,
graphics_queue: vk.Queue,
present_queue: vk.Queue, // if null, present happens on graphics queue
surface: vk.SurfaceKHR,

pub fn init(gpa: std.mem.Allocator, window: sdl.video.Window) !Renderer {
    const vkb = try gpa.create(vk.BaseWrapper);
    errdefer gpa.destroy(vkb);
    const vkGetInstanceProcAddr: vk.PfnGetInstanceProcAddr = @ptrCast(try sdl.vulkan.getVkGetInstanceProcAddr());
    vkb.* = vk.BaseWrapper.load(vkGetInstanceProcAddr);

    const vki = try gpa.create(vk.InstanceWrapper);
    errdefer gpa.destroy(vki);
    const instance = try createInstance(vkb, gpa);
    vki.* = vk.InstanceWrapper.load(instance, vkb.dispatch.vkGetInstanceProcAddr.?);
    errdefer vki.destroyInstance(instance, null);

    const sdl_vk_surface = try sdl.vulkan.Surface.init(window, @ptrFromInt(@intFromEnum(instance)), null);
    const surface: vk.SurfaceKHR = @enumFromInt(@intFromPtr(sdl_vk_surface.surface));
    errdefer vki.destroySurfaceKHR(instance, surface, null); // TODO: is this safe or does it have to go through SDL?

    const physical_device_and_queue_family_indices = try pickPhysicalDevice(vki, instance, gpa);
    const physical_device = physical_device_and_queue_family_indices.physical_device;
    const graphics_queue_family_index = physical_device_and_queue_family_indices.graphics_queue_family_index;
    const present_queue_family_index = physical_device_and_queue_family_indices.present_queue_family_index;

    const vkd = try gpa.create(vk.DeviceWrapper);
    errdefer gpa.destroy(vkd);
    const device = try createDevice(vki, physical_device_and_queue_family_indices);
    vkd.* = vk.DeviceWrapper.load(device, vki.dispatch.vkGetDeviceProcAddr.?);
    errdefer vkd.destroyDevice(device, null);

    const graphics_queue = vkd.getDeviceQueue(device, graphics_queue_family_index, 0);
    var present_queue: vk.Queue = .null_handle;
    if (graphics_queue_family_index != present_queue_family_index) {
        present_queue = vkd.getDeviceQueue(device, present_queue_family_index, 0);
    }

    return Renderer{
        .vkb = vkb,
        .vki = vki,
        .vkd = vkd,
        .instance = instance,
        .physical_device = physical_device,
        .graphics_queue_family_index = graphics_queue_family_index,
        .present_queue_family_index = present_queue_family_index,
        .device = device,
        .graphics_queue = graphics_queue,
        .present_queue = present_queue,
        .surface = surface,
    };
}

pub fn deinit(self: *Renderer, gpa: std.mem.Allocator) void {
    self.vki.destroySurfaceKHR(self.instance, self.surface, null);
    self.vkd.destroyDevice(self.device, null);
    self.vki.destroyInstance(self.instance, null);

    gpa.destroy(self.vkb);
    gpa.destroy(self.vki);
    gpa.destroy(self.vkd);

    self.* = undefined;
}

fn createInstance(vkb: *const vk.BaseWrapper, gpa: std.mem.Allocator) !vk.Instance {
    const supported_api_version = try vkb.enumerateInstanceVersion();
    if (supported_api_version < target_api_version) {
        return error.VulkanApiVersionUnsupported;
    }

    // extensions

    var extensions = try std.ArrayList([*:0]const u8).initCapacity(gpa, required_instance_extensions.len);
    defer extensions.deinit(gpa);

    try extensions.appendSlice(gpa, &required_instance_extensions);

    const sdl_extensions = try sdl.vulkan.getInstanceExtensions();
    try extensions.appendSlice(gpa, sdl_extensions);

    if (extensions.items.len > 0) {
        std.debug.print("required instance extensions:\n", .{});
        for (extensions.items) |ext| {
            std.debug.print("  {s}\n", .{ext});
        }
        std.debug.print("\n", .{});
    }

    try checkInstanceExtensionsSupported(vkb, gpa, null, extensions.items);

    // layers

    var layers = std.ArrayList([*:0]const u8){};
    defer layers.deinit(gpa);

    if (runtime_safety) {
        try layers.appendSlice(gpa, &validation_layers);
    }

    if (layers.items.len > 0) {
        std.debug.print("required instance layers:\n", .{});
        for (layers.items) |layer| {
            std.debug.print("  {s}\n", .{layer});
        }
        std.debug.print("\n", .{});
    }

    try checkInstanceLayersSupported(vkb, gpa, layers.items);

    // create instance

    const app_info = vk.ApplicationInfo{
        .api_version = target_api_version,
        .p_application_name = "Aldrheim",
        .application_version = @bitCast(vk.makeApiVersion(0, 0, 0, 0)),
        .p_engine_name = "No Engine",
        .engine_version = @bitCast(vk.makeApiVersion(0, 0, 0, 0)),
    };

    const create_info = vk.InstanceCreateInfo{
        .p_application_info = &app_info,
        .enabled_extension_count = @intCast(extensions.items.len),
        .pp_enabled_extension_names = extensions.items.ptr,
        .enabled_layer_count = @intCast(layers.items.len),
        .pp_enabled_layer_names = layers.items.ptr,
    };

    const instance = try vkb.createInstance(&create_info, null);
    return instance;
}

fn checkInstanceExtensionsSupported(vkb: *const vk.BaseWrapper, gpa: std.mem.Allocator, layer: ?[*:0]const u8, extensions: []const [*:0]const u8) !void {
    var properties_count: u32 = 0;
    _ = try vkb.enumerateInstanceExtensionProperties(layer, &properties_count, null);

    const properties = try gpa.alloc(vk.ExtensionProperties, properties_count);
    defer gpa.free(properties);
    _ = try vkb.enumerateInstanceExtensionProperties(layer, &properties_count, properties.ptr);

    std.debug.print("supported instance extensions:\n", .{});
    for (properties) |ext_props| {
        std.debug.print("  {s}\n", .{&ext_props.extension_name});
    }
    std.debug.print("\n", .{});

    for (extensions) |required_ext| {
        var have_ext = false;
        for (properties) |ext_props| {
            if (std.mem.orderZ(u8, @ptrCast(&ext_props.extension_name), required_ext) == .eq) {
                have_ext = true;
                break;
            }
        }
        if (have_ext == false) {
            std.debug.print("error: required extension unsupported: {s}\n", .{required_ext});
            return error.RequiredExtensionUnsupported;
        }
    }
}

fn checkInstanceLayersSupported(vkb: *const vk.BaseWrapper, gpa: std.mem.Allocator, layers: []const [*:0]const u8) !void {
    var properties_count: u32 = 0;
    _ = try vkb.enumerateInstanceLayerProperties(&properties_count, null);

    const properties = try gpa.alloc(vk.LayerProperties, properties_count);
    defer gpa.free(properties);
    _ = try vkb.enumerateInstanceLayerProperties(&properties_count, properties.ptr);

    std.debug.print("supported instance layers:\n", .{});
    for (properties) |layer_props| {
        std.debug.print("  {s}\n", .{&layer_props.layer_name});
    }
    std.debug.print("\n", .{});

    for (layers) |required_layer| {
        var have_layer = false;
        for (properties) |layer_props| {
            if (std.mem.orderZ(u8, @ptrCast(&layer_props.layer_name), required_layer) == .eq) {
                have_layer = true;
                break;
            }
        }
        if (have_layer == false) {
            std.debug.print("error: required layer unsupported: {s}\n", .{required_layer});
            return error.RequiredLayerUnsupported;
        }
    }
}

const PhysicalDeviceAndQueueFamilyIndices = struct {
    physical_device: vk.PhysicalDevice,
    graphics_queue_family_index: u32,
    present_queue_family_index: u32,
};

fn pickPhysicalDevice(vki: *const vk.InstanceWrapper, instance: vk.Instance, gpa: std.mem.Allocator) !PhysicalDeviceAndQueueFamilyIndices {
    var discrete: ?PhysicalDeviceAndQueueFamilyIndices = null;
    var fallback: ?PhysicalDeviceAndQueueFamilyIndices = null;

    var physical_device_count: u32 = 0;
    _ = try vki.enumeratePhysicalDevices(instance, &physical_device_count, null);

    const physical_devices = try gpa.alloc(vk.PhysicalDevice, physical_device_count);
    defer gpa.free(physical_devices);
    _ = try vki.enumeratePhysicalDevices(instance, &physical_device_count, physical_devices.ptr);

    for (physical_devices) |physical_device| {
        const device_props = vki.getPhysicalDeviceProperties(physical_device);
        if (device_props.api_version < target_api_version) {
            continue;
        }

        var graphics_and_present_index: ?u32 = null;
        var graphics_only_index: ?u32 = null;
        var present_only_index: ?u32 = null;

        var queue_family_properties_count: u32 = 0;
        _ = vki.getPhysicalDeviceQueueFamilyProperties(physical_device, &queue_family_properties_count, null);

        const queue_family_properties = try gpa.alloc(vk.QueueFamilyProperties, queue_family_properties_count);
        defer gpa.free(queue_family_properties);
        _ = vki.getPhysicalDeviceQueueFamilyProperties(physical_device, &queue_family_properties_count, queue_family_properties.ptr);

        for (queue_family_properties, 0..) |family_props, family_index| {
            const supports_graphics = family_props.queue_flags.graphics_bit;
            const supports_present = sdl.vulkan.getPresentationSupport(
                @ptrFromInt(@intFromEnum(instance)),
                @ptrFromInt(@intFromEnum(physical_device)),
                @intCast(family_index),
            );

            if (supports_graphics and supports_present and graphics_and_present_index == null) {
                graphics_and_present_index = @intCast(family_index);
            } else if (supports_graphics and graphics_only_index == null) {
                graphics_only_index = @intCast(family_index);
            } else if (supports_present and present_only_index == null) {
                present_only_index = @intCast(family_index);
            }
        }

        var graphics_index: ?u32 = null;
        var present_index: ?u32 = null;
        if (graphics_and_present_index) |gp| {
            graphics_index = gp;
            present_index = gp;
        } else {
            graphics_index = graphics_only_index;
            present_index = present_only_index;
        }

        if (graphics_index == null or present_index == null) {
            continue;
        }

        if (discrete == null and device_props.device_type == .discrete_gpu) {
            discrete = PhysicalDeviceAndQueueFamilyIndices{
                .physical_device = physical_device,
                .graphics_queue_family_index = graphics_index.?,
                .present_queue_family_index = present_index.?,
            };
        }

        if (fallback == null) {
            fallback = PhysicalDeviceAndQueueFamilyIndices{
                .physical_device = physical_device,
                .graphics_queue_family_index = graphics_index.?,
                .present_queue_family_index = present_index.?,
            };
        }
    }

    if (discrete) |d| {
        return d;
    }

    if (fallback) |f| {
        return f;
    }

    return error.NoSupportedGPU;
}

fn createDevice(vki: *const vk.InstanceWrapper, physical_device_and_queue_family_indices: PhysicalDeviceAndQueueFamilyIndices) !vk.Device {
    const physical_device = physical_device_and_queue_family_indices.physical_device;
    const graphics_queue_family_index = physical_device_and_queue_family_indices.graphics_queue_family_index;
    const present_queue_family_index = physical_device_and_queue_family_indices.present_queue_family_index;

    const queue_priorities = [1]f32{1.0};

    const device_queue_create_info_count: u32 = if (graphics_queue_family_index == present_queue_family_index) 1 else 2;
    const device_queue_create_infos = [2]vk.DeviceQueueCreateInfo{
        vk.DeviceQueueCreateInfo{
            .queue_family_index = graphics_queue_family_index,
            .queue_count = queue_priorities.len,
            .p_queue_priorities = &queue_priorities,
        },
        vk.DeviceQueueCreateInfo{
            .queue_family_index = present_queue_family_index,
            .queue_count = queue_priorities.len,
            .p_queue_priorities = &queue_priorities,
        },
    };

    var features_vk13 = vk.PhysicalDeviceVulkan13Features{
        .dynamic_rendering = .true,
    };

    var features = vk.PhysicalDeviceFeatures2{
        .p_next = &features_vk13,
        .features = vk.PhysicalDeviceFeatures{},
    };

    const device_create_info = vk.DeviceCreateInfo{
        .p_next = &features,
        .queue_create_info_count = device_queue_create_info_count,
        .p_queue_create_infos = &device_queue_create_infos,
        .enabled_extension_count = required_device_extensions.len,
        .pp_enabled_extension_names = &required_device_extensions,
    };

    const device = try vki.createDevice(physical_device, &device_create_info, null);
    return device;
}
