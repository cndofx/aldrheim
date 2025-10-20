use std::{rc::Rc, sync::Arc};

use glam::Mat4;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::{
    asset_manager::{BiTreeAsset, ModelAsset, Texture2DAsset},
    renderer::{
        camera::{Camera, Frustum},
        pipelines::{
            particles::ParticlesPipeline,
            render_deferred_effect::{RenderDeferredEffectPipeline, RenderDeferredEffectUniform},
        },
    },
    scene::{self, vfx::VisualEffectNodeRenderable},
    xnb::{
        self,
        asset::model::{BoundingBox, BoundingSphere},
    },
};

pub mod camera;
pub mod pipelines;

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub window: Arc<Window>,

    camera_uniform_buffer: wgpu::Buffer,
    camera_uniform_bind_group: wgpu::BindGroup,

    depth_texture: wgpu::Texture,
    placeholder_texture: wgpu::Texture,
    placeholder_texture_view: wgpu::TextureView,

    linear_sampler: wgpu::Sampler,

    particles_pipeline: ParticlesPipeline,
    render_deferred_effect_pipeline: RenderDeferredEffectPipeline,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
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
                    max_push_constant_size: 128,
                    ..wgpu::Limits::defaults()
                },
                required_features: wgpu::Features::TEXTURE_COMPRESSION_BC
                    | wgpu::Features::PUSH_CONSTANTS,
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

        let camera_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Uniform Buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let camera_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Uniform Bind Group"),
            layout: &camera_uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform_buffer.as_entire_binding(),
            }],
        });

        let particles_pipeline =
            ParticlesPipeline::new(&device, &surface_config, &camera_uniform_bind_group_layout)?;
        let render_deferred_effect_pipeline = RenderDeferredEffectPipeline::new(
            &device,
            &surface_config,
            &camera_uniform_bind_group_layout,
        )?;

        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let depth_texture = create_depth_texture(&device, &surface_config);

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

        let renderer = Renderer {
            surface,
            surface_config,
            is_surface_configured: false,
            device,
            queue,
            window,

            camera_uniform_buffer,
            camera_uniform_bind_group,

            linear_sampler,
            depth_texture,
            placeholder_texture,
            placeholder_texture_view,

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
            forward: [camera_forward.x, camera_forward.y, camera_forward.z, 1.0],
            right: [camera_right.x, camera_right.y, camera_right.z, 1.0],
            up: [camera_up.x, camera_up.y, camera_up.z, 1.0],
        };
        self.queue.write_buffer(
            &self.camera_uniform_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );

        // let pre_cull_draw_count = draw_commands.len();
        let frustum = Frustum::new(view_proj);
        let culled_draw_commands = draw_commands
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

        let surface_texture = self.surface.get_current_texture()?;
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture_view = self
            .depth_texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut command_encoder = self
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

        self.queue.submit([command_encoder.finish()]);

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
        self.surface.configure(&self.device, &self.surface_config);
        self.is_surface_configured = true;

        self.depth_texture = create_depth_texture(&self.device, &self.surface_config);
    }

    pub fn reconfigure_surface(&mut self) {
        let size = self.window.inner_size();
        self.resize(size.width, size.height);
    }

    pub fn load_bitree(
        &self,
        tree: &xnb::BiTree,
        diffuse_texture_0: Option<Rc<Texture2DAsset>>,
        diffuse_texture_1: Option<Rc<Texture2DAsset>>,
        effect_uniform: RenderDeferredEffectUniform,
    ) -> anyhow::Result<BiTreeAsset> {
        let index_format = tree.index_buffer.wgpu_format();

        let vertex_layout_uniform_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Effect Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[effect_uniform]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        let vertex_layout_uniform_bind_group =
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Effect Uniform Bind Group"),
                layout: &self
                    .render_deferred_effect_pipeline
                    .vertex_layout_uniform_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(
                        vertex_layout_uniform_buffer.as_entire_buffer_binding(),
                    ),
                }],
            });

        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: &tree.vertex_buffer.data,
                usage: wgpu::BufferUsages::STORAGE,
            });

        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: &tree.index_buffer.data,
                usage: wgpu::BufferUsages::INDEX,
            });

        let vertex_buffer_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Vertex Buffer Bind Group"),
            layout: &self
                .render_deferred_effect_pipeline
                .vertex_buffer_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(vertex_buffer.as_entire_buffer_binding()),
            }],
        });

        let texture_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &self
                .render_deferred_effect_pipeline
                .texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        if let Some(diffuse_0) = &diffuse_texture_0 {
                            &diffuse_0.view
                        } else {
                            &self.placeholder_texture_view
                        },
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        if let Some(diffuse_1) = &diffuse_texture_1 {
                            &diffuse_1.view
                        } else {
                            &self.placeholder_texture_view
                        },
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                },
            ],
        });

        Ok(BiTreeAsset {
            visible: tree.visible,
            vertex_buffer,
            vertex_buffer_bind_group,
            vertex_layout_uniform_buffer,
            vertex_layout_uniform_bind_group,
            index_buffer,
            index_format,
            texture_bind_group,
            diffuse_texture_0,
            diffuse_texture_1,
        })
    }

    pub fn load_model(
        &self,
        model: &xnb::Model,
        texture: Rc<Texture2DAsset>,
    ) -> anyhow::Result<ModelAsset> {
        todo!()

        // let mesh0 = &model.meshes[0];
        // let part0 = &mesh0.parts[0];
        // let vertex_decl = &model.vertex_decls[part0.vertex_decl_index as usize];
        // let index_format = mesh0.index_buffer.wgpu_format();
        // let index_count = part0.primitive_count * 3;
        // let start_index = part0.start_index;
        // let base_vertex = part0.base_vertex;

        // let vertex_layout_uniform = VertexLayoutUniform::from_xnb_decl(vertex_decl)?;
        // let vertex_layout_uniform_buffer =
        //     self.device
        //         .create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //             label: Some("Vertex Layout Uniform Buffer"),
        //             contents: bytemuck::cast_slice(&[vertex_layout_uniform]),
        //             usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        //         });
        // let vertex_layout_uniform_bind_group =
        //     self.device.create_bind_group(&wgpu::BindGroupDescriptor {
        //         label: Some("Vertex Layout Uniform Bind Group"),
        //         layout: &self
        //             .render_deferred_effect_pipeline
        //             .vertex_layout_uniform_bind_group_layout,
        //         entries: &[wgpu::BindGroupEntry {
        //             binding: 0,
        //             resource: wgpu::BindingResource::Buffer(
        //                 vertex_layout_uniform_buffer.as_entire_buffer_binding(),
        //             ),
        //         }],
        //     });

        // let vertex_buffer = self
        //     .device
        //     .create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //         label: Some("Vertex Buffer"),
        //         contents: &mesh0.vertex_buffer.data,
        //         usage: wgpu::BufferUsages::STORAGE,
        //     });

        // let index_buffer = self
        //     .device
        //     .create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //         label: Some("Index Buffer"),
        //         contents: &mesh0.index_buffer.data,
        //         usage: wgpu::BufferUsages::INDEX,
        //     });

        // let vertex_buffer_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     label: Some("Vertex Buffer Bind Group"),
        //     layout: &self
        //         .render_deferred_effect_pipeline
        //         .vertex_buffer_bind_group_layout,
        //     entries: &[wgpu::BindGroupEntry {
        //         binding: 0,
        //         resource: wgpu::BindingResource::Buffer(vertex_buffer.as_entire_buffer_binding()),
        //     }],
        // });

        // Ok(ModelAsset {
        //     pipeline: self.render_deferred_effect_pipeline.pipeline.clone(),
        //     vertex_buffer,
        //     vertex_buffer_bind_group,
        //     vertex_layout_uniform_buffer,
        //     vertex_layout_uniform_bind_group,
        //     index_buffer,
        //     index_format,
        //     index_count,
        //     start_index,
        //     base_vertex,
        //     texture,
        // })
    }

    pub fn load_texture_2d(&self, texture: &xnb::Texture2D) -> anyhow::Result<Texture2DAsset> {
        let texture_format = texture.format.to_wgpu();

        let texture_size = wgpu::Extent3d {
            width: texture.width,
            height: texture.height,
            depth_or_array_layers: 1,
        };

        let wgpu_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Texture 2D"),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            size: texture_size,
            format: texture_format,
            dimension: wgpu::TextureDimension::D2,
            mip_level_count: texture.mips.len() as u32,
            sample_count: 1,
            view_formats: &[],
        });

        for (i, mip) in texture.mips.iter().enumerate() {
            // TODO: is this the correct thing to do here?
            // wgpu validation doesnt like copying 2x2 pixel mips with 4x4 block size
            let mip_size = wgpu::Extent3d {
                width: (texture.width / 2u32.pow(i as u32)).max(texture.format.block_dim()),
                height: (texture.height / 2u32.pow(i as u32)).max(texture.format.block_dim()),
                depth_or_array_layers: 1,
            };

            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &wgpu_texture,
                    mip_level: i as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                mip,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(texture.bytes_per_row(i)?),
                    rows_per_image: Some(texture.rows_per_image(i)?),
                },
                mip_size,
            );
        }

        let view = wgpu_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Ok(Texture2DAsset {
            texture: wgpu_texture,
            view,
        })
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub forward: [f32; 4],
    pub right: [f32; 4],
    pub up: [f32; 4],
}

pub enum Renderable {
    Model(scene::ModelNode),
    BiTreeNode(scene::BiTreeNode),
    VisualEffect(VisualEffectNodeRenderable),
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
