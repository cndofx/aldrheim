use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Debug)]
pub struct VertexBuffer {
    pub data: Vec<u8>,
}

impl VertexBuffer {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let size = reader.read_u32::<LittleEndian>()? as usize;
        let mut data = vec![0; size];
        reader.read_exact(&mut data)?;
        Ok(VertexBuffer { data })
    }
}
