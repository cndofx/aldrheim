use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct VertexDeclaration {
    pub elements: Vec<VertexElement>,
}

impl VertexDeclaration {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let num_elements = reader.read_u32::<LittleEndian>()? as usize;
        let mut elements = Vec::with_capacity(num_elements);
        for _ in 0..num_elements {
            let element = VertexElement::read(reader)?;
            elements.push(element);
        }
        Ok(VertexDeclaration { elements })
    }

    pub fn stride(&self) -> usize {
        self.elements
            .iter()
            .map(|el| el.offset as usize + el.format.size())
            .max()
            .unwrap_or(0)
    }

    pub fn to_wgpu(&self) -> Vec<wgpu::VertexAttribute> {
        self.elements
            .iter()
            .enumerate()
            .map(|(i, xnb)| wgpu::VertexAttribute {
                shader_location: i as u32,
                offset: xnb.offset as wgpu::BufferAddress,
                format: xnb.format.to_wgpu(),
            })
            .collect()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct VertexElement {
    pub stream: u16,
    pub offset: u16,
    pub format: ElementFormat,
    pub method: ElementMethod,
    pub usage: ElementUsage,
    pub usage_index: u8,
}

impl VertexElement {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let stream = reader.read_u16::<LittleEndian>()?;
        let offset = reader.read_u16::<LittleEndian>()?;
        let format = ElementFormat::read(reader)?;
        let method = ElementMethod::read(reader)?;
        let usage = ElementUsage::read(reader)?;
        let usage_index = reader.read_u8()?;
        Ok(VertexElement {
            stream,
            offset,
            format,
            method,
            usage,
            usage_index,
        })
    }

    pub fn debug_string(&self) -> String {
        format!("{:?}-{:?}", self.format, self.usage)
    }
}

#[repr(u8)]
#[derive(strum::FromRepr, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementFormat {
    Single,
    Vector2,
    Vector3,
    Vector4,
    Color,
    Byte4,
    Short2,
    Short4,
    Rgba32,
    NormalizedShort2,
    NormalizedShort4,
    Rgb32,
    Rgba64,
    UInt40,
    Normalized40,
    HalfVector2,
    HalfVector4,
}

impl ElementFormat {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let value = reader.read_u8()?;
        let format = ElementFormat::from_repr(value)
            .ok_or_else(|| anyhow::anyhow!("unknown element format: {value}"))?;
        Ok(format)
    }

    pub fn size(self) -> usize {
        match self {
            ElementFormat::Single => 4,
            ElementFormat::Vector2 => 8,
            ElementFormat::Vector3 => 12,
            ElementFormat::Vector4 => 16,
            ElementFormat::Byte4 => 4,
            other => unimplemented!("element format size: {other:?}"),
        }
    }

    pub fn to_wgpu(self) -> wgpu::VertexFormat {
        match self {
            ElementFormat::Single => wgpu::VertexFormat::Float32,
            ElementFormat::Vector2 => wgpu::VertexFormat::Float32x2,
            ElementFormat::Vector3 => wgpu::VertexFormat::Float32x3,
            ElementFormat::Vector4 => wgpu::VertexFormat::Float32x4,
            ElementFormat::Byte4 => wgpu::VertexFormat::Uint8x4,
            _ => unimplemented!("unsupported vertex element format: {self:?}"),
        }
    }
}

#[repr(u8)]
#[derive(strum::FromRepr, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementMethod {
    Default,
    UV = 4,
    LookUp = 5,
    LookUpPresampled,
}

impl ElementMethod {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let value = reader.read_u8()?;
        let method = ElementMethod::from_repr(value)
            .ok_or_else(|| anyhow::anyhow!("unknown element method: {value}"))?;
        Ok(method)
    }
}

#[repr(u8)]
#[derive(strum::FromRepr, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementUsage {
    Position,
    BlendWeight,
    BlendIndices,
    Normal,
    PointSize,
    TextureCoordinate,
    Tangent,
    Binormal,
    TessellateFactor,
    Color = 10,
    Fog,
    Depth,
    Sample,
}

impl ElementUsage {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let value = reader.read_u8()?;
        let usage = ElementUsage::from_repr(value)
            .ok_or_else(|| anyhow::anyhow!("unknown element usage: {value}"))?;
        Ok(usage)
    }
}
