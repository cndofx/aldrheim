use std::{
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
    time::Instant,
};

use glam::Vec3;
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
    renderer::{RenderContext, Renderer, camera::Camera},
    scene::Scene,
};

pub struct App {
    magicka_path: PathBuf,

    asset_manager: Option<AssetManager>,
    renderer: Option<Renderer>,
    scene: Option<Scene>,

    last_time: Instant,
    camera_input_state: InputState,
    camera_speed: f32,
    cursor_grabbed: bool,
}

impl App {
    pub fn new(magicka_path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let app = App {
            magicka_path: magicka_path.into(),

            asset_manager: None,
            renderer: None,
            scene: None,

            last_time: Instant::now(),
            camera_input_state: InputState::default(),
            camera_speed: 5.0,
            cursor_grabbed: false,
        };
        Ok(app)
    }

    fn update(&mut self, dt: f32) {
        let scene = self.scene.as_mut().unwrap();

        scene.update(dt);

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

            let (forward, right, _) = scene.camera.forward_right_up();

            let mut amount = self.camera_speed * (dt as f32);
            if self.camera_input_state.fast {
                amount *= 4.0;
            }

            scene.camera.position += forward * camera_move_direction.z * amount;
            scene.camera.position += right * camera_move_direction.x * amount;
            scene.camera.position += Camera::UP * camera_move_direction.y * amount;
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
        scene.camera.pitch_radians =
            (scene.camera.pitch_radians - delta_y as f32 * 0.002).clamp(-PITCH_MAX, PITCH_MAX);
        scene.camera.yaw_radians -= delta_x as f32 * 0.002;
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

        // TODO: handle these errors properly instead of unwrapping
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        let (render_context, surface, surface_config) =
            pollster::block_on(RenderContext::new(window.clone())).unwrap();
        let render_context = Rc::new(render_context);
        let mut asset_manager =
            AssetManager::new(&self.magicka_path, render_context.clone()).unwrap();
        let renderer = Renderer::new(
            render_context,
            window,
            surface,
            surface_config,
            &mut asset_manager,
        )
        .unwrap();

        self.renderer = Some(renderer);
        self.asset_manager = Some(asset_manager);

        if self.scene.is_none() {
            let asset_manager = self.asset_manager.as_mut().unwrap();
            let scene = load_scene(asset_manager).unwrap();
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
                let dt = (current_time - self.last_time).as_secs_f32();
                self.last_time = current_time;
                self.update(dt);

                let renderer = self.renderer.as_mut().unwrap();
                let scene = self.scene.as_mut().unwrap();
                let draws = scene.render(&renderer.context);
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

// TODO: NOT YET LOADING LEVELS:
// - ch_volcano_hideout.xnb (needs LavaEffect)

fn load_scene(asset_manager: &mut AssetManager) -> anyhow::Result<Scene> {
    // let level_path = Path::new("Content/Levels/WizardCastle/wc_s4.xml");
    let level_path = Path::new("Content/Levels/Challenges/chs_havindr_arena.xml");

    let scene = Scene::load_level(level_path, None, asset_manager)?;

    Ok(scene)
}
