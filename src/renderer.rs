use std::{path::Path, rc::Rc, sync::Arc};

use glam::Mat4;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::{
    asset_manager::AssetManager,
    scene::Camera,
    xnb::{
        self,
        asset::{
            XnbAsset,
            vertex_decl::{ElementUsage, VertexDeclaration},
        },
    },
};

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    device: wgpu::Device,
    queue: wgpu::Queue,
    depth_texture: wgpu::Texture,
    pub window: Arc<Window>,

    render_deferred_effect_pipeline: RenderDeferredEffectPipeline,
    linear_sampler: wgpu::Sampler,

    // using `Rc` instead of `Weak` so that resources arent immediately dropped
    // when no longer used. if all the "goblin" enemies died, the goblin mesh
    // would disappear, even though the game is likely to need the goblin mesh
    // again. i'm thinking all meshes should be loaded during a loading screen,
    // and all unneeded meshes are dropped during that same loading screen
    meshes: Vec<Rc<Mesh>>,
    textures: Vec<Rc<wgpu::Texture>>, // i think wgpu resources are already refcounted? but i cant query the reference count
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
        dbg!(&surface_caps);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        dbg!(surface_format);
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

        let depth_texture = create_depth_texture(&device, &surface_config);

        let render_deferred_effect_pipeline =
            RenderDeferredEffectPipeline::new(&device, &surface_config)?;

        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let renderer = Renderer {
            surface,
            surface_config,
            is_surface_configured: false,
            device,
            queue,
            window,
            depth_texture,
            render_deferred_effect_pipeline,
            linear_sampler,
            meshes: Vec::new(),
            textures: Vec::new(),
        };
        Ok(renderer)
    }

    pub fn render(
        &mut self,
        draw_commands: &[MeshDrawCommand],
        camera: &Camera,
    ) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        let window_size = self.window.inner_size();
        let projection = Mat4::perspective_lh(
            camera.fov_y_radians,
            (window_size.width as f32) / (window_size.height as f32),
            camera.z_near,
            camera.z_far,
        );

        let view = camera.view_matrix();

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

            for draw in draw_commands {
                // TODO: this assumes that all pipelines use the same bind groups and does no sorting or batching

                render_pass.set_pipeline(&draw.mesh.pipeline);
                render_pass.set_bind_group(0, &draw.mesh.vertex_buffer_bind_group, &[]);
                render_pass.set_bind_group(1, &draw.mesh.vertex_layout_uniform_bind_group, &[]);
                render_pass.set_bind_group(2, &draw.mesh.texture_bind_group, &[]);
                render_pass
                    .set_index_buffer(draw.mesh.index_buffer.slice(..), draw.mesh.index_format);

                let mvp = projection * view * draw.transform;
                render_pass.set_push_constants(
                    wgpu::ShaderStages::VERTEX,
                    0,
                    bytemuck::cast_slice(&[mvp]),
                );

                render_pass.draw_indexed(
                    draw.mesh.start_index..draw.mesh.start_index + draw.mesh.index_count,
                    draw.mesh.base_vertex as i32,
                    0..1,
                );
            }
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

    pub fn load_model_from_path(
        &mut self,
        path: impl AsRef<Path>,
        asset_manager: &AssetManager,
    ) -> anyhow::Result<Rc<Mesh>> {
        let path = path.as_ref();

        let model_xnb = asset_manager.load_xnb(path)?;
        let model_content = model_xnb.parse_content()?;
        let XnbAsset::Model(model) = &model_content.primary_asset else {
            anyhow::bail!("expected model at path {}", path.display());
        };
        let XnbAsset::RenderDeferredEffect(effect) = &model_content.shared_assets[0] else {
            anyhow::bail!(
                "expected render deferred effect in model at path {}",
                path.display()
            );
        };

        let texture = self.load_texture_2d_from_relative_path(
            path,
            &effect.material_0.diffuse_texture,
            asset_manager,
        )?;
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mesh0 = &model.meshes[0];
        let part0 = &mesh0.parts[0];
        let vertex_decl = &model.vertex_decls[part0.vertex_decl_index as usize];

        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: &mesh0.vertex_buffer.data,
                usage: wgpu::BufferUsages::STORAGE,
            });

        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: &mesh0.index_buffer.data,
                usage: wgpu::BufferUsages::INDEX,
            });

        let index_format = if mesh0.index_buffer.is_16_bit {
            wgpu::IndexFormat::Uint16
        } else {
            wgpu::IndexFormat::Uint32
        };
        let index_count = part0.primitive_count * 3;
        let start_index = part0.start_index;
        let base_vertex = part0.base_vertex;

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

        let vertex_layout_uniform = VertexLayoutUniform::from_xnb_decl(vertex_decl)?;
        let vertex_layout_uniform_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Layout Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[vertex_layout_uniform]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        let vertex_layout_uniform_bind_group =
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Vertex Layout Uniform Bind Group"),
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

        let texture_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &self
                .render_deferred_effect_pipeline
                .texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                },
            ],
        });

        let mesh = Mesh {
            pipeline: self.render_deferred_effect_pipeline.pipeline.clone(),
            vertex_buffer,
            vertex_buffer_bind_group,
            vertex_layout_uniform_buffer,
            vertex_layout_uniform_bind_group,
            index_buffer,
            index_format,
            index_count,
            start_index,
            base_vertex,
            texture_bind_group,
        };

        Ok(Rc::new(mesh))
    }

    pub fn load_texture_2d(
        &mut self,
        texture: &xnb::asset::texture_2d::Texture2D,
    ) -> anyhow::Result<Rc<wgpu::Texture>> {
        let texture_format = texture.format.to_wgpu();
        dbg!(texture_format);

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
            dbg!(i, mip_size);

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

        Ok(Rc::new(wgpu_texture))
    }

    pub fn load_texture_2d_from_relative_path(
        &mut self,
        base: impl AsRef<Path>,
        relative: impl AsRef<Path>,
        asset_manager: &AssetManager,
    ) -> anyhow::Result<Rc<wgpu::Texture>> {
        let base = base.as_ref();
        let relative = relative.as_ref();

        let texture_xnb = asset_manager.load_xnb_relative(base, relative)?;
        let texture_content = texture_xnb.parse_content()?;
        let XnbAsset::Texture2D(texture) = &texture_content.primary_asset else {
            anyhow::bail!(
                "expected texture at relative path {} (base {})",
                relative.display(),
                base.display(),
            );
        };

        let texture = self.load_texture_2d(texture)?;

        Ok(texture)
    }
}

