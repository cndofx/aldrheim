use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};
use glam::Vec2;

use crate::{read_ext::MyReadBytesExt, xnb::asset::color::Color};

#[derive(Debug)]
pub struct RenderDeferredLiquidEffect {
    pub reflection_map: String,
    pub wave_height: f32,
    pub wave_speed_0: Vec2,
    pub wave_speed_1: Vec2,
    pub water_reflectiveness: f32,
    pub bottom_color: Color,
    pub deep_bottom_color: Color,
    pub water_emissive_amount: f32,
    pub water_spec_amount: f32,
    pub water_spec_power: f32,
    pub bottom_texture: String,
    pub water_normal_map: String,
    pub ice_reflectiveness: f32,
    pub ice_color: Color,
    pub ice_emissive_amount: f32,
    pub ice_spec_amount: f32,
    pub ice_spec_power: f32,
    pub ice_diffuse_map: String,
    pub ice_normal_map: String,
}

impl RenderDeferredLiquidEffect {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let reflection_map = reader.read_7bit_length_string()?;
        let wave_height = reader.read_f32::<LittleEndian>()?;
        let wave_speed_0 = reader.read_vec2()?;
        let wave_speed_1 = reader.read_vec2()?;
        let water_reflectiveness = reader.read_f32::<LittleEndian>()?;
        let bottom_color = Color::read(reader)?;
        let deep_bottom_color = Color::read(reader)?;
        let water_emissive_amount = reader.read_f32::<LittleEndian>()?;
        let water_spec_amount = reader.read_f32::<LittleEndian>()?;
        let water_spec_power = reader.read_f32::<LittleEndian>()?;
        let bottom_texture = reader.read_7bit_length_string()?;
        let water_normal_map = reader.read_7bit_length_string()?;
        let ice_reflectiveness = reader.read_f32::<LittleEndian>()?;
        let ice_color = Color::read(reader)?;
        let ice_emissive_amount = reader.read_f32::<LittleEndian>()?;
        let ice_spec_amount = reader.read_f32::<LittleEndian>()?;
        let ice_spec_power = reader.read_f32::<LittleEndian>()?;
        let ice_diffuse_map = reader.read_7bit_length_string()?;
        let ice_normal_map = reader.read_7bit_length_string()?;

        Ok(RenderDeferredLiquidEffect {
            reflection_map,
            wave_height,
            wave_speed_0,
            wave_speed_1,
            water_reflectiveness,
            bottom_color,
            deep_bottom_color,
            water_emissive_amount,
            water_spec_amount,
            water_spec_power,
            bottom_texture,
            water_normal_map,
            ice_reflectiveness,
            ice_color,
            ice_emissive_amount,
            ice_spec_amount,
            ice_spec_power,
            ice_diffuse_map,
            ice_normal_map,
        })
    }
}
