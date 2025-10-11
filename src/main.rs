use std::{
    collections::HashMap,
    ffi::OsStr,
    io::{BufReader, Write},
    path::Path,
};

use clap::Parser;
use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};
use winit::event_loop::EventLoop;

use crate::{
    app::App,
    xnb::{
        Xnb,
        asset::{XnbAsset, texture_2d, vertex_decl::VertexDeclaration},
    },
};

mod app;
mod asset_manager;
mod read_ext;
mod renderer;
mod scene;
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
    let mut app = App::new(path)?;
    event_loop.run_app(&mut app)?;

    Ok(())
}
