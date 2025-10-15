use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};

use crate::{
    read_ext::MyReadBytesExt,
    xnb::asset::{
        color::Color,
        vertex_decl::{ElementUsage, VertexDeclaration},
    },
};

#[derive(Debug)]
pub struct RenderDeferredEffect {
    pub alpha: f32,
    pub sharpness: f32,
    pub vertex_color_enabled: bool,
    pub use_material_texture_for_reflectiveness: bool,
    pub reflection_map: String,
    pub material_0: RenderDeferredEffectMaterial,
    pub material_1: Option<RenderDeferredEffectMaterial>,
}

impl RenderDeferredEffect {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let alpha = reader.read_f32::<LittleEndian>()?;
        let sharpness = reader.read_f32::<LittleEndian>()?;
        let vertex_color_enabled = reader.read_bool()?;
        let use_material_texture_for_reflectiveness = reader.read_bool()?;
        let reflection_map = reader.read_7bit_length_string()?;
        let material_0 = RenderDeferredEffectMaterial::read(reader)?;
        let has_material_1 = reader.read_bool()?;
        let material_1 = if has_material_1 {
            Some(RenderDeferredEffectMaterial::read(reader)?)
        } else {
            None
        };
        Ok(RenderDeferredEffect {
            alpha,
            sharpness,
            vertex_color_enabled,
            use_material_texture_for_reflectiveness,
            reflection_map,
            material_0,
            material_1,
        })
    }
}

#[derive(Debug)]
pub struct RenderDeferredEffectMaterial {
    pub diffuse_texture_alpha_disabled: bool,
    pub alpha_mask_enabled: bool,
    pub diffuse_color: Color,
    pub spec_amount: f32,
    pub spec_power: f32,
    pub emissive_amount: f32,
    pub normal_power: f32,
    pub reflectiveness: f32,
    pub diffuse_texture: String,
    pub material_texture: String,
    pub normal_texture: String,
}

impl RenderDeferredEffectMaterial {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let diffuse_texture_alpha_disabled = reader.read_bool()?;
        let alpha_mask_enabled = reader.read_bool()?;
        let diffuse_color = Color::read(reader)?;
        let spec_amount = reader.read_f32::<LittleEndian>()?;
        let spec_power = reader.read_f32::<LittleEndian>()?;
        let emissive_amount = reader.read_f32::<LittleEndian>()?;
        let normal_power = reader.read_f32::<LittleEndian>()?;
        let reflectiveness = reader.read_f32::<LittleEndian>()?;
        let diffuse_texture = reader.read_7bit_length_string()?;
        let material_texture = reader.read_7bit_length_string()?;
        let normal_texture = reader.read_7bit_length_string()?;
        Ok(RenderDeferredEffectMaterial {
            diffuse_texture_alpha_disabled,
            alpha_mask_enabled,
            diffuse_color,
            spec_amount,
            spec_power,
            emissive_amount,
            normal_power,
            reflectiveness,
            diffuse_texture,
            material_texture,
            normal_texture,
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

        println!("\n\n");
        dbg!(effect, decl);

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

        if effect.material_0.diffuse_texture.contains("stoneedge01_0")
            && m1_diffuse_texture_alpha_enabled == 0
        {
            println!("break");
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
