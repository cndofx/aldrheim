use std::rc::Rc;

use glam::{Mat4, Vec3};

use crate::{
    asset_manager::{BiTreeAsset, ModelAsset},
    renderer::{DrawCommand, Renderable, RenderableBounds, Renderer, camera::Camera},
    scene::vfx::VisualEffectNode,
    xnb::asset::model::BoundingBox,
};

pub mod level;
pub mod trigger;
pub mod vfx;

pub struct Scene {
    pub root_node: SceneNode,
    pub camera: Camera,
}

impl Scene {
    pub fn new() -> Scene {
        Scene {
            root_node: SceneNode {
                name: "Root Node".into(),
                visible: true,
                transform: Mat4::IDENTITY,
                children: Vec::new(),
                kind: SceneNodeKind::Empty,
            },
            camera: Camera {
                position: Vec3::new(0.0, 5.0, 0.0),
                pitch_radians: 0.0,
                yaw_radians: 0.0,
                fov_y_radians: 75.0f32.to_radians(),
                z_near: 0.1,
                z_far: 10000.0,
            },
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.root_node.update(dt);
    }

    pub fn render(&mut self, renderer: &Renderer) -> Vec<DrawCommand> {
        if !self.root_node.visible {
            return Vec::new();
        }

        let mut draw_commands = Vec::new();
        let mut transform_stack = Vec::new();
        transform_stack.push(Mat4::IDENTITY);
        self.root_node
            .render(&mut draw_commands, &mut transform_stack, renderer);

        draw_commands
    }
}

pub struct SceneNode {
    pub name: String,
    pub visible: bool,
    pub transform: Mat4,
    pub children: Vec<SceneNode>,
    pub kind: SceneNodeKind,
}

impl SceneNode {
    pub fn update(&mut self, dt: f32) {
        match &mut self.kind {
            SceneNodeKind::Empty => {}
            SceneNodeKind::Model(_) => {}
            SceneNodeKind::BiTree(_) => {}
            SceneNodeKind::VisualEffect(vfx_node) => vfx_node.update(dt, self.transform),
        }

        for child in self.children.iter_mut() {
            child.update(dt);
        }
    }

    pub fn render(
        &mut self,
        draw_commands: &mut Vec<DrawCommand>,
        transform_stack: &mut Vec<Mat4>,
        renderer: &Renderer,
    ) {
        if !self.visible {
            return;
        }

        let parent_transform = *transform_stack.last().unwrap();
        let current_transform = parent_transform * self.transform;
        transform_stack.push(current_transform);

        match &mut self.kind {
            SceneNodeKind::Model(model_node) => draw_commands.push(DrawCommand {
                renderable: Renderable::Model(model_node.clone()),
                bounds: None, // TODO
                transform: current_transform,
            }),
            // TODO: it seems like bitree parent nodes draw all of the same mesh as their child nodes combined?
            // should i render just the parent nodes or just the leaf child nodes?
            SceneNodeKind::BiTree(bitree_node) => draw_commands.push(DrawCommand {
                renderable: Renderable::BiTreeNode(bitree_node.clone()),
                bounds: Some(RenderableBounds::Box(bitree_node.bounding_box.clone())),
                transform: current_transform,
            }),
            SceneNodeKind::VisualEffect(vfx_node) => {
                if let Some(draw) = vfx_node.render(current_transform, renderer) {
                    draw_commands.push(draw);
                }
            }
            _ => {}
        }

        for child in self.children.iter_mut() {
            child.render(draw_commands, transform_stack, renderer);
        }

        transform_stack.pop();
    }
}

pub enum SceneNodeKind {
    Empty,
    Model(ModelNode),
    BiTree(BiTreeNode),
    VisualEffect(VisualEffectNode),
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
    pub bounding_box: BoundingBox,
}
