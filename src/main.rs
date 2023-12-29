use anyhow::{bail, Context, Result};
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use kcd_utils::{modify_raf_file, rename_video_hdr, Mode};
use patternscan::scan;
use std::fs::File;
use std::io::{BufRead as _, BufWriter};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(
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
        #[arg(short, long, value_name = "OUTPUT")]
        output: PathBuf,

        /// Method to move the video  (Default: Copy)
        #[arg(short, long, value_enum, value_name ="MODE", default_value_t  = Mode::Copy)]
        mode: Mode,
    },
}
fn get_kcrmovie_position<P: AsRef<Path>>(p: P) -> Result<usize> {
    // let mut file = File::open(p)?;
    // let mut buf = Vec::new();
    // file.read_to_end(buf.as_mut())?;
    // let reader = Cursor::new(buf);

    let file = File::open(p)?;
    let mut reader = BufReader::new(file);

    let pattern = "4B 43 52 4D 4F 56 49 45";
    // let pattern = "45 49 56 4f 4D 52 43 4B";
    let mut count = 0;
    let mut res = 0;
    while let Ok(buf) = reader.fill_buf() {
        let n = buf.len();
        if n == 0 {
            break;
        }
        if let Ok(pos) = scan(std::io::Cursor::new(buf), pattern) {
            if pos.len() > 1 {
                reader.consume(n);
                bail!("Invalid KCD file. (no `KCRMOIVE` or multiple kcd found)");
            } else if pos.len() == 1 {
                res = count + pos[0];
                break;
            } else {
                count += n;
                reader.consume(n);
            }
        }
    }
    Ok(res)
}

fn change_kcrmovie_text<P: AsRef<Path>>(input: P, output: P, pos: usize) -> Result<()> {
    let input_p: &Path = input.as_ref();
    let output_p: &Path = output.as_ref();

    let file_stem = output_p
        .file_stem()
        .with_context(|| format!("output is not a valid name: {}", output_p.display()))?
        .to_string_lossy();

    let hdr_tag: String = format!("{}\\{}.hdr", file_stem, file_stem);
    let hdr_buf = hdr_tag.as_bytes();
    if hdr_buf.len() > 256 {
        bail!("output name is too long, please reduce to < 120 charaters");
    }

    let mut header = vec![0u8; pos + 16];

    let in_file = File::open(input_p).with_context(|| "Fail to open input file")?;
    let mut reader = BufReader::new(in_file);

    reader.read(header.as_mut())?;

    let mut writer = BufWriter::new(File::create(output_p)?);

    writer.write(&header)?;

    let padding = vec![0u8; 256 - hdr_buf.len()];

    writer.write(hdr_buf)?;
    writer.write(&padding)?;

    reader.seek_relative(256)?;
    while let Ok(buf) = reader.fill_buf() {
        if buf.is_empty() {
            break;
        }
        let n = buf.len();
        let res = writer.write(buf);
        reader.consume(n);
        res.with_context(|| "Fail to write to file")?;
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut cmd = Cli::command();
    match cli.subcommands {
        Utils::Kcd {
            input,
            output,
            mode,
        } => {
            if !input.is_file() {
                cmd.error(ErrorKind::Io, "Input is not a file").exit()
            }

            let old_prefix = input.file_stem().unwrap().to_string_lossy();
            let new_prefix = output.file_stem().unwrap().to_string_lossy();

            let video_folder: PathBuf = input.with_file_name(old_prefix.as_ref());

            let src_hdr: PathBuf = video_folder.join(format!("{}.hdr", old_prefix));
            if !src_hdr.exists() {
                println!("Could not find video folder or video.hdr file!");
                println!("Only copy file to the output");
                if !output.exists() {
                    let _ = std::fs::copy(&input, &output)?;
                } else {
                    cmd.error(ErrorKind::Io, "Target file existed! Aborted the copy")
                        .exit();
                }
                return Ok(());
            }

            let pos = if let Ok(pos) = get_kcrmovie_position(&input) {
                pos
            } else {
                cmd.error(ErrorKind::Io, "Fail to retrieve position").exit();
            };

            change_kcrmovie_text(&input, &output, pos)?;
            let dst_hdr = video_folder
                .with_file_name(new_prefix.as_ref())
                .join(format!("{}.hdr", new_prefix));

            match rename_video_hdr(src_hdr, dst_hdr, &old_prefix, &new_prefix, mode) {
                Err(e) => cmd.error(ErrorKind::Io, format!("{:?}", e)).exit(),
                _ => (),
            };
        }
        Utils::Raf { input, kcd } => {
            modify_raf_file(&input, &kcd)?;
            println!(
                "New RAF file was save as:{}",
                &input.with_extension("raf.modify").display()
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_kcrmovie_position() -> Result<()> {
        let pos = get_kcrmovie_position("./test.0001.kcd")?;
        println!("{pos}");
        Ok(())
    }
}
