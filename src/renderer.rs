use std::{rc::Rc, sync::Arc};

use glam::Mat4;
use winit::window::Window;

use crate::{
    asset_manager::AssetManager,
    renderer::{
        camera::{Camera, Frustum},
        pipelines::{
            particles::{ParticleInstance, ParticlesPipeline},
            render_deferred_effect::RenderDeferredEffectPipeline,
            skymap::{SkymapPipeline, SkymapUniform},
        },
    },
    scene::{self, Skymap},
};

pub mod camera;
pub mod pipelines;

pub struct RenderContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_format: wgpu::TextureFormat,
    pub linear_sampler: wgpu::Sampler,
    pub placeholder_texture_view: wgpu::TextureView,

    pub vertex_storage_buffer_bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_buffer_bind_group_layout: wgpu::BindGroupLayout,
    pub texture_2d_bind_group_layout: wgpu::BindGroupLayout,
    pub texture_2d_2x_bind_group_layout: wgpu::BindGroupLayout,
    pub texture_3d_bind_group_layout: wgpu::BindGroupLayout,
}

impl RenderContext {
    pub async fn new(
        window: Arc<Window>,
    ) -> anyhow::Result<(Self, wgpu::Surface<'static>, wgpu::SurfaceConfiguration)> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_limits: wgpu::Limits {
                    max_push_constant_size: 64,
                    max_binding_array_elements_per_shader_stage: 4,
                    ..wgpu::Limits::defaults()
                },
                required_features: wgpu::Features::TEXTURE_COMPRESSION_BC
                    | wgpu::Features::PUSH_CONSTANTS
                    | wgpu::Features::TEXTURE_BINDING_ARRAY
                    // this one seems like a pretty modern feature... 
                    // maybe revisit later if compatibility with older hardware is wanted?
                    | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
        };

        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let vertex_storage_buffer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Vertex Buffer Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let uniform_buffer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniform Buffer Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let texture_2d_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture2D Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let texture_2d_2x_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture2D 2x Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let texture_3d_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture3D Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let placeholder_pixel = [0xFF, 0x00, 0xFF, 0xFF];
        let placeholder_texture_size = wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        };
        let placeholder_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Placeholder Texture"),
            size: placeholder_texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let placeholder_texture_view =
            placeholder_texture.create_view(&wgpu::TextureViewDescriptor::default());
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &placeholder_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &placeholder_pixel,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            placeholder_texture_size,
        );

        let ctx = RenderContext {
            device,
            queue,
            surface_format,
            linear_sampler,
            placeholder_texture_view,
            vertex_storage_buffer_bind_group_layout,
            uniform_buffer_bind_group_layout,
            texture_2d_bind_group_layout,
            texture_2d_2x_bind_group_layout,
            texture_3d_bind_group_layout,
            // skymap_bind_group_layout,
        };
        Ok((ctx, surface, surface_config))
    }
}

pub struct Renderer {
    pub context: Rc<RenderContext>,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    pub window: Arc<Window>,

    particles_pipeline: ParticlesPipeline,
    render_deferred_effect_pipeline: RenderDeferredEffectPipeline,
    skymap_pipeline: SkymapPipeline,

    depth_texture: wgpu::Texture,

    camera_uniform_buffer: wgpu::Buffer,
    camera_uniform_bind_group: wgpu::BindGroup,
    skymap_uniform_buffer: wgpu::Buffer,
    skymap_uniform_bind_group: wgpu::BindGroup,

    particles_instance_buffer: wgpu::Buffer,

    // holding onto allocated buffers to avoid recreating them (potentially multiple times) every frame
    draw_commands: DrawCommands,
}

