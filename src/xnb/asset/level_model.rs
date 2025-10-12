use std::{collections::HashMap, io::Read};

use byteorder::{LittleEndian, ReadBytesExt};
use glam::{Mat4, Quat, Vec3};

use crate::{
    read_ext::MyReadBytesExt,
    xnb::{
        TypeReader,
        asset::{
            LIST_READER_NAME, XnbAsset, animation::AnimationChannel, bi_tree_model::BiTreeModel,
            color::Color, index_buffer::IndexBuffer, model::Model, vertex_buffer::VertexBuffer,
            vertex_decl::VertexDeclaration,
        },
    },
};

#[derive(Debug)]
pub struct LevelModel {
    pub model: BiTreeModel,
    pub animated_parts: Vec<AnimatedLevelPart>,
    pub lights: Vec<LevelModelLight>,
    pub effect_storages: Vec<EffectStorage>,
    pub physics_entity_storages: Vec<PhysicsEntityStorage>,
    pub liquids: Vec<Liquid>,
    pub force_fields: Vec<ForceField>,
    pub collision_meshes: Vec<TriangleMesh>,
    pub camera_mesh: Option<TriangleMesh>,
    pub trigger_areas: Vec<TriggerArea>,
    pub locators: Vec<Locator>,
    pub nav_mesh: NavMesh,
}

impl LevelModel {
    pub fn read(reader: &mut impl Read, type_readers: &[TypeReader]) -> anyhow::Result<Self> {
        let model = XnbAsset::read(reader, type_readers)?;
        let XnbAsset::BiTreeModel(model) = model else {
            anyhow::bail!("expected bi tree model");
        };

        let num_animated_parts = reader.read_i32::<LittleEndian>()?;
        let mut animated_parts = Vec::with_capacity(num_animated_parts as usize);
        for _ in 0..num_animated_parts {
            let part = AnimatedLevelPart::read(reader, type_readers)?;
            animated_parts.push(part);
        }

        let num_lights = reader.read_i32::<LittleEndian>()?;
        let mut lights = Vec::with_capacity(num_lights as usize);
        for _ in 0..num_lights {
            let light = LevelModelLight::read(reader)?;
            lights.push(light);
        }

        let num_effect_storages = reader.read_i32::<LittleEndian>()?;
        let mut effect_storages = Vec::with_capacity(num_effect_storages as usize);
        for _ in 0..num_effect_storages {
            let effect = EffectStorage::read(reader)?;
            effect_storages.push(effect);
        }

        let num_physics_entity_storages = reader.read_i32::<LittleEndian>()?;
        let mut physics_entity_storages = Vec::with_capacity(num_physics_entity_storages as usize);
        for _ in 0..num_physics_entity_storages {
            let entity = PhysicsEntityStorage::read(reader)?;
            physics_entity_storages.push(entity);
        }

        let num_liquids = reader.read_i32::<LittleEndian>()?;
        let mut liquids = Vec::with_capacity(num_liquids as usize);
        for _ in 0..num_liquids {
            let liquid = Liquid::read(reader)?;
            liquids.push(liquid);
        }

        let num_force_fields = reader.read_i32::<LittleEndian>()?;
        let mut force_fields = Vec::with_capacity(num_force_fields as usize);
        for _ in 0..num_force_fields {
            let force_field = ForceField::read(reader, type_readers)?;
            force_fields.push(force_field);
        }

        let max_collision_meshes = 10;
        let mut collision_meshes = Vec::with_capacity(max_collision_meshes);
        for _ in 0..max_collision_meshes {
            let exists = reader.read_bool()?;
            if !exists {
                continue;
            }

            let mesh = TriangleMesh::read(reader, type_readers)?;
            collision_meshes.push(mesh);
        }

        let camera_mesh = if reader.read_bool()? {
            Some(TriangleMesh::read(reader, type_readers)?)
        } else {
            None
        };

        let num_trigger_areas = reader.read_i32::<LittleEndian>()?;
        let mut trigger_areas = Vec::with_capacity(num_trigger_areas as usize);
        for _ in 0..num_trigger_areas {
            let area = TriggerArea::read(reader)?;
            trigger_areas.push(area);
        }

        let num_locators = reader.read_i32::<LittleEndian>()?;
        let mut locators = Vec::with_capacity(num_locators as usize);
        for _ in 0..num_locators {
            let locator = Locator::read(reader)?;
            locators.push(locator);
        }

        let nav_mesh = NavMesh::read(reader)?;

        Ok(LevelModel {
            model,
            animated_parts,
            lights,
            effect_storages,
            physics_entity_storages,
            liquids,
            force_fields,
            collision_meshes,
            camera_mesh,
            trigger_areas,
            locators,
            nav_mesh,
        })
    }
}

