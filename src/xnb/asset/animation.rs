use std::{collections::HashMap, io::Read};

use byteorder::{LittleEndian, ReadBytesExt};
use glam::{Quat, Vec3};

use crate::read_ext::MyReadBytesExt;

#[derive(Debug)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    pub channels: HashMap<String, AnimationChannel>,
}

#[derive(Debug)]
pub struct AnimationChannel {
    pub keyframes: Vec<AnimationKeyframe>,
}

impl AnimationChannel {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let num_frames = reader.read_i32::<LittleEndian>()?;
        let mut keyframes = Vec::with_capacity(num_frames as usize);
        for _ in 0..num_frames {
            let keyframe = AnimationKeyframe::read(reader)?;
            keyframes.push(keyframe);
        }

        Ok(AnimationChannel { keyframes })
    }
}

#[derive(Debug)]
pub struct AnimationKeyframe {
    pub time: f32,
    pub pose: AnimationPose,
}

impl AnimationKeyframe {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let time = reader.read_f32::<LittleEndian>()?;
        let pose = AnimationPose::read(reader)?;

        Ok(AnimationKeyframe { time, pose })
    }
}

#[derive(Debug)]
pub struct AnimationPose {
    pub translation: Vec3,
    pub orientation: Quat,
    pub scale: Vec3,
}

impl AnimationPose {
    pub fn read(reader: &mut impl Read) -> anyhow::Result<Self> {
        let translation = reader.read_vec3()?;
        let orientation = reader.read_quat()?;
        let scale = reader.read_vec3()?;

        Ok(AnimationPose {
            translation,
            orientation,
            scale,
        })
    }
}