pub struct RenderDeferredEffectPipeline {
    vertex_buffer_bind_group_layout: wgpu::BindGroupLayout,
    vertex_layout_uniform_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
}

impl RenderDeferredEffectPipeline {
    pub fn new(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> anyhow::Result<Self> {
        let vertex_buffer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Render Deferred Effect Vertex Buffer Bind Group Layout"),
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

        let vertex_layout_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Render Deferred Effect Vertex Layout Uniform Bind Group Layout"),
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

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Render Deferred Effect Texture Bind Group Layout"),
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

        let shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/render_deferred_effect.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                &vertex_buffer_bind_group_layout,
                &vertex_layout_uniform_bind_group_layout,
                &texture_bind_group_layout,
            ],
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::VERTEX,
                range: 0..(size_of::<Mat4>() as u32), // mvp matrix
            }],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::all(),
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Ok(RenderDeferredEffectPipeline {
            vertex_buffer_bind_group_layout,
            vertex_layout_uniform_bind_group_layout,
            texture_bind_group_layout,
            pipeline,
        })
    }
}

pub struct Mesh {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_buffer_bind_group: wgpu::BindGroup,
    vertex_layout_uniform_buffer: wgpu::Buffer,
    vertex_layout_uniform_bind_group: wgpu::BindGroup,
    index_buffer: wgpu::Buffer,
    index_format: wgpu::IndexFormat,
    index_count: u32,
    start_index: u32,
    base_vertex: u32,
    texture_bind_group: wgpu::BindGroup,
}

pub struct MeshDrawCommand {
    pub mesh: Rc<Mesh>,
    pub transform: Mat4,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Debug, Clone, Copy)]
struct VertexLayoutUniform {
    stride: u32,
    position: i32,
    normal: i32,
    color: i32,
    tex_coord_0: i32,
    tex_coord_1: i32,
}

impl VertexLayoutUniform {
    fn from_xnb_decl(decl: &VertexDeclaration) -> anyhow::Result<Self> {
        // TODO: this is terrible

        let mut position = -1;
        let mut normal = -1;
        let mut color = -1;
        let mut tex_coord_0 = -1;
        let mut tex_coord_1 = -1;
        for el in &decl.elements {
            match el.usage {
                ElementUsage::Position => {
                    if position < 0 {
                        position = el.offset as i32;
                    } else {
                        anyhow::bail!("duplicate 'position' elements in vertex declaration");
                    }
                }
                ElementUsage::Normal => {
                    if normal < 0 {
                        normal = el.offset as i32;
                    } else {
                        anyhow::bail!("duplicate 'normal' elements in vertex declaration");
                    }
                }
                ElementUsage::TextureCoordinate => {
                    if tex_coord_0 < 0 {
                        tex_coord_0 = el.offset as i32;
                    } else if tex_coord_1 < 0 {
                        tex_coord_1 = el.offset as i32;
                    }
                    // else {
                    //     anyhow::bail!("duplicate 'tex_coord' elements in vertex declaration");
                    // }
                }
                ElementUsage::Color => {
                    if color < 0 {
                        color = el.offset as i32;
                    } else {
                        anyhow::bail!("duplicate 'color' elements in vertex declaration");
                    }
                }
                _ => anyhow::bail!("unsupported vertex usage '{:?}'", el.usage),
            }
        }

        if position == -1 {
            anyhow::bail!("missing vertex element 'position'");
        }

        if normal == -1 {
            anyhow::bail!("missing vertex element 'normal'");
        }

        if tex_coord_0 == -1 {
            anyhow::bail!("missing vertex element 'tex_coord'");
        }

        Ok(VertexLayoutUniform {
            stride: decl.stride() as u32,
            position,
            normal,
            color,
            tex_coord_0,
            tex_coord_1,
        })
    }
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
