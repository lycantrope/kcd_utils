use anyhow::{bail, Result};
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use kcd_utils::{modify_kcrmovie_text, modify_raf_file, modify_video_hdr, Mode};
use std::path::PathBuf;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
    about = env!("CARGO_PKG_DESCRIPTION"),
    arg_required_else_help = true,
)]
struct Cli {
    #[clap(subcommand)]
    subcommands: Utils,
}

#[derive(Debug, clap::Subcommand)]
enum Utils {
    /// Modify the Raf file to make association for a KCD file
    #[clap(arg_required_else_help = true)]
    Raf {
        /// Specify the input RAF file
        #[clap(short, long, value_name = "RAF FILE")]
        input: PathBuf,
        /// Specify the KCD file for association.
        #[clap(short, long, value_name = "KCD FILE")]
        kcd: PathBuf,
    },

    /// Rename the KCD file and modify its assoicated HDR files.
    #[clap(arg_required_else_help = true)]
    Kcd {
        /// KCD input file
        #[arg(short, long, value_name = "KCD FILE")]
        input: PathBuf,

        /// Output name of KCD file
        #[arg(short, long, value_name = "HDR FILE")]
        output: PathBuf,

        /// Method to move the video  (Default: Copy)
        #[arg(short, long, value_enum, value_name ="MODE", default_value_t  = Mode::Copy)]
        mode: Mode,
    },
    /// Output HDR file will be in same folder named as `prefix.hdr``.
    #[clap(arg_required_else_help = true)]
    Hdr {
        /// HDR input file
        #[arg(short, long, value_name = "HDR FILE")]
        input: PathBuf,

        /// Prefix for new HDR file as well as video prefix
        #[arg(short, long)]
        prefix: String,
    },

    /// Move video based on hdr files.
    #[clap(arg_required_else_help = true)]
    Video {
        #[arg(short, long, value_name = "SOURCE HDR FILE")]
        src: PathBuf,

        #[arg(short, long, value_name = "TARGET HDR FILE")]
        dst: PathBuf,

        #[arg(short, long, value_enum, value_name ="MODE", default_value_t  = Mode::Copy)]
        mode: Mode,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut cmd = Cli::command();
    let res = match cli.subcommands {
        Utils::Kcd {
            input,
            output,
            mode,
        } => modify_kcrmovie_text(input, output, mode),
        Utils::Raf { input, kcd } => modify_raf_file(input, kcd),
        Utils::Hdr { input, prefix } => modify_video_hdr(input, &prefix),
        Utils::Video { src, dst, mode } => {
            let _ = (src, dst, mode);
            bail!("This function is not implemented")
        }
    };

    match res {
        Ok(_) => res,
        Err(e) => cmd.error(ErrorKind::Io, format!("{:?}", e)).exit(),
    }
}
