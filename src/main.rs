use anyhow::Result;
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser, ValueHint};
use kcd_utils::{
    clone_kcd_with_videos, modify_kcrmovie_text, modify_raf_file, modify_video_hdr, move_videos,
    Mode,
};
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
    /// Rename the KCD file and modify its associated HDR tag same as `output.kcd`.
    #[clap(arg_required_else_help = true)]
    Kcd {
        /// Specify the input KCD file.
        #[arg(short, long, value_name = "*.KCD FILE", value_hint = ValueHint::FilePath)]
        input: PathBuf,

        /// Specify the HDR file to establish associations with new KCD file
        #[arg(short, long, value_name = "HDR FILE", value_hint = ValueHint::FilePath)]
        source: PathBuf,

        /// Method to generate the KCD file  (Default: Copy)
        #[arg(short, long, value_enum, value_name ="MODE", default_value_t  = Mode::Copy)]
        mode: Mode,
    },

    /// Modify the Raf file to establish associations with a KCD file.
    #[clap(arg_required_else_help = true)]
    Raf {
        /// Specify the input RAF file
        #[clap(short, long, value_name = "RAF FILE", value_hint = ValueHint::FilePath)]
        input: PathBuf,
        /// Specify the KCD file for association.
        #[clap(short, long, value_name = "KCD FILE", value_hint = ValueHint::FilePath)]
        kcd: PathBuf,
    },

    /// Output HDR files named as `prefix.hdr` in the same folder.
    #[clap(arg_required_else_help = true)]
    Hdr {
        /// Specify the input HDR file.
        #[arg(short, long, value_name = "HDR FILE", value_hint = ValueHint::FilePath)]
        input: PathBuf,

        /// Specify the text for labeling new HDR file.
        #[arg(short, long)]
        label: String,
    },

    /// Move or copy videos based on source and target HDR files.
    #[clap(arg_required_else_help = true)]
    Video {
        /// Specify a HDR file, which should be placed in the video folder
        #[arg(short, long, value_name = "SOURCE HDR FILE", value_hint = ValueHint::FilePath)]
        src: PathBuf,

        /// Target HDR file, which should be placed in a folder named as HDR file
        #[arg(short, long, value_name = "TARGET HDR FILE", value_hint = ValueHint::FilePath)]
        dst: PathBuf,

        /// Method to move the video  (Default: Copy)
        #[arg(short, long, value_enum, value_name ="MODE", default_value_t  = Mode::Copy)]
        mode: Mode,
    },
    /// Clone existing KCD and videos into new labeled KCD file
    #[clap(arg_required_else_help = true)]
    Clone {
        /// Specify the input KCD file.
        #[arg(short, long, value_name = "KCD FILE", value_hint = ValueHint::FilePath)]
        input: PathBuf,

        /// Specify the label for cloned KCD, HDR and video files.
        #[arg(short, long)]
        label: String,

        /// Method to generate the KCD and videos file  (Default: Copy)
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
            source,
            mode,
        } => modify_kcrmovie_text(input, source, mode).map(|_|()),
        Utils::Raf { input, kcd } => modify_raf_file(input, kcd),
        Utils::Hdr {
            input,
            label: prefix,
        } => modify_video_hdr(input, &prefix).map(|_| ()),
        Utils::Video { src, dst, mode } => move_videos(src, dst, mode),
        Utils::Clone { input, label, mode } => clone_kcd_with_videos(input, label, mode),
    };

    match res {
        Ok(_) => res,
        Err(e) => cmd.error(ErrorKind::Io, format!("{:?}", e)).exit(),
    }
}
