use std::io::{BufReader, Write};

use argh::FromArgs;

use crate::xnb::Xnb;

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
    dbg!(xnb.header(), xnb.data().len());

    let decompressed = xnb.decompress()?;
    dbg!(decompressed.len());

    // dump decompressed
    {
        let out_path = format!("{path}.decompressed");
        let mut out_file = std::fs::File::create(out_path)?;
        out_file.write_all(&decompressed)?;
    }

    todo!();
}
