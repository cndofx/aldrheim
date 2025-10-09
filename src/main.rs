use std::io::{BufReader, Write};

use argh::FromArgs;
use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};

use crate::xnb::{
    Xnb,
    asset::{
        XnbAsset,
        texture_2d::{self, PixelFormat, Texture2D},
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
            todo!("run command");
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
