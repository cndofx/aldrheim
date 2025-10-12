use std::io::Read;

use crate::{
    read_ext::MyReadBytesExt,
    xnb::{
        TypeReader,
        asset::{
            additive_effect::AdditiveEffect, bi_tree_model::BiTreeModel, index_buffer::IndexBuffer,
            level_model::LevelModel, model::Model, render_deferred_effect::RenderDeferredEffect,
            render_deferred_liquid_effect::RenderDeferredLiquidEffect, texture_2d::Texture2D,
            texture_3d::Texture3D, vertex_buffer::VertexBuffer, vertex_decl::VertexDeclaration,
        },
    },
};

pub mod additive_effect;
pub mod animation;
pub mod bi_tree_model;
pub mod color;
pub mod index_buffer;
pub mod level_model;
pub mod model;
pub mod render_deferred_effect;
pub mod render_deferred_liquid_effect;
pub mod texture_2d;
pub mod texture_3d;
pub mod vertex_buffer;
pub mod vertex_decl;

const STRING_READER_NAME: &str = "Microsoft.Xna.Framework.Content.StringReader";
const LIST_READER_NAME: &str = "Microsoft.Xna.Framework.Content.ListReader";
const TEXTURE_2D_READER_NAME: &str = "Microsoft.Xna.Framework.Content.Texture2DReader";
const TEXTURE_3D_READER_NAME: &str = "Microsoft.Xna.Framework.Content.Texture3DReader";
const MODEL_READER_NAME: &str = "Microsoft.Xna.Framework.Content.ModelReader";
const VERTEX_DECL_READER_NAME: &str = "Microsoft.Xna.Framework.Content.VertexDeclarationReader";
const VERTEX_BUFFER_READER_NAME: &str = "Microsoft.Xna.Framework.Content.VertexBufferReader";
const INDEX_BUFFER_READER_NAME: &str = "Microsoft.Xna.Framework.Content.IndexBufferReader";

const BI_TREE_MODEL_READER_NAME: &str = "PolygonHead.Pipeline.BiTreeModelReader";
const ADDITIVE_EFFECT_READER_NAME: &str = "PolygonHead.Pipeline.AdditiveEffectReader";
const RENDER_DEFERRED_EFFECT_READER_NAME: &str = "PolygonHead.Pipeline.RenderDeferredEffectReader";
const RENDER_DEFERRED_LIQUID_EFFECT_READER_NAME: &str =
    "PolygonHead.Pipeline.RenderDeferredLiquidEffectReader";

const LEVEL_MODEL_READER_NAME: &str = "Magicka.ContentReaders.LevelModelReader";

#[derive(strum::AsRefStr, Debug)]
pub enum XnbAsset {
    Null,
    String(String),
    Texture2D(Texture2D),
    Texture3D(Texture3D),
    Model(Model),
    VertexDeclaration(VertexDeclaration),
    VertexBuffer(VertexBuffer),
    IndexBuffer(IndexBuffer),
    BiTreeModel(BiTreeModel),
    AdditiveEffect(AdditiveEffect),
    RenderDeferredEffect(RenderDeferredEffect),
    RenderDeferredLiquidEffect(RenderDeferredLiquidEffect),
    LevelModel(LevelModel),
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
            STRING_READER_NAME => {
                let string = reader.read_7bit_length_string()?;
                Ok(XnbAsset::String(string))
            }
            TEXTURE_2D_READER_NAME => {
                let texture = Texture2D::read(reader)?;
                Ok(XnbAsset::Texture2D(texture))
            }
            TEXTURE_3D_READER_NAME => {
                let texture = Texture3D::read(reader)?;
                Ok(XnbAsset::Texture3D(texture))
            }
            MODEL_READER_NAME => {
                let model = Model::read(reader, type_readers)?;
                Ok(XnbAsset::Model(model))
            }
            VERTEX_DECL_READER_NAME => {
                let decl = VertexDeclaration::read(reader)?;
                Ok(XnbAsset::VertexDeclaration(decl))
            }
            VERTEX_BUFFER_READER_NAME => {
                let buffer = VertexBuffer::read(reader)?;
                Ok(XnbAsset::VertexBuffer(buffer))
            }
            INDEX_BUFFER_READER_NAME => {
                let buffer = IndexBuffer::read(reader)?;
                Ok(XnbAsset::IndexBuffer(buffer))
            }
            BI_TREE_MODEL_READER_NAME => {
                let model = BiTreeModel::read(reader, type_readers)?;
                Ok(XnbAsset::BiTreeModel(model))
            }
            ADDITIVE_EFFECT_READER_NAME => {
                let effect = AdditiveEffect::read(reader)?;
                Ok(XnbAsset::AdditiveEffect(effect))
            }
            RENDER_DEFERRED_EFFECT_READER_NAME => {
                let effect = RenderDeferredEffect::read(reader)?;
                Ok(XnbAsset::RenderDeferredEffect(effect))
            }
            RENDER_DEFERRED_LIQUID_EFFECT_READER_NAME => {
                let effect = RenderDeferredLiquidEffect::read(reader)?;
                Ok(XnbAsset::RenderDeferredLiquidEffect(effect))
            }
            LEVEL_MODEL_READER_NAME => {
                let model = LevelModel::read(reader, type_readers)?;
                Ok(XnbAsset::LevelModel(model))
            }
            _ => {
                anyhow::bail!("unknown type reader: {}", type_reader.name);
            }
        }
    }
}
