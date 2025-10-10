use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};

use crate::xnb::asset::texture_2d::PixelFormat;

#[derive(Debug)]
pub struct Texture3D {
    pub format: PixelFormat,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub mips: Vec<Vec<u8>>,
}

impl Texture3D {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let format = reader.read_u32::<LittleEndian>()?;
        let format = PixelFormat::from_repr(format)
            .ok_or_else(|| anyhow::anyhow!("unknown texture format: {}", format))?;
        let width = reader.read_u32::<LittleEndian>()?;
        let height = reader.read_u32::<LittleEndian>()?;
        let depth = reader.read_u32::<LittleEndian>()?;
        let mip_count = reader.read_u32::<LittleEndian>()?;
        let mut mips = Vec::with_capacity(mip_count as usize);
        for _ in 0..mip_count {
            let size = reader.read_u32::<LittleEndian>()?;
            let mut mip = vec![0u8; size as usize];
            reader.read_exact(&mut mip)?;
            mips.push(mip);
        }
        Ok(Texture3D {
            format,
            width,
            height,
            depth,
            mips,
        })
    }
}
