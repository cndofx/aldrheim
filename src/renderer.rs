use std::{cmp::Ordering, rc::Rc, sync::Arc};

use glam::Mat4;
use winit::window::Window;

use crate::{
    asset_manager::AssetManager,
    renderer::{
        camera::{Camera, Frustum},
        pipelines::{
            particles::ParticlesPipeline, render_deferred_effect::RenderDeferredEffectPipeline,
        },
    },
    scene::{self, vfx::VisualEffectNodeRenderable},
    xnb::asset::model::{BoundingBox, BoundingSphere},
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
    pub texture_2d_2x_bind_group_layout: wgpu::BindGroupLayout,
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
            texture_2d_2x_bind_group_layout,
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

    camera_uniform_buffer: wgpu::Buffer,
    camera_uniform_bind_group: wgpu::BindGroup,

    depth_texture: wgpu::Texture,

    particles_pipeline: ParticlesPipeline,
    render_deferred_effect_pipeline: RenderDeferredEffectPipeline,
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

        let particles_pipeline = ParticlesPipeline::new(&context, asset_manager)?;
        let render_deferred_effect_pipeline = RenderDeferredEffectPipeline::new(&context)?;

        let depth_texture = create_depth_texture(&context.device, &surface_config);

        let renderer = Renderer {
            context,
            surface,
            surface_config,
            is_surface_configured: false,
            window,

            camera_uniform_buffer,
            camera_uniform_bind_group,

            depth_texture,

            particles_pipeline,
            render_deferred_effect_pipeline,
        };
        Ok(renderer)
    }

    pub fn render(
        &mut self,
        draw_commands: &[DrawCommand],
        camera: &Camera,
    ) -> Result<(), wgpu::SurfaceError> {
        // TODO: next required features
        // - support texture alpha (mostly for foliage, binary alpha, no blending needed?)
        // - some surfaces are supposed to be a blend between different textures (eg stone and dirt)
        // - primitive liquid rendering for now just so there is no empty void, proper material can come later
        // - render background image over clear color if present ("skymap" in level xml?)

        // TODO: future optimizations
        // performance is currently *fine*, and maybe will continue to be fine,
        // but early profiling seems to show that rendering performance is heavily
        // bottlenecked by memory access latency, seemingly due to my method of
        // storing vertex data in a storage buffer and looking up vertex data dynamically
        // based on a uniform. some alternative options are:
        // - create multiple versions of the pipeline for different vertex layouts (probably not a great idea, too many possibilities)
        // - at load time, parse vertex data and transcode it into a fixed layout that the pipeline can optimize for (possible memory overhead for unused attributes)
        // - at load time, parse vertex data into multiple vertex buffers and only bind ones that are used (uncertain tradeoffs?)
        // maybe throw in zeux/meshoptimizer too if we're preprocessing vertex data anyway

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

        // let pre_cull_draw_count = draw_commands.len();
        let frustum = Frustum::new(view_proj);
        let mut culled_draw_commands = draw_commands
            .iter()
            .filter(|draw| {
                let Some(bounds) = &draw.bounds else {
                    return true;
                };

                // TODO: transform bounds from local space to world space

                match bounds {
                    RenderableBounds::Box(bounding_box) => frustum.test_aabb(bounding_box),
                    RenderableBounds::Sphere(bounding_sphere) => {
                        frustum.test_sphere(bounding_sphere)
                    }
                }
            })
            .collect::<Vec<_>>();

        // make sure everything that needs alpha blending is drawn last
        // TODO: this sucks though, this value and the pipeline in use
        // by the draw should be more closely tied so they cant be mismatched
        culled_draw_commands.sort_unstable_by(|a, b| {
            match (
                a.renderable.needs_alpha_blending(),
                b.renderable.needs_alpha_blending(),
            ) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => Ordering::Equal,
            }
        });

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

            // let mut draw_count = 0;
            for draw in culled_draw_commands {
                // draw_count += 1;
                // TODO: this assumes that all pipelines use the same bind groups and does no sorting or batching

                // render_pass.set_pipeline(&draw.model.pipeline);
                // render_pass.set_bind_group(0, &draw.model.vertex_buffer_bind_group, &[]);
                // render_pass.set_bind_group(1, &draw.model.vertex_layout_uniform_bind_group, &[]);
                // render_pass.set_bind_group(2, &draw.model.texture.bind_group, &[]);
                // render_pass
                //     .set_index_buffer(draw.model.index_buffer.slice(..), draw.model.index_format);

                // let mvp = projection * view * draw.transform;
                // render_pass.set_push_constants(
                //     wgpu::ShaderStages::VERTEX,
                //     0,
                //     bytemuck::cast_slice(&[mvp]),
                // );

                // render_pass.draw_indexed(
                //     draw.model.start_index..draw.model.start_index + draw.model.index_count,
                //     draw.model.base_vertex as i32,
                //     0..1,
                // );

                match &draw.renderable {
                    Renderable::Model(model) => todo!(),
                    Renderable::BiTreeNode(bitree_node) => {
                        render_pass.set_pipeline(&self.render_deferred_effect_pipeline.pipeline);
                        render_pass.set_bind_group(0, &self.camera_uniform_bind_group, &[]);
                        render_pass.set_bind_group(
                            1,
                            &bitree_node.tree.vertex_buffer_bind_group,
                            &[],
                        );
                        render_pass.set_bind_group(
                            2,
                            &bitree_node.tree.vertex_layout_uniform_bind_group,
                            &[],
                        );
                        render_pass.set_bind_group(3, &bitree_node.tree.texture_bind_group, &[]);
                        render_pass.set_push_constants(
                            wgpu::ShaderStages::VERTEX,
                            0,
                            bytemuck::cast_slice(&[draw.transform]),
                        );
                        render_pass.set_index_buffer(
                            bitree_node.tree.index_buffer.slice(..),
                            bitree_node.tree.index_format,
                        );
                        render_pass.draw_indexed(
                            bitree_node.start_index
                                ..bitree_node.start_index + bitree_node.index_count,
                            0,
                            0..1,
                        );
                    }
                    Renderable::VisualEffect(vfx) => {
                        render_pass.set_pipeline(&self.particles_pipeline.pipeline);
                        render_pass.set_bind_group(0, &self.camera_uniform_bind_group, &[]);
                        render_pass.set_bind_group(
                            1,
                            &self.particles_pipeline.textures_bind_group,
                            &[],
                        );
                        render_pass.set_vertex_buffer(0, vfx.instance_buffer.slice(..));
                        render_pass.set_push_constants(
                            wgpu::ShaderStages::VERTEX,
                            0,
                            bytemuck::cast_slice(&[draw.transform]),
                        );
                        render_pass.draw(0..4, 0..vfx.instance_count);
                    }
                }
            }

            // dbg!(pre_cull_draw_count, draw_count);
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

pub enum Renderable {
    Model(scene::ModelNode),
    BiTreeNode(scene::BiTreeNode),
    VisualEffect(VisualEffectNodeRenderable),
}

impl Renderable {
    pub fn needs_alpha_blending(&self) -> bool {
        match self {
            Renderable::Model(_) => false,
            Renderable::BiTreeNode(_) => false,
            Renderable::VisualEffect(_) => true,
        }
    }
}

pub enum RenderableBounds {
    Box(BoundingBox),
    Sphere(BoundingSphere),
}

pub struct DrawCommand {
    pub renderable: Renderable,
    pub bounds: Option<RenderableBounds>,
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
