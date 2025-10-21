use glam::Mat4;

use crate::{
    renderer::RenderContext,
    xnb::asset::{
        render_deferred_effect::RenderDeferredEffect,
        vertex_decl::{ElementUsage, VertexDeclaration},
    },
};

pub struct RenderDeferredEffectPipeline {
    pub vertex_buffer_bind_group_layout: wgpu::BindGroupLayout,
    pub vertex_layout_uniform_bind_group_layout: wgpu::BindGroupLayout,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub pipeline: wgpu::RenderPipeline,
}

impl RenderDeferredEffectPipeline {
    pub fn new(
        context: &RenderContext,
        camera_uniform_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> anyhow::Result<Self> {
        let vertex_buffer_bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let effect_properties_uniform_bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Render Deferred Effect Properties Uniform Bind Group Layout"),
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

        let texture_bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let shader = context.device.create_shader_module(wgpu::include_wgsl!(
            "../../shaders/render_deferred_effect.wgsl"
        ));

        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[
                        &camera_uniform_bind_group_layout,
                        &vertex_buffer_bind_group_layout,
                        &effect_properties_uniform_bind_group_layout,
                        &texture_bind_group_layout,
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
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: context.surface_config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::all(),
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Cw,
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
            vertex_layout_uniform_bind_group_layout: effect_properties_uniform_bind_group_layout,
            texture_bind_group_layout,
            pipeline,
        })
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Debug, Clone, Copy)]
pub struct RenderDeferredEffectUniform {
    pub vertex_layout_stride: u32,
    pub vertex_layout_position: i32,
    pub vertex_layout_normal: i32,
    pub vertex_layout_tangent_0: i32,
    pub vertex_layout_tangent_1: i32,
    pub vertex_layout_color: i32,
    pub vertex_layout_tex_coords_0: i32,
    pub vertex_layout_tex_coords_1: i32,

    pub sharpness: f32,
    pub vertex_color_enabled: i32,

    pub m0_diffuse_color_r: f32,
    pub m0_diffuse_color_g: f32,
    pub m0_diffuse_color_b: f32,
    pub m0_diffuse_texture_enabled: i32,
    pub m0_diffuse_texture_alpha_enabled: i32,
    pub m0_alpha_mask_enabled: i32,
    pub m1_enabled: i32,
    pub m1_diffuse_color_r: f32,
    pub m1_diffuse_color_g: f32,
    pub m1_diffuse_color_b: f32,
    pub m1_diffuse_texture_enabled: i32,
    pub m1_diffuse_texture_alpha_enabled: i32,
    pub m1_alpha_mask_enabled: i32, // always opposite of m0_alpha_mask_enabled?
}

