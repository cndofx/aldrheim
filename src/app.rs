use std::{
    f32::consts::TAU,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use glam::{Mat4, Quat, Vec3};
use rand::Rng;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, WindowAttributes, WindowId},
};

#[cfg(target_os = "linux")]
use winit::platform::wayland::WindowAttributesExtWayland;

use crate::{
    asset_manager::AssetManager,
    renderer::Renderer,
    scene::{ModelNode, Scene, SceneNode, SceneNodeKind},
};

pub struct App {
    asset_manager: AssetManager,
    renderer: Option<Renderer>,
    scene: Option<Scene>,

    last_time: Instant,
    camera_input_state: InputState,
    camera_speed: f32,
    cursor_grabbed: bool,
}

impl App {
    pub fn new(magicka_path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let asset_manager = AssetManager::new(magicka_path);

        let app = App {
            asset_manager,
            renderer: None,
            scene: None,

            last_time: Instant::now(),
            camera_input_state: InputState::default(),
            camera_speed: 2.0,
            cursor_grabbed: false,
        };
        Ok(app)
    }

    fn update(&mut self, dt: f64) {
        // println!("{:.2} fps", 1.0 / dt);

        let scene = self.scene.as_mut().unwrap();

        let mut camera_move_direction = Vec3::ZERO;
        if self.camera_input_state.forward {
            camera_move_direction.z += 1.0;
        }
        if self.camera_input_state.backward {
            camera_move_direction.z -= 1.0;
        }
        if self.camera_input_state.left {
            camera_move_direction.x -= 1.0;
        }
        if self.camera_input_state.right {
            camera_move_direction.x += 1.0;
        }
        if self.camera_input_state.up {
            camera_move_direction.y += 1.0;
        }
        if self.camera_input_state.down {
            camera_move_direction.y -= 1.0;
        }

        if camera_move_direction.length_squared() > 0.1 {
            camera_move_direction = camera_move_direction.normalize();

            let (forward, right, up) = scene.camera.forward_right_up();

            let mut amount = self.camera_speed * (dt as f32);
            if self.camera_input_state.fast {
                amount *= 3.0;
            }

            scene.camera.position += forward * camera_move_direction.z * amount;
            scene.camera.position += right * camera_move_direction.x * amount;
            scene.camera.position += up * camera_move_direction.y * amount;
        }
    }

    fn handle_key_input(
        &mut self,
        code: KeyCode,
        state: ElementState,
        _event_loop: &ActiveEventLoop,
    ) {
        match (code, state) {
            (KeyCode::Escape, ElementState::Pressed) => self.grab_cursor(false).unwrap(),
            (KeyCode::KeyW, ElementState::Pressed) => self.camera_input_state.forward = true,
            (KeyCode::KeyW, ElementState::Released) => self.camera_input_state.forward = false,
            (KeyCode::KeyS, ElementState::Pressed) => self.camera_input_state.backward = true,
            (KeyCode::KeyS, ElementState::Released) => self.camera_input_state.backward = false,
            (KeyCode::KeyA, ElementState::Pressed) => self.camera_input_state.left = true,
            (KeyCode::KeyA, ElementState::Released) => self.camera_input_state.left = false,
            (KeyCode::KeyD, ElementState::Pressed) => self.camera_input_state.right = true,
            (KeyCode::KeyD, ElementState::Released) => self.camera_input_state.right = false,
            (KeyCode::Space, ElementState::Pressed) => self.camera_input_state.up = true,
            (KeyCode::Space, ElementState::Released) => self.camera_input_state.up = false,
            (KeyCode::ShiftLeft, ElementState::Pressed) => self.camera_input_state.down = true,
            (KeyCode::ShiftLeft, ElementState::Released) => self.camera_input_state.down = false,
            (KeyCode::ControlLeft, ElementState::Pressed) => self.camera_input_state.fast = true,
            (KeyCode::ControlLeft, ElementState::Released) => self.camera_input_state.fast = false,
            _ => {}
        }
    }

