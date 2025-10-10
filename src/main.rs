use std::{
    collections::HashMap,
    ffi::OsStr,
    io::{BufReader, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use clap::Parser;
use glam::{Mat4, Vec3};
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
        XnbAsset, texture_2d,
        vertex_decl::{ElementUsage, VertexDeclaration},
    },
};

mod read_ext;
mod xnb;

#[derive(clap::Parser)]
struct Args {
    #[command(subcommand)]
    subcommand: Subcommands,
}

#[derive(clap::Subcommand, Clone)]
enum Subcommands {
    Run(RunCommand),
    Extract(ExtractCommand),
    Dev(DevCommand),
}

/// Run the game
#[derive(clap::Args, Clone)]
struct RunCommand {
    /// path to magicka install directory
    path: String,
}

/// Extract content from an XNB file
#[derive(clap::Args, Clone)]
struct ExtractCommand {
    /// path to xnb file
    path: String,
}

/// Development utilities
#[derive(clap::Args, Clone)]
struct DevCommand {
    #[command(subcommand)]
    subcommand: DevSubcommands,
}

#[derive(clap::Subcommand, Clone)]
enum DevSubcommands {
    DedupPipelines(DedupPipelinesCommand),
}

/// Parse all models in a directory and find all unique shader and vertex layout combinations
#[derive(clap::Args, Clone)]
struct DedupPipelinesCommand {
    /// path to search directory
    path: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.subcommand {
        Subcommands::Run(args) => {
            run(&args.path)?;
        }
        Subcommands::Extract(args) => {
            extract(&args.path)?;
        }
        Subcommands::Dev(args) => match args.subcommand {
            DevSubcommands::DedupPipelines(args) => {
                dedup_pipelines(&args.path)?;
            }
        },
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
            let slice_stride = (texture.width * texture.height * 4) as usize;
            for z in 0..texture.depth {
                let slice_start = slice_stride * z as usize;
                let slice = &texture.mips[0][slice_start..slice_start + slice_stride];
                let bgra8 = texture_2d::decode_pixels(
                    slice,
                    texture.width as usize,
                    texture.height as usize,
                    texture.format,
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

fn dedup_pipelines(path: &str) -> anyhow::Result<()> {
    let mut xnb_paths = Vec::new();

    let entries = std::fs::read_dir(path)?;
    for entry in entries {
        let entry = match entry {
            Ok(v) => v,
            Err(e) => {
                eprintln!("error: {e}");
                continue;
            }
        };

        let path = entry.path();

        if !entry.file_type()?.is_file() {
            println!("skipping non file path: {}", path.display());
            continue;
        }

        if path.extension() != Some(&OsStr::new("xnb")) {
            println!("skipping non xnb file: {}", path.display());
            continue;
        }

        xnb_paths.push(path);
    }

    // number of mesh parts using a unique vertex declaration and effect
    let mut map: HashMap<DedupedPipelineInfo, u32> = HashMap::new();

    let mut num_processed = 0;
    let mut num_errors = 0;
    for path in &xnb_paths {
        num_processed += 1;
        match dedup_pipelines_handle_file(path, &mut map) {
            Ok(_) => {}
            Err(e) => {
                num_errors += 1;
                eprintln!("error on {}: {}", path.file_name().unwrap().display(), e);
                continue;
            }
        }
    }

    let mut kvs = map.iter().collect::<Vec<_>>();
    kvs.sort_unstable_by_key(|kv| *kv.1);

    for (pipeline, count) in &kvs {
        print!(
            "count: {:>5}, effect: {}, vertex decl: ",
            count, pipeline.effect
        );
        for el in &pipeline.vertex_declaration.elements {
            print!("{} ", el.debug_string());
        }
        println!();
    }

    println!(
        "processed {} files with {} errors",
        num_processed, num_errors
    );
    println!("{} unique pipelines found", kvs.len());

    Ok(())
}

fn dedup_pipelines_handle_file(
    path: impl AsRef<Path>,
    map: &mut HashMap<DedupedPipelineInfo, u32>,
) -> anyhow::Result<()> {
    let path = path.as_ref();
    let file = std::fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let xnb = Xnb::read(&mut reader)?;
    let content = xnb.parse_content()?;

    let XnbAsset::Model(model) = content.primary_asset else {
        return Ok(());
    };

    for mesh in &model.meshes {
        for part in &mesh.parts {
            let key = DedupedPipelineInfo {
                vertex_declaration: model.vertex_decls[part.vertex_decl_index as usize].clone(),
                effect: content.shared_assets[(part.shared_content_material_index - 1) as usize]
                    .as_ref()
                    .into(),
            };
            map.entry(key).and_modify(|count| *count += 1).or_insert(1);
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct DedupedPipelineInfo {
    vertex_declaration: VertexDeclaration,
    effect: String,
}

fn run(path: &str) -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App::new(path);
    event_loop.run_app(&mut app)?;

    Ok(())
}

struct App {
    magicka_path: PathBuf,
    start_time: Instant,
    graphics: Option<GraphicsContext>,
}

impl App {
    pub fn new(magicka_path: impl Into<PathBuf>) -> Self {
        App {
            magicka_path: magicka_path.into(),
            start_time: Instant::now(),
            graphics: None,
        }
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
        self.graphics =
            Some(pollster::block_on(GraphicsContext::new(window, &self.magicka_path)).unwrap());
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let graphics = self.graphics.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                graphics.resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => {
                let time = self.start_time.elapsed().as_secs_f32();
                match graphics.render(time) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = graphics.window.inner_size();
                        graphics.resize(size.width, size.height);
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

struct GraphicsContext {
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_buffer_bind_group: wgpu::BindGroup,
    vertex_layout_uniform_buffer: wgpu::Buffer,
    vertex_layout_uniform_bind_group: wgpu::BindGroup,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    start_index: u32,
    base_vertex: u32,
    camera_uniform_buffer: wgpu::Buffer,
    camera_uniform_bind_group: wgpu::BindGroup,
    depth_texture: wgpu::Texture,
    // texture_bind_group: wgpu::BindGroup,
    window: Arc<Window>,
}

impl GraphicsContext {
    pub async fn new(window: Arc<Window>, magicka_path: &Path) -> anyhow::Result<Self> {
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
                required_features: wgpu::Features::TEXTURE_COMPRESSION_BC,
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

        //

        let mut path = magicka_path.to_owned();
        // path.push("Content/Models/Items_Wizard/staff_basic_0.xnb");
        path.push("Content/Models/Items_Wizard/staff_plus_0.xnb");
        // path.push("Content/Models/Items_Wizard/staff_of_deflection_0.xnb");
        // path.push("Content/Models/Items_Wizard/knife_of_counterstriking_1.xnb");
        // path.push("Content/Models/Items_Wizard/m16_1.xnb");
        let file = std::fs::File::open(&path)?;
        let mut reader = BufReader::new(file);
        let xnb = Xnb::read(&mut reader)?;
        let content = xnb.parse_content()?;
        let XnbAsset::Model(xnb_model) = content.primary_asset else {
            anyhow::bail!("expected model at path {}", path.display());
        };
        let xnb_mesh = &xnb_model.meshes[0];
        let xnb_part = &xnb_mesh.parts[0];
        let xnb_vertex_decl = &xnb_model.vertex_decls[xnb_part.vertex_decl_index as usize];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Loaded XNB Vertex Buffer"),
            contents: &xnb_mesh.vertex_buffer.data,
            usage: wgpu::BufferUsages::STORAGE,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Loaded XNB Index Buffer"),
            contents: &xnb_mesh.index_buffer.data,
            usage: wgpu::BufferUsages::INDEX,
        });

        let index_count = xnb_part.primitive_count * 3;
        let start_index = xnb_part.start_index;
        let base_vertex = xnb_part.base_vertex;

        let vertex_buffer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Vertex Buffer Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let vertex_buffer_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Vertex Layout Uniform Bind Group"),
            layout: &vertex_buffer_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(vertex_buffer.as_entire_buffer_binding()),
            }],
        });

        let vertex_layout_uniform = VertexLayoutUniform::from_xnb_decl(xnb_vertex_decl)?;
        let vertex_layout_uniform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Layout Uniform Buffer"),
                contents: bytemuck::cast_slice(&[vertex_layout_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let vertex_layout_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Vertex Layout Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let vertex_layout_uniform_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Vertex Layout Uniform Bind Group"),
                layout: &vertex_layout_uniform_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(
                        vertex_layout_uniform_buffer.as_entire_buffer_binding(),
                    ),
                }],
            });

        // let texture = {
        //     let mut path = magicka_path.to_owned();
        //     path.push("Content/UI/Menu/CampaignMap.xnb");
        //     let file = std::fs::File::open(&path)?;
        //     let mut reader = BufReader::new(file);
        //     let xnb = Xnb::read(&mut reader)?;
        //     let content = xnb.parse_content()?;
        //     let XnbAsset::Texture2D(xnb_texture) = content.primary_asset else {
        //         anyhow::bail!("expected texture 2d at path {}", path.display());
        //     };

        //     let texture_format = xnb_texture.format.to_wgpu();
        //     dbg!(texture_format);

        //     let texture_size = wgpu::Extent3d {
        //         width: xnb_texture.width,
        //         height: xnb_texture.height,
        //         depth_or_array_layers: 1,
        //     };

        //     let texture = device.create_texture(&wgpu::TextureDescriptor {
        //         label: Some("Campaign Map"),
        //         usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        //         size: texture_size,
        //         format: texture_format,
        //         dimension: wgpu::TextureDimension::D2,
        //         mip_level_count: xnb_texture.mips.len() as u32,
        //         sample_count: 1,
        //         view_formats: &[],
        //     });

        //     for (i, mip) in xnb_texture.mips.iter().enumerate() {
        //         println!(
        //             "mip {} size: {}, bytes_per_row: {}, rows_per_image: {}",
        //             i,
        //             mip.len(),
        //             xnb_texture.bytes_per_row(i)?,
        //             xnb_texture.rows_per_image(i)?,
        //         );
        //     }

        //     for (i, mip) in xnb_texture.mips.iter().enumerate() {
        //         // TODO: is this the correct thing to do here?
        //         // wgpu validation doesnt like copying 2x2 pixel mips with 4x4 block size
        //         let mip_size = wgpu::Extent3d {
        //             width: (xnb_texture.width / 2u32.pow(i as u32))
        //                 .max(xnb_texture.format.block_dim()),
        //             height: (xnb_texture.height / 2u32.pow(i as u32))
        //                 .max(xnb_texture.format.block_dim()),
        //             depth_or_array_layers: 1,
        //         };
        //         dbg!(i, mip_size);

        //         queue.write_texture(
        //             wgpu::TexelCopyTextureInfo {
        //                 texture: &texture,
        //                 mip_level: i as u32,
        //                 origin: wgpu::Origin3d::ZERO,
        //                 aspect: wgpu::TextureAspect::All,
        //             },
        //             mip,
        //             wgpu::TexelCopyBufferLayout {
        //                 offset: 0,
        //                 bytes_per_row: Some(xnb_texture.bytes_per_row(i)?),
        //                 rows_per_image: Some(xnb_texture.rows_per_image(i)?),
        //             },
        //             mip_size,
        //         );
        //     }

        //     texture
        // };

        // let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        // let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        //     label: None,
        //     address_mode_u: wgpu::AddressMode::ClampToEdge,
        //     address_mode_v: wgpu::AddressMode::ClampToEdge,
        //     address_mode_w: wgpu::AddressMode::ClampToEdge,
        //     mag_filter: wgpu::FilterMode::Linear,
        //     min_filter: wgpu::FilterMode::Linear,
        //     mipmap_filter: wgpu::FilterMode::Linear,
        //     ..Default::default()
        // });

        // let texture_bind_group_layout =
        //     device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //         label: Some("Texture Bind Group Layout"),
        //         entries: &[
        //             wgpu::BindGroupLayoutEntry {
        //                 binding: 0,
        //                 visibility: wgpu::ShaderStages::FRAGMENT,
        //                 ty: wgpu::BindingType::Texture {
        //                     sample_type: wgpu::TextureSampleType::Float { filterable: true },
        //                     view_dimension: wgpu::TextureViewDimension::D2,
        //                     multisampled: false,
        //                 },
        //                 count: None,
        //             },
        //             wgpu::BindGroupLayoutEntry {
        //                 binding: 1,
        //                 visibility: wgpu::ShaderStages::FRAGMENT,
        //                 ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        //                 count: None,
        //             },
        //         ],
        //     });
        // let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     label: Some("Texture Bind Group"),
        //     layout: &texture_bind_group_layout,
        //     entries: &[
        //         wgpu::BindGroupEntry {
        //             binding: 0,
        //             resource: wgpu::BindingResource::TextureView(&texture_view),
        //         },
        //         wgpu::BindGroupEntry {
        //             binding: 1,
        //             resource: wgpu::BindingResource::Sampler(&texture_sampler),
        //         },
        //     ],
        // });

        let camera_uniform = CameraUniform {
            view: Mat4::IDENTITY,
            projection: Mat4::IDENTITY,
        };
        let camera_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let camera_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Uniform Bind Group"),
            layout: &camera_uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(
                    camera_uniform_buffer.as_entire_buffer_binding(),
                ),
            }],
        });

        let shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/render_deferred_effect.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                // &texture_bind_group_layout,
                &vertex_buffer_bind_group_layout,
                &vertex_layout_uniform_bind_group_layout,
                &camera_uniform_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
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
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let depth_texture = create_depth_texture(&device, &surface_config);

        let ctx = GraphicsContext {
            surface,
            surface_config,
            is_surface_configured: false,
            device,
            queue,
            pipeline,
            vertex_buffer,
            vertex_buffer_bind_group,
            vertex_layout_uniform_buffer,
            vertex_layout_uniform_bind_group,
            index_buffer,
            index_count,
            start_index,
            base_vertex,
            camera_uniform_buffer,
            camera_uniform_bind_group,
            depth_texture,
            // texture_bind_group,
            window,
        };
        Ok(ctx)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
        self.is_surface_configured = true;

        self.depth_texture = create_depth_texture(&self.device, &self.surface_config);
    }

    pub fn render(&mut self, time: f32) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        let window_size = self.window.inner_size();
        let projection = Mat4::perspective_lh(
            75.0_f32.to_radians(),
            (window_size.width as f32) / (window_size.height as f32),
            1.0,
            10000.0,
        );

        let radius = 4.0;
        let x = time.sin() * radius;
        let z = time.cos() * radius;
        let view = Mat4::look_at_lh(Vec3::new(x, 0.0, z), Vec3::ZERO, Vec3::Y);

        let camera_uniform = CameraUniform { view, projection };
        self.queue.write_buffer(
            &self.camera_uniform_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );

        let surface_texture = self.surface.get_current_texture()?;
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture_view = self
            .depth_texture
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                label: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            // render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
            render_pass.set_bind_group(0, &self.vertex_buffer_bind_group, &[]);
            render_pass.set_bind_group(1, &self.vertex_layout_uniform_bind_group, &[]);
            render_pass.set_bind_group(2, &self.camera_uniform_bind_group, &[]);
            // render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(
                self.start_index..self.start_index + self.index_count,
                self.base_vertex as i32,
                0..1,
            );
        }

        self.queue.submit([command_encoder.finish()]);

        self.window.pre_present_notify();
        surface_texture.present();

        Ok(())
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Debug, Clone, Copy)]
struct CameraUniform {
    view: Mat4,
    projection: Mat4,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Debug, Clone, Copy)]