impl RenderDeferredEffectUniform {
    pub fn new(effect: &RenderDeferredEffect, decl: &VertexDeclaration) -> anyhow::Result<Self> {
        let layout = RenderDeferredEffectVertexLayout::new(decl)?;

        let vertex_layout_stride = layout.stride;
        let vertex_layout_position = layout.position;
        let vertex_layout_normal = layout.normal;
        let vertex_layout_tangent_0 = layout.tangent_0;
        let vertex_layout_tangent_1 = layout.tangent_1;
        let vertex_layout_color = layout.color;
        let vertex_layout_tex_coords_0 = layout.tex_coords_0;
        let vertex_layout_tex_coords_1 = layout.tex_coords_1;

        // println!("\n\n");
        // dbg!(effect, decl);

        let sharpness = effect.sharpness;
        let vertex_color_enabled = if effect.vertex_color_enabled { 1 } else { 0 };

        let m0_diffuse_color_r = effect.material_0.diffuse_color.r;
        let m0_diffuse_color_g = effect.material_0.diffuse_color.g;
        let m0_diffuse_color_b = effect.material_0.diffuse_color.b;
        let m0_diffuse_texture_alpha_enabled = if effect.material_0.diffuse_texture_alpha_disabled {
            0
        } else {
            1
        };
        let m0_alpha_mask_enabled = if effect.material_0.alpha_mask_enabled {
            1
        } else {
            0
        };

        let mut m1_enabled = 0;
        let mut m1_diffuse_color_r = 0.0;
        let mut m1_diffuse_color_g = 0.0;
        let mut m1_diffuse_color_b = 0.0;
        let mut m1_diffuse_texture_alpha_enabled = 0;
        let mut m1_alpha_mask_enabled = 0;

        if let Some(material_1) = &effect.material_1 {
            m1_enabled = 1;
            m1_diffuse_color_r = material_1.diffuse_color.r;
            m1_diffuse_color_g = material_1.diffuse_color.g;
            m1_diffuse_color_b = material_1.diffuse_color.b;
            m1_diffuse_texture_alpha_enabled = if material_1.diffuse_texture_alpha_disabled {
                0
            } else {
                1
            };
            m1_alpha_mask_enabled = if material_1.alpha_mask_enabled { 1 } else { 0 };
        }

        Ok(RenderDeferredEffectUniform {
            vertex_layout_stride,
            vertex_layout_position,
            vertex_layout_normal,
            vertex_layout_tangent_0,
            vertex_layout_tangent_1,
            vertex_layout_color,
            vertex_layout_tex_coords_0,
            vertex_layout_tex_coords_1,

            sharpness,
            vertex_color_enabled,

            m0_diffuse_color_r,
            m0_diffuse_color_g,
            m0_diffuse_color_b,
            m0_diffuse_texture_enabled: 1, // TODO
            m0_diffuse_texture_alpha_enabled,
            m0_alpha_mask_enabled,
            m1_enabled,
            m1_diffuse_color_r,
            m1_diffuse_color_g,
            m1_diffuse_color_b,
            m1_diffuse_texture_enabled: 1, // TODO
            m1_diffuse_texture_alpha_enabled,
            m1_alpha_mask_enabled,
        })
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Debug, Clone, Copy)]
pub struct RenderDeferredEffectVertexLayout {
    pub stride: u32,
    pub position: i32,
    pub normal: i32,
    pub tangent_0: i32,
    pub tangent_1: i32,
    pub color: i32,
    pub tex_coords_0: i32,
    pub tex_coords_1: i32,
}

impl RenderDeferredEffectVertexLayout {
    pub fn new(decl: &VertexDeclaration) -> anyhow::Result<Self> {
        let mut position = -1;
        let mut normal = -1;
        let mut tangent_0 = -1;
        let mut tangent_1 = -1;
        let mut color = -1;
        let mut tex_coords_0 = -1;
        let mut tex_coords_1 = -1;

        let mut ignored_positions = 0;
        let mut ignored_normals = 0;
        let mut ignored_tangents = 0;
        let mut ignored_colors = 0;
        let mut ignored_tex_coords = 0;

        for el in &decl.elements {
            let offset = el.offset as i32;
            match el.usage {
                ElementUsage::Position => {
                    if position < 0 {
                        position = offset;
                    } else {
                        ignored_positions += 1;
                    }
                }
                ElementUsage::Normal => {
                    if normal < 0 {
                        normal = offset;
                    } else {
                        ignored_normals += 1;
                    }
                }
                ElementUsage::Tangent => {
                    if tangent_0 < 0 {
                        tangent_0 = offset;
                    } else if tangent_1 < 0 {
                        tangent_1 = offset;
                    } else {
                        ignored_tangents += 1;
                    }
                }
                ElementUsage::Color => {
                    if color < 0 {
                        color = offset;
                    } else {
                        ignored_colors += 1;
                    }
                }
                ElementUsage::TextureCoordinate => {
                    if tex_coords_0 < 0 {
                        tex_coords_0 = offset;
                    } else if tex_coords_1 < 0 {
                        tex_coords_1 = offset;
                    } else {
                        ignored_tex_coords += 1;
                    }
                }
                _ => anyhow::bail!("unsupported vertex usage '{:?}'", el.usage),
            }
        }

        // TODO: figure out which are actually required and implement proper fallbacks for the rest

        if position == -1 {
            anyhow::bail!("missing vertex element 'position'");
        }

        if normal == -1 {
            anyhow::bail!("missing vertex element 'normal'");
        }

        if tex_coords_0 == -1 {
            anyhow::bail!("missing vertex element 'tex_coord'");
        }

        Ok(RenderDeferredEffectVertexLayout {
            stride: decl.stride() as u32,
            position,
            normal,
            tangent_0,
            tangent_1,
            color,
            tex_coords_0,
            tex_coords_1,
        })
    }
}
