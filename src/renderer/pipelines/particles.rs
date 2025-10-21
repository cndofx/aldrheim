use std::{path::Path, rc::Rc};

use glam::{Mat4, Vec3};

use crate::{
    asset_manager::{AssetManager, TextureAsset},
    renderer::RenderContext,
};

pub struct ParticlesPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub texture_a: Rc<TextureAsset>,
    pub texture_b: Rc<TextureAsset>,
    pub texture_c: Rc<TextureAsset>,
    pub texture_d: Rc<TextureAsset>,
    pub textures_bind_group: wgpu::BindGroup,
}

impl ParticlesPipeline {
    pub fn new(
        context: &RenderContext,
        camera_uniform_bind_group_layout: &wgpu::BindGroupLayout,
        asset_manager: &mut AssetManager,
    ) -> anyhow::Result<Self> {
        let texture_a = asset_manager.load_texture(
            Path::new("Content/EffectTextures/ParticlesA.xnb"),
            None,
            context,
        )?;

        let texture_b = asset_manager.load_texture(
            Path::new("Content/EffectTextures/ParticlesB.xnb"),
            None,
            context,
        )?;

        let texture_c = asset_manager.load_texture(
            Path::new("Content/EffectTextures/ParticlesC.xnb"),
            None,
            context,
        )?;

        let texture_d = asset_manager.load_texture(
            Path::new("Content/EffectTextures/ParticlesD.xnb"),
            None,
            context,
        )?;

        let sampler = context.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Particle Textures Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let textures_bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Particle Textures Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D3,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D3,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D3,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D3,
                                multisampled: false,
                            },
                            count: None,
                        },
                    ],
                });

        let textures_bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Particle Textures Bind Group"),
                layout: &textures_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&texture_a.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&texture_b.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&texture_c.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&texture_d.view),
                    },
                ],
            });

        let shader = context
            .device
            .create_shader_module(wgpu::include_wgsl!("../../shaders/particles.wgsl"));

        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[
                        &camera_uniform_bind_group_layout,
                        &textures_bind_group_layout,
                    ],
                    push_constant_ranges: &[wgpu::PushConstantRange {
                        stages: wgpu::ShaderStages::VERTEX,
                        range: 0..(size_of::<Mat4>() as u32), // mvp matrix
                    }],
                });

        let pipeline = context
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[ParticleInstance::layout()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: context.surface_config.format,
                        // TODO: not all particles use additive blending?
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent::OVER,
                        }),
                        write_mask: wgpu::ColorWrites::all(),
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        Ok(ParticlesPipeline {
            pipeline,
            texture_a,
            texture_b,
            texture_c,
            texture_d,
            textures_bind_group,
        })
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct ParticleInstance {
    pub position: Vec3,
    /// starts at 0.0, approaches 1.0 towards the end of the particles life
    pub lifetime: f32,
    pub size: f32,
    pub rotation: f32,
    pub sprite: u32,
}

impl ParticleInstance {
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32,
        2 => Float32,
        3 => Float32,
        4 => Uint32,
    ];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ParticleInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ParticleInstance::ATTRIBUTES,
        }
    }
}
