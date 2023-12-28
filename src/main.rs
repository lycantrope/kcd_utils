use anyhow::{bail, Context, Result};
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use kcd_rename::Mode;
use patternscan::scan;
use std::fs::File;
use std::io::BufWriter;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(name = "kcd_rename")]
#[command(version = "0.1.0")]
#[command(author = "Chung-Kuan Chen <b97b01045@gmail.com>")]
#[command(about = "A CLI Tool to rename KISSEICOMTEC Raw Data (KCD) file", long_about = None)]
struct Cli {
    /// KCD input file
    #[arg(short, long, value_name = "KCD FILE")]
    input: PathBuf,

    /// Output name of KCD file
    #[arg(short, long, value_name = "OUTPUT")]
    output: PathBuf,
    
    /// Method to move the video  (Default: Copy)
    #[arg(short, long, value_enum, value_name ="MODE", default_value_t  = Mode::Copy)]
    mode:Mode
}


fn get_kcrmovie_position<P: AsRef<Path>>(p: P) -> Result<usize> {
    // let mut file = File::open(p)?;
    // let mut buf = Vec::new();
    // file.read_to_end(buf.as_mut())?;
    // let reader = Cursor::new(buf);

    let file = File::open(p)?;
    let reader = BufReader::with_capacity(1024, file);

    let pattern = "4B 43 52 4D 4F 56 49 45";
    // let pattern = "45 49 56 4f 4D 52 43 4B";

    let res = dbg!(scan(reader, pattern));
    match res {
        Ok(pos) if pos.len() == 1 => Ok(pos[0]),
        _ => bail!("Invalid KCD file. (no `KCRMOIVE` or multiple kcd found)"),
    }
}

fn change_kcrmovie_text<P: AsRef<Path>>(input: P, output: P, pos: usize) -> Result<()> {
    let input_p: &Path = input.as_ref();
    let output_p: &Path = output.as_ref();

    let file_stem = output_p
        .file_stem()
        .with_context(|| format!("output is not a valid name: {}", output_p.display()))?
        .to_string_lossy();

    let mut buf: Vec<u8> = Vec::new();
    let mut file = File::open(input_p).with_context(|| "Fail to open input file")?;
    file.read_to_end(&mut buf)?;
    let total: usize = buf.len();
    let mut writer = BufWriter::new(File::create(output_p)?);

    let _ = writer.write(&buf[0..total.min(pos + 16)])?;

    let hdr_tag: String = format!("{}\\{}.hdr", file_stem, file_stem);
    let hdr_buf = hdr_tag.as_bytes();
    if hdr_buf.len() > 256 {
        bail!("output name is too long, please reduce to < 120 charaters");
    }
    let padding = vec![0u8; 256 - hdr_buf.len()];
    writer.write_all(hdr_buf)?;
    writer.write_all(&padding)?;
    writer
        .write(&buf[(pos + 256 + 16).min(total)..total])
        .with_context(|| "Fail to write remaining bytes")?;
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut cmd = Cli::command();
    if !cli.input.is_file() {
        cmd.error(ErrorKind::Io, "Input is not a file").exit()
    }

    let file_stem = cli.input.file_stem().unwrap().to_string_lossy();
    let prefix = cli.output.file_stem().unwrap().to_string_lossy();

    let video_folder: PathBuf = cli.input.with_file_name(file_stem.as_ref());

    let video_hdr: PathBuf = video_folder.join(format!("{}.hdr", file_stem));
    if !video_hdr.exists() {
        println!("Could not find video folder or video.hdr file!");
        println!("Only copy file to the output");
        if !cli.output.exists() {
            let _ = std::fs::copy(cli.input, cli.output)?;
        } else {
            cmd.error(ErrorKind::Io, "Target file existed! Aborted the copy")
                .exit();
        }
        return Ok(());
    }

    let pos = if let Ok(pos) = get_kcrmovie_position(cli.input.as_path()) {
        pos
    } else {
        cmd.error(ErrorKind::Io, "Fail to retrieve position").exit();
    };

    change_kcrmovie_text(&cli.input, &cli.output, pos)?;
    let dst_hdr = video_folder
        .with_file_name(prefix.as_ref())
        .join(format!("{}.hdr", prefix));
    println!("{:?}", dst_hdr.display());
    match kcd_rename::rename_video_hdr(video_hdr, dst_hdr, &prefix, cli.mode) {
        Err(e) => cmd.error(ErrorKind::Io, format!("{:?}", e)).exit(),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_kcrmovie_position() -> Result<()> {
        let pos = get_kcrmovie_position("./short_test.kcd")?;

        Ok(())
    }
}
