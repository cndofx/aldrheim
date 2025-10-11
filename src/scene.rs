use std::rc::Rc;

use glam::{Mat4, Vec3};

use crate::renderer::{self, MeshDrawCommand};

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
                position: Vec3::new(10.0, 0.0, 0.0),
                direction: Vec3::new(-1.0, 0.0, 0.0),
                fov_y_degrees: 75.0,
                z_near: 1.0,
                z_far: 10000.0,
            },
        }
    }

    pub fn render(&self) -> Vec<MeshDrawCommand> {
        let mut draw_commands = Vec::new();
        let mut transform_stack = Vec::new();
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
        draw_commands: &mut Vec<MeshDrawCommand>,
        transform_stack: &mut Vec<Mat4>,
    ) {
        let parent_transform = *transform_stack.last().unwrap_or(&Mat4::IDENTITY);
        let current_transform = parent_transform * self.transform;
        transform_stack.push(current_transform);

        match &self.kind {
            SceneNodeKind::Mesh(mesh) => draw_commands.push(MeshDrawCommand {
                mesh: mesh.mesh.clone(),
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
    Mesh(MeshNode),
}

pub struct MeshNode {
    pub mesh: Rc<renderer::Mesh>,
}

#[derive(Debug)]
pub struct Camera {
    pub position: Vec3,
    pub direction: Vec3,
    pub fov_y_degrees: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Camera {
    const UP: Vec3 = Vec3::Y;

    /// rotate this camera to look at the target position
    pub fn look_at(&mut self, target: Vec3) {
        self.direction = (target - self.position).normalize();
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_lh(self.position, self.direction, Self::UP)
    }
}
