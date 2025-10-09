use std::{
    io::{BufReader, Write},
    sync::Arc,
};

use argh::FromArgs;
use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    platform::wayland::WindowAttributesExtWayland,
    window::{Window, WindowAttributes, WindowId},
};

use crate::xnb::{
    Xnb,
    asset::{
        XnbAsset,
        texture_2d::{self, PixelFormat},
    },
};

mod read_ext;
mod xnb;

/// Aldrheim, a Magicka engine reimplementation.
#[derive(FromArgs, Debug)]
struct Args {
    #[argh(subcommand)]
    command: Command,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
enum Command {
    Run(RunCommand),
    Extract(ExtractCommand),
}

/// run game
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "run")]
struct RunCommand {
    /// path to magicka install directory
    #[argh(positional)]
    path: String,
}

/// extract content from an xnb file
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "extract")]
struct ExtractCommand {
    /// path to xnb file
    #[argh(positional)]
    path: String,
}

fn main() -> anyhow::Result<()> {
    let args: Args = argh::from_env();
    dbg!(&args);

    match args.command {
        Command::Run(args) => {
            run(&args.path)?;
        }
        Command::Extract(args) => {
            extract(&args.path)?;
        }
    }

    Ok(())
}

fn extract(path: &str) -> anyhow::Result<()> {
    let file = std::fs::File::open(path)?;
    let mut reader = BufReader::new(file);

    let xnb = Xnb::read(&mut reader)?;
    dbg!(&xnb.header, xnb.data.len());

    let decompressed = xnb.decompress()?;
    dbg!(decompressed.len());

    let content = Xnb::parse_content_from(&decompressed)?;

    {
        let out_path = format!("{path}.decompressed");
        let mut out_file = std::fs::File::create(out_path)?;
        out_file.write_all(&decompressed)?;
    }

    match content.primary_asset {
        XnbAsset::Texture2D(texture) => {
            // dump png
            let bgra8 = texture.decode(0)?;
            let rgba8 = texture_2d::bgra8_to_rgba8(&bgra8);
            let mut png = Vec::new();
            let encoder = PngEncoder::new(&mut png);
            encoder.write_image(
                &rgba8,
                texture.width,
                texture.height,
                ExtendedColorType::Rgba8,
            )?;

            let out_path = format!("{path}.png");
            let mut out_file = std::fs::File::create(out_path)?;
            out_file.write_all(&png)?;
        }
        XnbAsset::Texture3D(texture) => {
            // dump png slices
            let format = PixelFormat::from_repr(texture.format)
                .ok_or_else(|| anyhow::anyhow!("unknown pixel format: {}", texture.format))?;
            let slice_stride = (texture.width * texture.height * 4) as usize;
            for z in 0..texture.depth {
                let slice_start = slice_stride * z as usize;
                let slice = &texture.mips[0][slice_start..slice_start + slice_stride];
                let bgra8 = texture_2d::decode_pixels(
                    slice,
                    texture.width as usize,
                    texture.height as usize,
                    format,
                )?;
                let rgba8 = texture_2d::bgra8_to_rgba8(&bgra8);
                let mut png = Vec::new();
                let encoder = PngEncoder::new(&mut png);
                encoder.write_image(
                    &rgba8,
                    texture.width,
                    texture.height,
                    ExtendedColorType::Rgba8,
                )?;

                let out_path = format!("{path}-depth{z}.png");
                let mut out_file = std::fs::File::create(out_path)?;
                out_file.write_all(&png)?;
            }
        }
        _ => {}
    }

    Ok(())
}

fn run(path: &str) -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}

struct App {
    graphics: Option<GraphicsContext>,
}

impl App {
    pub fn new() -> Self {
        App { graphics: None }
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
        let window_attributes = WindowAttributes::default()
            .with_title("Aldrheim")
            .with_name("cndofx.Aldrheim", "");
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.graphics = Some(pollster::block_on(GraphicsContext::new(window)).unwrap());
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let graphics = self.graphics.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => graphics.resize(size.width, size.height),
            WindowEvent::RedrawRequested => match graphics.render() {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    let size = graphics.window.inner_size();
                    graphics.resize(size.width, size.height);
                }
                Err(e) => {
                    log::error!("render failed: {e}");
                }
            },
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

struct GraphicsContext {
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    window: Arc<Window>,
}

impl GraphicsContext {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_limits: wgpu::Limits::defaults(),
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        dbg!(&surface_caps);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        dbg!(surface_format);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
        };

        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/shader.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::all(),
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
            multiview: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let vertex_count = VERTICES.len() as u32;

        let ctx = GraphicsContext {
            surface,
            surface_config,
            is_surface_configured: false,
            device,
            queue,
            pipeline,
            vertex_buffer,
            vertex_count,
            window,
        };
        Ok(ctx)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
            self.is_surface_configured = true;
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        let surface_texture = self.surface.get_current_texture()?;
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut command_encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                label: None,
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..self.vertex_count, 0..1);
        }

        self.queue.submit([command_encoder.finish()]);
        surface_texture.present();

        Ok(())
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Debug, Clone, Copy)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.5, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.0],
        color: [0.0, 0.0, 1.0],
    },
];