impl Renderer {
    pub fn new(
        context: Rc<RenderContext>,
        window: Arc<Window>,
        surface: wgpu::Surface<'static>,
        surface_config: wgpu::SurfaceConfiguration,
        asset_manager: &mut AssetManager,
    ) -> anyhow::Result<Self> {
        let camera_uniform_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Uniform Buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_uniform_bind_group =
            context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Camera Uniform Bind Group"),
                    layout: &context.uniform_buffer_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera_uniform_buffer.as_entire_binding(),
                    }],
                });

        let skymap_uniform_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Skymap Uniform Buffer"),
            size: std::mem::size_of::<SkymapUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let skymap_uniform_bind_group =
            context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Skymap Uniform Bind Group"),
                    layout: &context.uniform_buffer_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: skymap_uniform_buffer.as_entire_binding(),
                    }],
                });

        let particles_pipeline = ParticlesPipeline::new(&context, asset_manager)?;
        let render_deferred_effect_pipeline = RenderDeferredEffectPipeline::new(&context)?;
        let skymap_pipeline = SkymapPipeline::new(&context)?;

        let depth_texture = create_depth_texture(&context.device, &surface_config);

        let particles_instance_buffer = create_particles_buffer(&context.device, 1000);

        let renderer = Renderer {
            context,
            surface,
            surface_config,
            is_surface_configured: false,
            window,

            camera_uniform_buffer,
            camera_uniform_bind_group,
            skymap_uniform_buffer,
            skymap_uniform_bind_group,

            depth_texture,

            particles_pipeline,
            render_deferred_effect_pipeline,
            skymap_pipeline,

            particles_instance_buffer,
            draw_commands: DrawCommands::new(),
        };
        Ok(renderer)
    }

    pub fn render(&mut self, camera: &Camera) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        let window_size = self.window.inner_size();
        let projection = Mat4::perspective_rh(
            camera.fov_y_radians,
            (window_size.width as f32) / (window_size.height as f32),
            camera.z_near,
            camera.z_far,
        );

        let (camera_forward, camera_right, camera_up) = camera.forward_right_up();
        let view = Mat4::look_to_rh(camera.position, camera_forward, camera_up);
        let view_proj = projection * view;

        let camera_uniform = CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
            position: [camera.position.x, camera.position.y, camera.position.z, 1.0],
            forward: [camera_forward.x, camera_forward.y, camera_forward.z, 1.0],
            right: [camera_right.x, camera_right.y, camera_right.z, 1.0],
            up: [camera_up.x, camera_up.y, camera_up.z, 1.0],
        };
        self.context.queue.write_buffer(
            &self.camera_uniform_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );

        if let Some(skymap) = &self.draw_commands.skymap {
            let skymap_uniform = SkymapUniform {
                texture_w: skymap.texture.texture.width() as f32,
                texture_h: skymap.texture.texture.height() as f32,
                target_w: self.surface_config.width as f32,
                target_h: self.surface_config.height as f32,
                color_r: skymap.color.r,
                color_g: skymap.color.g,
                color_b: skymap.color.b,
            };
            self.context.queue.write_buffer(
                &self.skymap_uniform_buffer,
                0,
                bytemuck::cast_slice(&[skymap_uniform]),
            );
        }

        if self.particles_instance_buffer.size()
            < (self.draw_commands.particles.len() * std::mem::size_of::<ParticleInstance>()) as u64
        {
            self.particles_instance_buffer = create_particles_buffer(
                &self.context.device,
                self.draw_commands.particles.len() * 2,
            );
        }
        self.context.queue.write_buffer(
            &self.particles_instance_buffer,
            0,
            bytemuck::cast_slice(&self.draw_commands.particles),
        );
        let particles_count = self.draw_commands.particles.len() as u32;

        let frustum = Frustum::new(view_proj);
        let culled_bitrees = self
            .draw_commands
            .bitrees
            .iter()
            .filter(|draw| frustum.test_aabb(&draw.node.bounding_box));

        let surface_texture = self.surface.get_current_texture()?;
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture_view = self
            .depth_texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut command_encoder = self
            .context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                label: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // render skymap
            if let Some(skymap) = &self.draw_commands.skymap {
                render_pass.set_pipeline(&self.skymap_pipeline.pipeline);
                render_pass.set_bind_group(0, &self.skymap_uniform_bind_group, &[]);
                render_pass.set_bind_group(1, &skymap.texture.bind_group, &[]);
                render_pass.draw(0..3, 0..1);
            }

            // render bitrees
            render_pass.set_pipeline(&self.render_deferred_effect_pipeline.pipeline);
            render_pass.set_bind_group(0, &self.camera_uniform_bind_group, &[]);
            for draw in culled_bitrees {
                render_pass.set_bind_group(1, &draw.node.tree.vertex_buffer_bind_group, &[]);
                render_pass.set_bind_group(
                    2,
                    &draw.node.tree.vertex_layout_uniform_bind_group,
                    &[],
                );
                render_pass.set_bind_group(3, &draw.node.tree.texture_bind_group, &[]);
                render_pass.set_push_constants(
                    wgpu::ShaderStages::VERTEX,
                    0,
                    bytemuck::cast_slice(&[draw.transform]),
                );
                render_pass.set_index_buffer(
                    draw.node.tree.index_buffer.slice(..),
                    draw.node.tree.index_format,
                );
                render_pass.draw_indexed(
                    draw.node.start_index..draw.node.start_index + draw.node.index_count,
                    0,
                    0..1,
                );
            }

            // render particles
            render_pass.set_pipeline(&self.particles_pipeline.pipeline);
            render_pass.set_bind_group(0, &self.camera_uniform_bind_group, &[]);
            render_pass.set_bind_group(1, &self.particles_pipeline.textures_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.particles_instance_buffer.slice(..));
            render_pass.draw(0..4, 0..particles_count);
        }

        self.context.queue.submit([command_encoder.finish()]);

        self.window.pre_present_notify();
        surface_texture.present();

        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface
            .configure(&self.context.device, &self.surface_config);
        self.is_surface_configured = true;

        self.depth_texture = create_depth_texture(&self.context.device, &self.surface_config);
    }

    pub fn reconfigure_surface(&mut self) {
        let size = self.window.inner_size();
        self.resize(size.width, size.height);
    }

    pub fn new_draw_commands(&mut self) -> &mut DrawCommands {
        self.draw_commands.clear();
        &mut self.draw_commands
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub position: [f32; 4],
    pub forward: [f32; 4],
    pub right: [f32; 4],
    pub up: [f32; 4],
}

pub struct DrawCommands {
    pub skymap: Option<Skymap>,
    pub bitrees: Vec<BiTreeDrawCommand>,
    pub particles: Vec<ParticleInstance>,
}

impl DrawCommands {
    pub fn new() -> Self {
        DrawCommands {
            skymap: None,
            bitrees: Vec::new(),
            particles: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.skymap = None;
        self.bitrees.clear();
        self.particles.clear();
    }

    pub fn add_bitree(&mut self, bitree: scene::BiTreeNode, transform: Mat4) {
        self.bitrees.push(BiTreeDrawCommand {
            node: bitree,
            transform,
        });
    }

    pub fn add_particles(&mut self, particles: impl IntoIterator<Item = ParticleInstance>) {
        self.particles.extend(particles);
    }
}

pub struct BiTreeDrawCommand {
    pub node: scene::BiTreeNode,
    pub transform: Mat4,
}

pub fn create_depth_texture(
    device: &wgpu::Device,
    surface_config: &wgpu::SurfaceConfiguration,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Depth Buffer"),
        size: wgpu::Extent3d {
            width: surface_config.width.max(1),
            height: surface_config.height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

pub fn create_particles_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    log::debug!("created particles buffer with capacity {capacity}");
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Particles Instance Buffer"),
        size: (capacity * std::mem::size_of::<ParticleInstance>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
