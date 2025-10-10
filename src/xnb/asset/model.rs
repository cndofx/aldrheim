use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};
use glam::{Mat4, Vec3};

use crate::{
    read_ext::MyReadBytesExt,
    xnb::{
        TypeReader,
        asset::{
            XnbAsset, index_buffer::IndexBuffer, vertex_buffer::VertexBuffer,
            vertex_decl::VertexDeclaration,
        },
    },
};

#[derive(Debug)]
pub struct Model {
    pub bones: Vec<Bone>,
    pub bones_hierarchy: Vec<BoneHierarchy>,
    pub vertex_decls: Vec<VertexDeclaration>,
    pub meshes: Vec<Mesh>,
    pub root_bone_ref: u32,
    pub tag: u8,
}

impl Model {
    pub fn read(reader: &mut impl Read, type_readers: &[TypeReader]) -> anyhow::Result<Self> {
        let num_bones = reader.read_u32::<LittleEndian>()?;

        let mut bones = Vec::with_capacity(num_bones as usize);
        for _ in 0..num_bones {
            let bone = Bone::read(reader, type_readers)?;
            bones.push(bone);
        }

        let mut bones_hierarchy = Vec::with_capacity(num_bones as usize);
        for _ in 0..num_bones {
            let hierarchy = BoneHierarchy::read(reader, num_bones)?;
            bones_hierarchy.push(hierarchy);
        }

        let num_vertex_decls = reader.read_u32::<LittleEndian>()?;
        let mut vertex_decls = Vec::with_capacity(num_vertex_decls as usize);
        for _ in 0..num_vertex_decls {
            let content = XnbAsset::read(reader, type_readers)?;
            let XnbAsset::VertexDeclaration(decl) = content else {
                anyhow::bail!("expected vertex declaration");
            };
            vertex_decls.push(decl);
        }

        let num_meshes = reader.read_u32::<LittleEndian>()?;
        let mut meshes = Vec::with_capacity(num_meshes as usize);
        for _ in 0..num_meshes {
            let mesh = Mesh::read(reader, type_readers)?;
            meshes.push(mesh);
        }

        let root_bone_ref = read_bone_ref(reader, num_bones)?;
        let tag = reader.read_u8()?;

        Ok(Model {
            bones,
            bones_hierarchy,
            vertex_decls,
            meshes,
            root_bone_ref,
            tag,
        })
    }
}

#[derive(Debug)]
pub struct Bone {
    pub name: String,
    pub transform: Mat4,
}

impl Bone {
    pub fn read(reader: &mut impl Read, type_readers: &[TypeReader]) -> anyhow::Result<Self> {
        let name = XnbAsset::read(reader, type_readers)?;
        let XnbAsset::String(name) = name else {
            anyhow::bail!("expected bone name to be a string");
        };
        let transform = reader.read_mat4()?;
        Ok(Bone { name, transform })
    }
}

#[derive(Debug)]
pub struct BoneHierarchy {
    pub parent_ref: u32,
    pub children_refs: Vec<u32>,
}

impl BoneHierarchy {
    pub fn read(reader: &mut impl Read, num_bones: u32) -> anyhow::Result<Self> {
        let parent_ref = read_bone_ref(reader, num_bones)?;
        let num_children = reader.read_u32::<LittleEndian>()? as usize;
        let mut children_refs = Vec::with_capacity(num_children);
        for _ in 0..num_children {
            let child_ref = read_bone_ref(reader, num_bones)?;
            children_refs.push(child_ref);
        }
        return Ok(BoneHierarchy {
            parent_ref,
            children_refs,
        });
    }
}

#[derive(Debug)]
pub struct Mesh {
    pub name: String,
    pub parent_bone_ref: u32,
    pub bounds: BoundingSphere,
    pub vertex_buffer: VertexBuffer,
    pub index_buffer: IndexBuffer,
    pub parts: Vec<MeshPart>,
    pub tag: u8,
}

impl Mesh {
    pub fn read(reader: &mut impl Read, type_readers: &[TypeReader]) -> anyhow::Result<Self> {
        let name = XnbAsset::read(reader, type_readers)?;
        let XnbAsset::String(name) = name else {
            anyhow::bail!("expected bone name to be a string");
        };

        let parent_bone_ref = read_bone_ref(reader, 0)?;
        let bounds = BoundingSphere::read(reader)?;

        let vertex_buffer = XnbAsset::read(reader, type_readers)?;
        let XnbAsset::VertexBuffer(vertex_buffer) = vertex_buffer else {
            anyhow::bail!("expected vertex buffer");
        };

        let index_buffer = XnbAsset::read(reader, type_readers)?;
        let XnbAsset::IndexBuffer(index_buffer) = index_buffer else {
            anyhow::bail!("expected index buffer");
        };

        let tag = reader.read_u8()?;

        let num_parts = reader.read_u32::<LittleEndian>()? as usize;
        let mut parts = Vec::with_capacity(num_parts);
        for _ in 0..num_parts {
            let part = MeshPart::read(reader)?;
            parts.push(part);
        }

        Ok(Mesh {
            name,
            parent_bone_ref,
            bounds,
            vertex_buffer,
            index_buffer,
            parts,
            tag,
        })
    }
}

#[derive(Debug)]
pub struct MeshPart {
    pub stream_offset: u32,
    pub base_vertex: u32,
    pub vertex_count: u32,
    pub start_index: u32,
    pub primitive_count: u32,
    pub vertex_decl_index: u32,
    pub tag: u8,
    pub shared_content_material_index: i32,
}

impl MeshPart {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let stream_offset = reader.read_u32::<LittleEndian>()?;
        let base_vertex = reader.read_u32::<LittleEndian>()?;
        let vertex_count = reader.read_u32::<LittleEndian>()?;
        let start_index = reader.read_u32::<LittleEndian>()?;
        let primitive_count = reader.read_u32::<LittleEndian>()?;
        let vertex_decl_index = reader.read_u32::<LittleEndian>()?;
        let tag = reader.read_u8()?;
        let shared_content_material_index = reader.read_7bit_encoded_i32()?;
        Ok(MeshPart {
            stream_offset,
            base_vertex,
            vertex_count,
            start_index,
            primitive_count,
            vertex_decl_index,
            tag,
            shared_content_material_index,
        })
    }
}

#[derive(Debug)]
pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

impl BoundingSphere {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let center = reader.read_vec3()?;
        let radius = reader.read_f32::<LittleEndian>()?;
        Ok(BoundingSphere { center, radius })
    }
}

#[derive(Debug)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl BoundingBox {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let min = reader.read_vec3()?;
        let max = reader.read_vec3()?;
        Ok(BoundingBox { min, max })
    }
}

fn read_bone_ref(reader: &mut impl Read, num_bones: u32) -> std::io::Result<u32> {
    let bone_ref = if num_bones <= 255 {
        reader.read_u8()? as u32
    } else {
        reader.read_u32::<LittleEndian>()?
    };
    Ok(bone_ref)
}
