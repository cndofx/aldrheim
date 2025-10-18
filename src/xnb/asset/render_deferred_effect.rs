use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};

use crate::{read_ext::MyReadBytesExt, xnb::asset::color::Color};

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
