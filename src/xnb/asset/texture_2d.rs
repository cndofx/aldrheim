use std::{borrow::Cow, io::Read};

use bcndecode::{BcnDecoderFormat, BcnEncoding};
use byteorder::{LittleEndian, ReadBytesExt};
use strum::FromRepr;

pub struct Texture2D {
    pub format: u32,
    pub width: u32,
    pub height: u32,
    pub mips: Vec<Vec<u8>>,
}

impl Texture2D {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let format = reader.read_u32::<LittleEndian>()?;
        let width = reader.read_u32::<LittleEndian>()?;
        let height = reader.read_u32::<LittleEndian>()?;
        let mip_count = reader.read_u32::<LittleEndian>()?;
        let mut mips = Vec::with_capacity(mip_count as usize);
        for _ in 0..mip_count {
            let size = reader.read_u32::<LittleEndian>()?;
            let mut mip = vec![0u8; size as usize];
            reader.read_exact(&mut mip)?;
            mips.push(mip);
        }
        Ok(Texture2D {
            format,
            width,
            height,
            mips,
        })
    }

    /// returns bgra8 pixels
    pub fn decode<'a>(&'a self, mip_index: usize) -> anyhow::Result<Cow<'a, [u8]>> {
        let format = PixelFormat::from_repr(self.format)
            .ok_or_else(|| anyhow::anyhow!("unknown pixel format: {}", self.format))?;

        let pixels = decode_pixels(
            &self.mips[mip_index],
            self.width as usize,
            self.height as usize,
            format,
        )?;

        Ok(pixels)
    }
}

/// returns bgra8 pixels
pub fn decode_pixels<'a>(
    source: &'a [u8],
    width: usize,
    height: usize,
    format: PixelFormat,
) -> anyhow::Result<Cow<'a, [u8]>> {
    match format {
        PixelFormat::Color => Ok(Cow::from(source)),
        PixelFormat::Bc1 => {
            let pixels = bcndecode::decode(
                source,
                width,
                height,
                BcnEncoding::Bc1,
                BcnDecoderFormat::BGRA,
            )?;
            Ok(Cow::from(pixels))
        }
        PixelFormat::Bc3 => {
            let pixels = bcndecode::decode(
                source,
                width,
                height,
                BcnEncoding::Bc3,
                BcnDecoderFormat::BGRA,
            )?;
            Ok(Cow::from(pixels))
        }
    }
}

pub fn bgra8_to_rgba8(bgra8: &[u8]) -> Vec<u8> {
    let mut rgba8 = Vec::with_capacity(bgra8.len());

    for pixel in bgra8.chunks_exact(4) {
        let b = pixel[0];
        let g = pixel[1];
        let r = pixel[2];
        let a = pixel[3];
        rgba8.extend_from_slice(&[r, g, b, a]);
    }

    rgba8
}

#[repr(u32)]
#[derive(FromRepr, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// bgra8?
    Color = 1,
    Bc1 = 28,
    Bc3 = 32,
}
