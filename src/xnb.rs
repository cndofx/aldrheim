use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use lzxd::Lzxd;
use std::{
    borrow::Cow,
    io::{Cursor, Read, Seek},
};

use crate::{read_ext::MyReadBytesExt, xnb::asset::XnbAsset};

pub mod asset;

pub use asset::bi_tree_model::{BiTree, BiTreeNode};
pub use asset::model::Model;
pub use asset::render_deferred_effect::RenderDeferredEffect;
pub use asset::texture_2d::Texture2D;
pub use asset::texture_3d::Texture3D;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    WindowsPhone,
    Xbox360,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    Xna31,
    Xna40,
}

#[derive(Debug)]
pub struct Header {
    pub platform: Platform,
    pub version: Version,
    pub hi_def: bool,
    pub compressed: bool,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
}

pub struct TypeReader {
    pub name: String,
    pub version: i32,
}

pub struct XnbContent {
    pub type_readers: Vec<TypeReader>,
    pub primary_asset: XnbAsset,
    pub shared_assets: Vec<XnbAsset>,
}

pub struct Xnb {
    pub header: Header,
    pub data: Vec<u8>,
}

impl Xnb {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let mut magic = [0u8; 3];
        reader.read_exact(&mut magic)?;
        if &magic != b"XNB" {
            anyhow::bail!("not an XNB file");
        }

        let platform = match reader.read_u8()? {
            b'w' => Platform::Windows,
            b'm' => Platform::WindowsPhone,
            b'x' => Platform::Xbox360,
            v => anyhow::bail!("unknown platform: {v}"),
        };

        let version = match reader.read_u8()? {
            4 => Version::Xna31,
            5 => Version::Xna40,
            v => anyhow::bail!("unknown version: {v}"),
        };
        if version != Version::Xna31 {
            anyhow::bail!("unsupported XNA version: {version:?}, only 3.1 is supported");
        }

        let flags = reader.read_u8()?;
        let hi_def = flags & 0x01 != 0;
        let compressed = flags & 0x80 != 0;

        let compressed_size = reader.read_u32::<LittleEndian>()?;
        let uncompressed_size = if compressed {
            reader.read_u32::<LittleEndian>()?
        } else {
            0
        };

        let header_size = if compressed { 14 } else { 10 };
        let data_size = compressed_size - header_size;
        let mut data = Vec::with_capacity(data_size as usize);
        reader.take(data_size as u64).read_to_end(&mut data)?;

        let xnb = Xnb {
            header: Header {
                platform,
                version,
                hi_def,
                compressed,
                compressed_size,
                uncompressed_size,
            },
            data,
        };
        Ok(xnb)
    }

    pub fn decompress(&self) -> anyhow::Result<Cow<'_, [u8]>> {
        if !self.header.compressed {
            return Ok(Cow::from(&self.data));
        }

        let mut reader = Cursor::new(self.data.as_slice());

        let mut lzxd = Lzxd::new(lzxd::WindowSize::KB64);

        let mut block: Vec<u8> = Vec::new();
        let mut decompressed: Vec<u8> = Vec::with_capacity(self.header.uncompressed_size as usize);

        while (reader.position() as usize) < self.data.len() {
            let frame_size;
            let block_size;
            if reader.read_u8()? == 0xFF {
                frame_size = reader.read_u16::<BigEndian>()?;
                block_size = reader.read_u16::<BigEndian>()?;
            } else {
                reader.seek_relative(-1)?;
                block_size = reader.read_u16::<BigEndian>()?;
                frame_size = 0x8000;
            }

            if block_size == 0 || frame_size == 0 {
                break;
            }

            block.resize(block_size as usize, 0);
            reader.read_exact(&mut block)?;

            let frame = lzxd.decompress_next(&block, frame_size as usize)?;
            decompressed.extend_from_slice(frame);
        }

        Ok(Cow::from(decompressed))
    }

    pub fn parse_content(&self) -> anyhow::Result<XnbContent> {
        let decompressed = self.decompress()?;
        let content = Xnb::parse_content_from(&decompressed)?;
        Ok(content)
    }

    pub fn parse_content_from(decompressed: &[u8]) -> anyhow::Result<XnbContent> {
        let mut reader = Cursor::new(decompressed);

        let type_reader_count = reader.read_7bit_encoded_i32()? as usize;
        let mut type_readers = Vec::with_capacity(type_reader_count);
        for _ in 0..type_reader_count {
            let name = reader.read_7bit_length_string()?;
            let version = reader.read_i32::<LittleEndian>()?;
            let type_reader = TypeReader { name, version };
            type_readers.push(type_reader);
        }

        let shared_asset_count = reader.read_7bit_encoded_i32()?;

        let primary_asset = XnbAsset::read(&mut reader, &type_readers)?;

        let mut shared_assets = Vec::with_capacity(shared_asset_count as usize);
        for _ in 0..shared_asset_count {
            let asset = XnbAsset::read(&mut reader, &type_readers)?;
            shared_assets.push(asset);
        }

        let content = XnbContent {
            type_readers,
            primary_asset,
            shared_assets,
        };
        Ok(content)
    }
}
