use std::rc::Rc;

use glam::{Mat4, Vec3};

use crate::{
    asset_manager::{BiTreeAsset, ModelAsset},
    renderer::{ModelDrawCommand, Renderable},
};

pub struct Scene {
    pub root_node: SceneNode,
    pub camera: Camera,
}

impl Scene {
    pub fn new() -> Scene {
        Scene {
            root_node: SceneNode {
                name: "Root Node".into(),
                transform: Mat4::IDENTITY,
                children: Vec::new(),
                kind: SceneNodeKind::Empty,
            },
            camera: Camera {
                position: Vec3::new(0.0, 0.0, -5.0),
                pitch_radians: 0.0,
                yaw_radians: 90.0f32.to_radians(),
                fov_y_radians: 75.0f32.to_radians(),
                z_near: 0.1,
                z_far: 10000.0,
            },
        }
    }

    pub fn render(&self) -> Vec<ModelDrawCommand> {
        let mut draw_commands = Vec::new();
        let mut transform_stack = Vec::new();
        transform_stack.push(Mat4::IDENTITY);
        self.root_node
            .render(&mut draw_commands, &mut transform_stack);

        draw_commands
    }
}

pub struct SceneNode {
    pub name: String,
    pub transform: Mat4,
    pub children: Vec<SceneNode>,
    pub kind: SceneNodeKind,
}

impl SceneNode {
    pub fn render(
        &self,
        draw_commands: &mut Vec<ModelDrawCommand>,
        transform_stack: &mut Vec<Mat4>,
    ) {
        let parent_transform = *transform_stack.last().unwrap();
        let current_transform = parent_transform * self.transform;
        transform_stack.push(current_transform);

        match &self.kind {
            SceneNodeKind::Model(model_node) => draw_commands.push(ModelDrawCommand {
                renderable: Renderable::Model(model_node.clone()),
                transform: current_transform,
            }),
            // TODO: it seems like bitree parent nodes draw all of the same mesh as their child nodes combined?
            // should i render just the parent nodes or just the leaf child nodes?
            SceneNodeKind::BiTree(bitree_node) => draw_commands.push(ModelDrawCommand {
                renderable: Renderable::BiTreeNode(bitree_node.clone()),
                transform: current_transform,
            }),
            _ => {}
        }

        for child in &self.children {
            child.render(draw_commands, transform_stack);
        }

        transform_stack.pop();
    }
}

pub enum SceneNodeKind {
    Empty,
    Model(ModelNode),
    BiTree(BiTreeNode),
}

#[derive(Clone)]
pub struct ModelNode {
    pub model: Rc<ModelAsset>,
}

#[derive(Clone)]
pub struct BiTreeNode {
    pub tree: Rc<BiTreeAsset>,
    pub start_index: u32,
    pub index_count: u32,
}

#[derive(Debug)]
pub struct Camera {
    pub position: Vec3,
    pub pitch_radians: f32,
    pub yaw_radians: f32,
    pub fov_y_radians: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Camera {
    pub const UP: Vec3 = Vec3::Y;

    pub fn look_at(&mut self, target: Vec3) {
        let forward = (target - self.position).normalize();
        self.yaw_radians = forward.x.atan2(forward.z);
        self.pitch_radians = forward.y.asin();
    }

    pub fn forward_right_up(&self) -> (Vec3, Vec3, Vec3) {
        let up = Self::UP;
        let forward_x = self.yaw_radians.sin() * self.pitch_radians.cos();
        let forward_y = self.pitch_radians.sin();
        let forward_z = self.yaw_radians.cos() * self.pitch_radians.cos();
        let forward = Vec3::new(forward_x, forward_y, forward_z).normalize();
        let right = up.cross(forward);
        (forward, right, up)
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_lh(self.position, self.forward_right_up().0, Self::UP)
    }
}
