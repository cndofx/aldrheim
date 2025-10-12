use std::io::Read;

use crate::{read_ext::MyReadBytesExt, xnb::asset::color::Color};

#[derive(Debug)]
pub struct AdditiveEffect {
    pub color_tint: Color,
    pub vertex_color_enabled: bool,
    pub texture_enabled: bool,
    pub texture: String,
}

impl AdditiveEffect {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let color_tint = Color::read(reader)?;
        let vertex_color_enabled = reader.read_bool()?;
        let texture_enabled = reader.read_bool()?;
        let texture = reader.read_7bit_length_string()?;
        Ok(AdditiveEffect {
            color_tint,
            vertex_color_enabled,
            texture_enabled,
            texture,
        })
    }
}