#[derive(Debug)]
pub struct AnimatedLevelPart {
    pub name: String,
    pub affect_shields: bool,
    pub model: Model,
    pub mesh_settings: HashMap<String, (bool, bool)>,
    pub liquids: Vec<Liquid>,
    pub locators: Vec<Locator>,
    pub animation_duration: f32,
    pub animation: AnimationChannel,
    pub effect_storages: Vec<EffectStorage>,
    pub light_refs: Vec<LevelModelLightRef>,
    pub collision: Option<AnimatedLevelPartCollision>,
    pub nav_mesh: Option<NavMesh>,
    pub children: Vec<AnimatedLevelPart>,
}

impl AnimatedLevelPart {
    pub fn read(reader: &mut impl Read, type_readers: &[TypeReader]) -> anyhow::Result<Self> {
        let name = reader.read_7bit_length_string()?;
        let affect_shields = reader.read_bool()?;

        let model = XnbAsset::read(reader, type_readers)?;
        let XnbAsset::Model(model) = model else {
            anyhow::bail!("expected model");
        };

        let num_settings = reader.read_i32::<LittleEndian>()?;
        let mut mesh_settings = HashMap::with_capacity(num_settings as usize);
        for _ in 0..num_settings {
            let setting = reader.read_7bit_length_string()?;
            let flag1 = reader.read_bool()?;
            let flag2 = reader.read_bool()?;
            mesh_settings.insert(setting, (flag1, flag2));
        }

        let num_liquids = reader.read_i32::<LittleEndian>()?;
        let mut liquids = Vec::with_capacity(num_liquids as usize);
        for _ in 0..num_liquids {
            let liquid = Liquid::read(reader)?;
            liquids.push(liquid);
        }

        let num_locators = reader.read_i32::<LittleEndian>()?;
        let mut locators = Vec::with_capacity(num_locators as usize);
        for _ in 0..num_locators {
            let locator = Locator::read(reader)?;
            locators.push(locator);
        }

        let animation_duration = reader.read_f32::<LittleEndian>()?;
        let animation = AnimationChannel::read(reader)?;

        let num_effect_storages = reader.read_i32::<LittleEndian>()?;
        let mut effect_storages = Vec::with_capacity(num_effect_storages as usize);
        for _ in 0..num_effect_storages {
            let effect = EffectStorage::read(reader)?;
            effect_storages.push(effect);
        }

        let num_lights = reader.read_i32::<LittleEndian>()?;
        let mut light_refs = Vec::with_capacity(num_lights as usize);
        for _ in 0..num_lights {
            let light = LevelModelLightRef::read(reader)?;
            light_refs.push(light);
        }

        let collision = if reader.read_bool()? {
            let material = CollisionMaterial::read(reader)?;
            let mesh = TriangleMesh::read(reader, type_readers)?;
            Some(AnimatedLevelPartCollision { material, mesh })
        } else {
            None
        };

        let nav_mesh = if reader.read_bool()? {
            Some(NavMesh::read(reader)?)
        } else {
            None
        };

        let num_children = reader.read_i32::<LittleEndian>()?;
        let mut children = Vec::with_capacity(num_children as usize);
        for _ in 0..num_children {
            let child = AnimatedLevelPart::read(reader, type_readers)?;
            children.push(child);
        }

        Ok(AnimatedLevelPart {
            name,
            affect_shields,
            model,
            mesh_settings,
            liquids,
            locators,
            animation_duration,
            animation,
            effect_storages,
            light_refs,
            collision,
            nav_mesh,
            children,
        })
    }
}

