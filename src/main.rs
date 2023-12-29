use anyhow::{bail, Result};
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use kcd_utils::{modify_kcrmovie_text, modify_raf_file, modify_video_hdr, move_videos, Mode};
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
        #[arg(short, long, value_name = "*.KCD FILE")]
        input: PathBuf,

        /// Specify the HDR file to establish associations with new KCD file
        #[arg(short, long, value_name = "HDR FILE")]
        source: PathBuf,

        /// Method to generate the KCD file  (Default: Copy)
        #[arg(short, long, value_enum, value_name ="MODE", default_value_t  = Mode::Copy)]
        mode: Mode,
    },

    /// Modify the Raf file to establish associations with a KCD file.
    #[clap(arg_required_else_help = true)]
    Raf {
        /// Specify the input RAF file
        #[clap(short, long, value_name = "RAF FILE")]
        input: PathBuf,
        /// Specify the KCD file for association.
        #[clap(short, long, value_name = "KCD FILE")]
        kcd: PathBuf,
    },

    /// Output HDR files named as `prefix.hdr` in the same folder.
    #[clap(arg_required_else_help = true)]
    Hdr {
        /// Specify the input HDR file.
        #[arg(short, long, value_name = "HDR FILE")]
        input: PathBuf,

        /// Specify the text for labeling new HDR file.
        #[arg(short, long)]
        label: String,
    },

    /// Move or copy videos based on source and target HDR files.
    #[clap(arg_required_else_help = true)]
    Video {
        /// Specify a HDR file, which should be placed in the video folder
        #[arg(short, long, value_name = "SOURCE HDR FILE")]
        src: PathBuf,

        /// Target HDR file, which should be placed in a folder named as HDR file
        #[arg(short, long, value_name = "TARGET HDR FILE")]
        dst: PathBuf,

        /// Method to move the video  (Default: Copy)
        #[arg(short, long, value_enum, value_name ="MODE", default_value_t  = Mode::Copy)]
        mode: Mode,
    },
    /// Clone existing KCD and videos into new labeled KCD file
    #[clap(arg_required_else_help = true)]
    Clone {
        /// Specify the input KCD file.
        #[arg(short, long, value_name = "KCD FILE")]
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
        } => modify_kcrmovie_text(input, source, mode),
        Utils::Raf { input, kcd } => modify_raf_file(input, kcd),
        Utils::Hdr {
            input,
            label: prefix,
        } => modify_video_hdr(input, &prefix).map(|_| ()),
        Utils::Video { src, dst, mode } => move_videos(src, dst, mode),
        Utils::Clone { input, label, mode } => {
            let kcd = input;
            if !kcd.is_file() {
                bail!("KCD was not a file. Abort the process")
            }

            let old_tag = kcd.file_stem().map(|x| x.to_string_lossy()).unwrap();
            let hdr = kcd
                .with_file_name(old_tag.as_ref())
                .join(format!("{}.hdr", &old_tag));

            if !hdr.is_file() {
                bail!("HDR was not existed. Abort the copy process")
            }

            let cwd = kcd.parent().unwrap();
            let new_video_folder = cwd.join(&label);
            let _ = std::fs::create_dir(&new_video_folder);

            let new_hdr = modify_video_hdr(&hdr, &label)?;

            std::fs::rename(
                &new_hdr,
                new_video_folder.join(new_hdr.file_name().unwrap()),
            )?;

            // Always use Copy to avoid trouble
            modify_kcrmovie_text(&kcd, &new_hdr, Mode::Copy)?;

            move_videos(&hdr, &new_hdr, mode)
        }
    };

    match res {
        Ok(_) => res,
        Err(e) => cmd.error(ErrorKind::Io, format!("{:?}", e)).exit(),
    }
}
