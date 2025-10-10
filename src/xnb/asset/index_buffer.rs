use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};

use crate::read_ext::MyReadBytesExt;

#[derive(Debug)]
pub struct IndexBuffer {
    pub is_16_bit: bool,
    pub data: Vec<u8>,
}

impl IndexBuffer {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let is_16_bit = reader.read_bool()?;
        let size = reader.read_u32::<LittleEndian>()? as usize;
        let mut data = vec![0; size];
        reader.read_exact(&mut data)?;
        Ok(IndexBuffer { is_16_bit, data })
    }
}