#[derive(Debug)]
pub struct AnimatedLevelPartCollision {
    pub material: CollisionMaterial,
    pub mesh: TriangleMesh,
}

#[repr(u8)]
#[derive(strum::FromRepr, Debug)]
pub enum CollisionMaterial {
    Generic,
    Gravel,
    Grass,
    Wood,
    Snow,
    Stone,
    Mud,
    Reflect,
    Water,
    Lava,
}

impl CollisionMaterial {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let value = reader.read_u8()?;
        let kind = CollisionMaterial::from_repr(value)
            .ok_or_else(|| anyhow::anyhow!("unknown collision material: {value}"))?;
        Ok(kind)
    }
}

#[derive(Debug)]
pub struct LevelModelLight {
    pub name: String,
    pub position: Vec3,
    pub direction: Vec3,
    pub kind: LevelModelLightKind,
    pub variation: LevelModelLightVariation,
    pub reach: f32,
    pub use_attenuation: bool,
    pub cutoff_angle: f32,
    pub sharpness: f32,
    pub diffuse_color: Color,
    pub ambient_color: Color,
    pub specular_amount: f32,
    pub variation_amount: f32,
    pub variation_speed: f32,
    pub shadow_map_size: i32,
    pub casts_shadows: bool,
}

impl LevelModelLight {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let name = reader.read_7bit_length_string()?;
        let position = reader.read_vec3()?;
        let direction = reader.read_vec3()?;
        let kind = LevelModelLightKind::read(reader)?;
        let variation = LevelModelLightVariation::read(reader)?;
        let reach = reader.read_f32::<LittleEndian>()?;
        let use_attenuation = reader.read_bool()?;
        let cutoff_angle = reader.read_f32::<LittleEndian>()?;
        let sharpness = reader.read_f32::<LittleEndian>()?;
        let diffuse_color = Color::read(reader)?;
        let ambient_color = Color::read(reader)?;
        let specular_amount = reader.read_f32::<LittleEndian>()?;
        let variation_speed = reader.read_f32::<LittleEndian>()?;
        let variation_amount = reader.read_f32::<LittleEndian>()?;
        let shadow_map_size = reader.read_i32::<LittleEndian>()?;
        let casts_shadows = reader.read_bool()?;

        Ok(LevelModelLight {
            name,
            position,
            direction,
            kind,
            variation,
            reach,
            use_attenuation,
            cutoff_angle,
            sharpness,
            diffuse_color,
            ambient_color,
            specular_amount,
            variation_amount,
            variation_speed,
            shadow_map_size,
            casts_shadows,
        })
    }
}

#[repr(u8)]
#[derive(strum::FromRepr, Debug)]
pub enum LevelModelLightKind {
    Point,
    Directional,
    Spot,
    Custom = 10,
}

impl LevelModelLightKind {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let value = reader.read_u32::<LittleEndian>()?;
        let kind = LevelModelLightKind::from_repr(value as u8)
            .ok_or_else(|| anyhow::anyhow!("unknown level model light kind: {value}"))?;
        Ok(kind)
    }
}

#[repr(u8)]
#[derive(strum::FromRepr, Debug)]
pub enum LevelModelLightVariation {
    None = 0,
    Sine,
    Flicker,
    Candle,
    Strobe,
}

impl LevelModelLightVariation {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let value = reader.read_u32::<LittleEndian>()?;
        let kind = LevelModelLightVariation::from_repr(value as u8)
            .ok_or_else(|| anyhow::anyhow!("unknown level model light variation: {value}"))?;
        Ok(kind)
    }
}

#[derive(Debug)]
pub struct LevelModelLightRef {
    name: String,
    transform: Mat4,
}

impl LevelModelLightRef {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let name = reader.read_7bit_length_string()?;
        let transform = reader.read_mat4()?;

        Ok(LevelModelLightRef { name, transform })
    }
}

#[derive(Debug)]
pub struct EffectStorage {
    pub name: String,
    pub position: Vec3,
    pub forward: Vec3,
    pub range: f32,
    pub effect: String,
}

impl EffectStorage {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let name = reader.read_7bit_length_string()?;
        let position = reader.read_vec3()?;
        let forward = reader.read_vec3()?;
        let range = reader.read_f32::<LittleEndian>()?;
        let effect = reader.read_7bit_length_string()?;