    fn handle_mouse_input(&mut self, button: MouseButton, state: ElementState) {
        match (button, state) {
            (MouseButton::Left, ElementState::Pressed) => self.grab_cursor(true).unwrap(),
            _ => {}
        }
    }

    fn handle_mouse_motion(&mut self, delta_x: f64, delta_y: f64) {
        const PITCH_MAX: f32 = 89.0f32.to_radians();

        if !self.cursor_grabbed {
            return;
        }

        let scene = self.scene.as_mut().unwrap();
        scene.camera.pitch_radians -= (delta_y as f32 * 0.002).clamp(-PITCH_MAX, PITCH_MAX);
        scene.camera.yaw_radians += delta_x as f32 * 0.002;
    }

    fn grab_cursor(&mut self, grab: bool) -> anyhow::Result<()> {
        let window = self.renderer.as_mut().unwrap().window.clone();

        let mode = if grab {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::None
        };

        match window.set_cursor_grab(mode) {
            Ok(_) => {}
            Err(e) => log::error!("failed to grab cursor: {e}"),
        }
        window.set_cursor_visible(!grab);
        self.cursor_grabbed = grab;

        Ok(())
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

        if self.scene.is_none() {
            let renderer = self.renderer.as_ref().unwrap();
            let mut scene = load_scene(&mut self.asset_manager, renderer).unwrap();
            scene.camera.look_at(Vec3::ZERO);
            self.scene = Some(scene);
        }

        self.last_time = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                self.renderer
                    .as_mut()
                    .unwrap()
                    .resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => {
                let current_time = Instant::now();
                let dt = (current_time - self.last_time).as_secs_f64();
                self.last_time = current_time;
                self.update(dt);

                let renderer = self.renderer.as_mut().unwrap();
                let scene = self.scene.as_ref().unwrap();
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
            WindowEvent::Focused(focused) => self.grab_cursor(focused).unwrap(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => self.handle_key_input(code, state, event_loop),
            WindowEvent::MouseInput { state, button, .. } => self.handle_mouse_input(button, state),
            _ => {}
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        match event {
            DeviceEvent::MouseMotion { delta } => self.handle_mouse_motion(delta.0, delta.1),
            _ => {}
        }
    }
}

#[derive(Default)]
struct InputState {
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    fast: bool,
}

fn load_scene(asset_manager: &mut AssetManager, renderer: &Renderer) -> anyhow::Result<Scene> {
    let basic_staff_model = asset_manager.load_model(
        Path::new("Content/Models/Items_Wizard/staff_basic_0.xnb"),
        None,
        renderer,
    )?;

    let plus_staff_model = asset_manager.load_model(
        Path::new("Content/Models/Items_Wizard/staff_plus_0.xnb"),
        None,
        renderer,
    )?;

    let book_model = asset_manager.load_model(
        Path::new("Content/Models/Items_Wizard/magickbook_major.xnb"),
        None,
        renderer,
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
        kind: SceneNodeKind::Mesh(ModelNode {
            model: plus_staff_model,
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
        kind: SceneNodeKind::Mesh(ModelNode {
            model: basic_staff_model.clone(),
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
        kind: SceneNodeKind::Mesh(ModelNode {
            model: basic_staff_model.clone(),
        }),
    });

    let mut rng = rand::rng();
    for _ in 0..15000 {
        let tx: f32 = rng.random_range(-10.0..10.0);
        let ty: f32 = rng.random_range(-10.0..10.0);
        let tz: f32 = rng.random_range(-10.0..10.0);

        let rx: f32 = rng.random_range(0.0..TAU);
        let ry: f32 = rng.random_range(0.0..TAU);
        let rz: f32 = rng.random_range(0.0..TAU);

        scene.root_node.children.push(SceneNode {
            name: "Book".into(),
            transform: Mat4::from_scale_rotation_translation(
                Vec3::ONE,
                Quat::from_euler(glam::EulerRot::XYZ, rx, ry, rz),
                Vec3::new(tx, ty, tz),
            ),
            children: Vec::new(),
            kind: SceneNodeKind::Mesh(ModelNode {
                model: book_model.clone(),
            }),
        });
    }

    Ok(scene)
}
