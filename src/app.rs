use std::{path::PathBuf, sync::Arc, time::Instant};

use glam::{Mat4, Quat, Vec3};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{WindowAttributes, WindowId},
};

#[cfg(target_os = "linux")]
use winit::platform::wayland::WindowAttributesExtWayland;

use crate::{
    asset_manager::AssetManager,
    renderer::Renderer,
    scene::{MeshNode, Scene, SceneNode, SceneNodeKind},
};

pub struct App {
    start_time: Instant,
    asset_manager: AssetManager,
    renderer: Option<Renderer>,
    scene: Option<Scene>,
}

impl App {
    pub fn new(magicka_path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let asset_manager = AssetManager::new(magicka_path);

        let app = App {
            start_time: Instant::now(),
            asset_manager,
            renderer: None,
            scene: None,
        };
        Ok(app)
    }

    fn update(&mut self) {}

    fn handle_key_input(
        &mut self,
        code: KeyCode,
        state: ElementState,
        event_loop: &ActiveEventLoop,
    ) {
        match (code, state) {
            (KeyCode::Escape, ElementState::Pressed) => event_loop.exit(),
            _ => {}
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = WindowAttributes::default().with_title("Aldrheim");

        #[cfg(target_os = "linux")]
        let window_attributes = window_attributes.with_name("cndofx.Aldrheim", "");

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        let renderer = pollster::block_on(Renderer::new(window)).unwrap();
        self.renderer = Some(renderer);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let renderer = self.renderer.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                renderer.resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => {
                let scene = self
                    .scene
                    .get_or_insert_with(|| load_scene(&self.asset_manager, renderer).unwrap());

                let time = self.start_time.elapsed().as_secs_f32();
                let radius = 5.0;
                let x = time.sin() * radius;
                let z = time.cos() * radius;
                scene.camera.position = Vec3::new(x, 0.0, z);
                scene.camera.look_at(Vec3::ZERO);

                let draws = scene.render();
                match renderer.render(&draws, &scene.camera) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        renderer.reconfigure_surface();
                    }
                    Err(e) => {
                        log::error!("{e}");
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => self.handle_key_input(code, state, event_loop),
            _ => {}
        }
    }
}

fn load_scene(asset_manager: &AssetManager, renderer: &mut Renderer) -> anyhow::Result<Scene> {
    let basic_staff_mesh = renderer.load_model_from_path(
        "Content/Models/Items_Wizard/staff_basic_0.xnb",
        asset_manager,
    )?;

    let plus_staff_mesh = renderer.load_model_from_path(
        "Content/Models/Items_Wizard/staff_plus_0.xnb",
        asset_manager,
    )?;

    let mut scene = Scene::new();
    scene.root_node.children.push(SceneNode {
        name: "Plus Staff".into(),
        transform: Mat4::from_scale_rotation_translation(
            Vec3::ONE,
            Quat::IDENTITY,
            Vec3::new(0.0, 0.0, 0.0),
        ),
        children: Vec::new(),
        kind: SceneNodeKind::Mesh(MeshNode {
            mesh: plus_staff_mesh,
        }),
    });
    scene.root_node.children.push(SceneNode {
        name: "Basic Staff 1".into(),
        transform: Mat4::from_scale_rotation_translation(
            Vec3::ONE,
            Quat::IDENTITY,
            Vec3::new(2.0, 0.0, 0.0),
        ),
        children: Vec::new(),
        kind: SceneNodeKind::Mesh(MeshNode {
            mesh: basic_staff_mesh.clone(),
        }),
    });
    scene.root_node.children.push(SceneNode {
        name: "Basic Staff 2".into(),
        transform: Mat4::from_scale_rotation_translation(
            Vec3::ONE,
            Quat::IDENTITY,
            Vec3::new(-2.0, 0.0, 0.0),
        ),
        children: Vec::new(),
        kind: SceneNodeKind::Mesh(MeshNode {
            mesh: basic_staff_mesh.clone(),
        }),
    });

    Ok(scene)
}
