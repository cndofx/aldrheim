use crate::{read_ext::MyReadBytesExt, xnb::TypeReader};
use std::io::Read;

use texture_2d::Texture2D;
use texture_3d::Texture3D;

pub mod texture_2d;
pub mod texture_3d;

const TEXTURE_2D_READER_NAME: &str = "Microsoft.Xna.Framework.Content.Texture2DReader";
const TEXTURE_3D_READER_NAME: &str = "Microsoft.Xna.Framework.Content.Texture3DReader";

pub enum XnbAsset {
    Null,
    Texture2D(Texture2D),
    Texture3D(Texture3D),
}

impl XnbAsset {
    pub fn read(reader: &mut impl Read, type_readers: &[TypeReader]) -> anyhow::Result<Self> {
        let type_id = reader.read_7bit_encoded_i32()? as usize;
        if type_id == 0 {
            return Ok(XnbAsset::Null);
        }
        let type_reader = &type_readers[type_id - 1];

        let name = type_reader.name.split(',').next().unwrap();
        match name {
            TEXTURE_2D_READER_NAME => {
                let texture = Texture2D::read(reader)?;
                Ok(XnbAsset::Texture2D(texture))
            }
            TEXTURE_3D_READER_NAME => {
                let texture = Texture3D::read(reader)?;
                Ok(XnbAsset::Texture3D(texture))
            }
            _ => {
                anyhow::bail!("unknown type reader: {}", type_reader.name);
            }
        }
    }
}
