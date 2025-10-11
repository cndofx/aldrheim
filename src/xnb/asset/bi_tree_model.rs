use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};

use crate::{
    read_ext::MyReadBytesExt,
    xnb::{
        TypeReader,
        asset::{
            XnbAsset, index_buffer::IndexBuffer, model::BoundingBox, vertex_buffer::VertexBuffer,
            vertex_decl::VertexDeclaration,
        },
    },
};

#[derive(Debug)]
pub struct BiTreeModel {
    pub trees: Vec<BiTree>,
}

impl BiTreeModel {
    pub fn read(reader: &mut impl Read, type_readers: &[TypeReader]) -> anyhow::Result<Self> {
        let num_trees = reader.read_i32::<LittleEndian>()?;
        let mut trees = Vec::with_capacity(num_trees as usize);
        for _ in 0..num_trees {
            let tree = BiTree::read(reader, type_readers)?;
            trees.push(tree);
        }
        Ok(BiTreeModel { trees })
    }
}

#[derive(Debug)]
pub struct BiTree {
    pub visible: bool,
    pub cast_shadows: bool,
    pub sway: f32,
    pub entity_influence: f32,
    pub ground_level: f32,
    pub num_vertices: i32,
    pub vertex_stride: i32,
    pub vertex_decl: VertexDeclaration,
    pub vertex_buffer: VertexBuffer,
    pub index_buffer: IndexBuffer,
    pub effect: XnbAsset,
    pub node: BiTreeNode,
}

impl BiTree {
    pub fn read(reader: &mut impl Read, type_readers: &[TypeReader]) -> anyhow::Result<Self> {
        let visible = reader.read_bool()?;
        let cast_shadows = reader.read_bool()?;
        let sway = reader.read_f32::<LittleEndian>()?;
        let entity_influence = reader.read_f32::<LittleEndian>()?;
        let ground_level = reader.read_f32::<LittleEndian>()?;
        let num_vertices = reader.read_i32::<LittleEndian>()?;
        let vertex_stride = reader.read_i32::<LittleEndian>()?;

        let vertex_decl = XnbAsset::read(reader, type_readers)?;
        let XnbAsset::VertexDeclaration(vertex_decl) = vertex_decl else {
            anyhow::bail!("expected vertex declaration");
        };

        let vertex_buffer = XnbAsset::read(reader, type_readers)?;
        let XnbAsset::VertexBuffer(vertex_buffer) = vertex_buffer else {
            anyhow::bail!("expected vertex buffer");
        };

        let index_buffer = XnbAsset::read(reader, type_readers)?;
        let XnbAsset::IndexBuffer(index_buffer) = index_buffer else {
            anyhow::bail!("expected index buffer");
        };

        let effect = XnbAsset::read(reader, type_readers)?;
        // TODO: validate that it's actually an effect

        let node = BiTreeNode::read(reader, type_readers)?;

        Ok(BiTree {
            visible,
            cast_shadows,
            sway,
            entity_influence,
            ground_level,
            num_vertices,
            vertex_stride,
            vertex_decl,
            vertex_buffer,
            index_buffer,
            effect,
            node,
        })
    }
}

#[derive(Debug)]
pub struct BiTreeNode {
    pub primitive_count: i32,
    pub start_index: i32,
    pub bounding_box: BoundingBox,
    pub child_a: Option<Box<BiTreeNode>>,
    pub child_b: Option<Box<BiTreeNode>>,
}

impl BiTreeNode {
    pub fn read(reader: &mut impl Read, type_readers: &[TypeReader]) -> anyhow::Result<Self> {
        let primitive_count = reader.read_i32::<LittleEndian>()?;
        let start_index = reader.read_i32::<LittleEndian>()?;
        let bounding_box = BoundingBox::read(reader)?;

        let child_a = if reader.read_bool()? {
            let node = BiTreeNode::read(reader, type_readers)?;
            Some(Box::new(node))
        } else {
            None
        };

        let child_b = if reader.read_bool()? {
            let node = BiTreeNode::read(reader, type_readers)?;
            Some(Box::new(node))
        } else {
            None
        };

        Ok(BiTreeNode {
            primitive_count,
            start_index,
            bounding_box,
            child_a,
            child_b,
        })
    }

    pub fn iter_children(&self) -> BiTreeNodeChildrenIter<'_> {
        BiTreeNodeChildrenIter {
            node: self,
            visited_child_a: false,
            visited_child_b: false,
        }
    }
}

pub struct BiTreeNodeChildrenIter<'a> {
    pub node: &'a BiTreeNode,
    pub visited_child_a: bool,
    pub visited_child_b: bool,
}

impl<'a> Iterator for BiTreeNodeChildrenIter<'a> {
    type Item = &'a BiTreeNode;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.visited_child_a {
            self.visited_child_a = true;
            if let Some(child) = &self.node.child_a {
                return Some(child.as_ref());
            }
        }

        if !self.visited_child_b {
            self.visited_child_b = true;
            if let Some(child) = &self.node.child_b {
                return Some(child.as_ref());
            }
        }

        None
    }
}