struct VertexLayoutUniform {
    stride: u32,
    position: i32,
    normal: i32,
    color: i32,
    tex_coord_0: i32,
    tex_coord_1: i32,
}

impl VertexLayoutUniform {
    fn from_xnb_decl(decl: &VertexDeclaration) -> anyhow::Result<Self> {
        // TODO: this is terrible

        let mut position = -1;
        let mut normal = -1;
        let mut color = -1;
        let mut tex_coord_0 = -1;
        let mut tex_coord_1 = -1;
        for el in &decl.elements {
            match el.usage {
                ElementUsage::Position => {
                    if position < 0 {
                        position = el.offset as i32;
                    } else {
                        anyhow::bail!("duplicate 'position' elements in vertex declaration");
                    }
                }
                ElementUsage::Normal => {
                    if normal < 0 {
                        normal = el.offset as i32;
                    } else {
                        anyhow::bail!("duplicate 'normal' elements in vertex declaration");
                    }
                }
                ElementUsage::TextureCoordinate => {
                    if tex_coord_0 < 0 {
                        tex_coord_0 = el.offset as i32;
                    } else if tex_coord_1 < 0 {
                        tex_coord_1 = el.offset as i32;
                    } else {
                        anyhow::bail!("duplicate 'tex_coord' elements in vertex declaration");
                    }
                }
                ElementUsage::Color => {
                    if color < 0 {
                        color = el.offset as i32;
                    } else {
                        anyhow::bail!("duplicate 'color' elements in vertex declaration");
                    }
                }
                _ => anyhow::bail!("unsupported vertex usage '{:?}'", el.usage),
            }
        }

        if position == -1 {
            anyhow::bail!("missing vertex element 'position'");
        }

        if normal == -1 {
            anyhow::bail!("missing vertex element 'normal'");
        }

        if tex_coord_0 == -1 {
            anyhow::bail!("missing vertex element 'tex_coord'");
        }

        Ok(VertexLayoutUniform {
            stride: decl.stride() as u32,
            position,
            normal,
            color,
            tex_coord_0,
            tex_coord_1,
        })
    }
}

fn create_depth_texture(
    device: &wgpu::Device,
    surface_config: &wgpu::SurfaceConfiguration,
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Depth Buffer"),
        size: wgpu::Extent3d {
            width: surface_config.width.max(1),
            height: surface_config.height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    // let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
    //     label: Some("Depth Sampler"),
    //     address_mode_u: wgpu::AddressMode::ClampToEdge,
    //     address_mode_v: wgpu::AddressMode::ClampToEdge,
    //     address_mode_w: wgpu::AddressMode::ClampToEdge,
    //     mag_filter: wgpu::FilterMode::Linear,
    //     min_filter: wgpu::FilterMode::Linear,
    //     mipmap_filter: wgpu::FilterMode::Nearest,
    //     compare: Some(wgpu::CompareFunction::LessEqual),
    //     lod_min_clamp: 0.0,
    //     lod_max_clamp: 100.0,
    //     ..Default::default()
    // });

    texture
}