        Ok(EffectStorage {
            name,
            position,
            forward,
            range,
            effect,
        })
    }
}

#[derive(Debug)]
pub struct PhysicsEntityStorage {
    pub transform: Mat4,
    pub template: String,
}

impl PhysicsEntityStorage {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let transform = reader.read_mat4()?;
        let template = reader.read_7bit_length_string()?;

        Ok(PhysicsEntityStorage {
            transform,
            template,
        })
    }
}

#[derive(Debug)]
pub enum Liquid {
    Water(Water),
    Lava(Lava),
}

impl Liquid {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let idx = reader.read_7bit_encoded_i32()?;
        dbg!(idx);
        todo!();
    }
}

#[derive(Debug)]
pub struct Water {}

#[derive(Debug)]
pub struct Lava {}

#[derive(Debug)]
pub struct ForceField {
    pub color: Color,
    pub width: f32,
    pub alpha_power: f32,
    pub alpha_falloff_power: f32,
    pub max_radius: f32,
    pub ripple_distortion: f32,
    pub map_distortion: f32,
    pub vertex_color_enabled: bool,
    pub displacement_map: String,
    pub ttl: f32, // time to live?
    pub vertex_buffer: VertexBuffer,
    pub index_buffer: IndexBuffer,
    pub vertex_declaration: VertexDeclaration,
    pub vertex_stride: i32,
    pub num_vertices: i32,
    pub primitive_count: i32,
}

impl ForceField {
    pub fn read(reader: &mut impl Read, type_readers: &[TypeReader]) -> anyhow::Result<Self> {
        let color = Color::read(reader)?;
        let width = reader.read_f32::<LittleEndian>()?;
        let alpha_power = reader.read_f32::<LittleEndian>()?;
        let alpha_falloff_power = reader.read_f32::<LittleEndian>()?;
        let max_radius = reader.read_f32::<LittleEndian>()?;
        let ripple_distortion = reader.read_f32::<LittleEndian>()?;
        let map_distortion = reader.read_f32::<LittleEndian>()?;
        let vertex_color_enabled = reader.read_bool()?;
        let displacement_map = reader.read_7bit_length_string()?;
        let ttl = reader.read_f32::<LittleEndian>()?;

        let vertex_buffer = XnbAsset::read(reader, type_readers)?;
        let XnbAsset::VertexBuffer(vertex_buffer) = vertex_buffer else {
            anyhow::bail!("expected vertex buffer");
        };

        let index_buffer = XnbAsset::read(reader, type_readers)?;
        let XnbAsset::IndexBuffer(index_buffer) = index_buffer else {
            anyhow::bail!("expected index buffer");
        };

        let vertex_declaration = XnbAsset::read(reader, type_readers)?;
        let XnbAsset::VertexDeclaration(vertex_declaration) = vertex_declaration else {
            anyhow::bail!("expected vertex declaration");
        };

        let vertex_stride = reader.read_i32::<LittleEndian>()?;
        let num_vertices = reader.read_i32::<LittleEndian>()?;
        let primitive_count = reader.read_i32::<LittleEndian>()?;

        Ok(ForceField {
            color,
            width,
            alpha_power,
            alpha_falloff_power,
            max_radius,
            ripple_distortion,
            map_distortion,
            vertex_color_enabled,
            displacement_map,
            ttl,
            vertex_buffer,
            index_buffer,
            vertex_declaration,
            vertex_stride,
            num_vertices,
            primitive_count,
        })
    }
}

#[derive(Debug)]
pub struct TriangleMesh {
    vertices: Vec<Vec3>,
    indices: Vec<[u32; 3]>,
}

impl TriangleMesh {
    pub fn read(reader: &mut impl Read, type_readers: &[TypeReader]) -> anyhow::Result<Self> {
        let reader_index = reader.read_7bit_encoded_i32()? as usize;
        let reader_name = &type_readers[reader_index - 1].name;
        if !reader_name.starts_with(LIST_READER_NAME) {
            anyhow::bail!("expected list");
        }

        let num_vertices = reader.read_u32::<LittleEndian>()? as usize;
        let mut vertices = Vec::with_capacity(num_vertices);
        for _ in 0..num_vertices {
            let vertex = reader.read_vec3()?;
            vertices.push(vertex);
        }

        let num_indices = reader.read_u32::<LittleEndian>()? as usize;
        let mut indices = Vec::with_capacity(num_indices);
        for _ in 0..num_indices {
            let i0 = reader.read_u32::<LittleEndian>()?;
            let i1 = reader.read_u32::<LittleEndian>()?;
            let i2 = reader.read_u32::<LittleEndian>()?;
            indices.push([i0, i1, i2]);
        }

        Ok(TriangleMesh { vertices, indices })
    }
}

#[derive(Debug)]
pub struct TriggerArea {
    name: String,
    position: Vec3,
    side_lengths: Vec3,
    orientation: Quat,
}

impl TriggerArea {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let name = reader.read_7bit_length_string()?;
        let position = reader.read_vec3()?;
        let side_lengths = reader.read_vec3()?;
        let orientation = reader.read_quat()?;

        Ok(TriggerArea {
            name,
            position,
            side_lengths,
            orientation,
        })
    }
}

#[derive(Debug)]
pub struct Locator {
    pub name: String,
    pub transform: Mat4,
    pub radius: f32,
}

impl Locator {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let name = reader.read_7bit_length_string()?;
        let transform = reader.read_mat4()?;
        let radius = reader.read_f32::<LittleEndian>()?;

        Ok(Locator {
            name,
            transform,
            radius,
        })
    }
}

#[derive(Debug)]
pub struct NavMesh {
    pub vertices: Vec<Vec3>,
    pub triangles: Vec<NavMeshTriangle>,
}

impl NavMesh {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let num_vertices = reader.read_u16::<LittleEndian>()?;
        let mut vertices = Vec::with_capacity(num_vertices as usize);
        for _ in 0..num_vertices {
            let vertex = reader.read_vec3()?;
            vertices.push(vertex);
        }

        let num_triangles = reader.read_u16::<LittleEndian>()?;
        let mut triangles = Vec::with_capacity(num_triangles as usize);
        for _ in 0..num_triangles {
            let triangle = NavMeshTriangle::read(reader)?;
            triangles.push(triangle);
        }

        Ok(NavMesh {
            vertices,
            triangles,
        })
    }
}

#[derive(Debug)]
pub struct NavMeshTriangle {
    pub vertex_a: u16,
    pub vertex_b: u16,
    pub vertex_c: u16,
    pub neighbor_a: u16,
    pub neighbor_b: u16,
    pub neighbor_c: u16,
    pub cost_ab: f32,
    pub cost_bc: f32,
    pub cost_ca: f32,
    pub properties: MovementProperties,
}

impl NavMeshTriangle {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let vertex_a = reader.read_u16::<LittleEndian>()?;
        let vertex_b = reader.read_u16::<LittleEndian>()?;
        let vertex_c = reader.read_u16::<LittleEndian>()?;
        let neighbor_a = reader.read_u16::<LittleEndian>()?;
        let neighbor_b = reader.read_u16::<LittleEndian>()?;
        let neighbor_c = reader.read_u16::<LittleEndian>()?;
        let cost_ab = reader.read_f32::<LittleEndian>()?;
        let cost_bc = reader.read_f32::<LittleEndian>()?;
        let cost_ca = reader.read_f32::<LittleEndian>()?;
        let properties = MovementProperties::read(reader)?;
        Ok(NavMeshTriangle {
            vertex_a,
            vertex_b,
            vertex_c,
            neighbor_a,
            neighbor_b,
            neighbor_c,
            cost_ab,
            cost_bc,
            cost_ca,
            properties,
        })
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct MovementProperties: u8 {
        const DEFAULT = 0;
        const WATER = 1;
        const JUMP = 2;
        const FLY = 4;
        const DYNAMIC = 128;
        const ALL = 255;
    }
}

impl MovementProperties {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let value = reader.read_u8()?;
        let properties = MovementProperties::from_bits(value)
            .ok_or_else(|| anyhow::anyhow!("unknown movement properties: {value}"))?;
        Ok(properties)
    }
}
