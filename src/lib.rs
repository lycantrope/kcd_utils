use anyhow::{bail, Context, Result};
use clap::ValueEnum;
use deku::{bitvec::Msb0, prelude::*};
use rayon::prelude::*;
use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum Mode {
    Copy,
    Move,
}

pub fn modify_raf_file<P: AsRef<Path>>(raf: P, kcd: P) -> Result<()> {
    let raf = raf.as_ref();
    let kcd = kcd.as_ref().canonicalize()?;

    let file = File::open(raf).with_context(|| "Input is not a file")?;
    let mut reader = BufReader::new(file);
    let mut header: [u8; 574] = [0; 574];
    reader
        .read_exact(header.as_mut())
        .with_context(|| "Input is not a vaild RAF file")?;

    if header[0..4] != [82u8, 65, 70, 0] {
        bail!("Input is not a vaild RAF file");
    }

    let kcd_str = kcd.to_string_lossy().replace("/", "\\");
    let kcd_bits = kcd_str.as_bytes();
    let n_bits = kcd_bits.len();
    if n_bits > 256 {
        bail!("Input path of KCD file is too long (>256)");
    }

    let padding = vec![0u8; 256 - n_bits];
    let out_file = File::create(&raf.with_extension("raf.modify"))
        .with_context(|| "Fail to create modified RAF file")?;
    let mut writer = BufWriter::new(out_file);
    writer.write(&header)?;
    writer.write(kcd_bits)?;
    writer.write(padding.as_ref())?;
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

pub fn rename_video_hdr<P: AsRef<Path>>(
    src_hdr: P,
    dst_hdr: P,
    old_prefix: &str,
    new_prefix: &str,
    mode: Mode,
) -> Result<()> {
    // video folder which contains hdr file and videos
    let dst: &Path = dst_hdr.as_ref().parent().unwrap();

    if !dst.is_dir() {
        std::fs::create_dir_all(dst).with_context(|| "Fail to create dst")?;
    }

    let mut input = File::open(&src_hdr).with_context(|| "Fail to open src")?;
    let mut src_hdr_buf: Vec<u8> = Vec::new();
    input.read_to_end(&mut src_hdr_buf)?;

    let (_, mut kcd) =
        KCDVideoHDR::from_bytes((&src_hdr_buf, 0)).with_context(|| "Fail to parse kcd hdr file")?;

    let ext = kcd.get_file_ext().unwrap();
    kcd.rename(new_prefix)?;
    let mut output = File::create(&dst_hdr).with_context(|| "Fail to create dst_p")?;
    let kcd_bytes = kcd.to_bytes()?;
    output.write_all(&kcd_bytes)?;

    let pattern = src_hdr
        .as_ref()
        .parent()
        .map(|p| {
            p.join(format!("{}*.{}", &old_prefix, &ext))
                .to_string_lossy()
                .to_string()
        })
        .unwrap();

    if let Ok(paths) = glob::glob(pattern.as_ref()) {
        let paths: Vec<_> = paths.filter_map(Result::ok).collect();
        let paths: Arc<[PathBuf]> = paths.into();
        let bar: indicatif::ProgressBar = indicatif::ProgressBar::new(paths.len() as u64);
        paths.par_iter().try_for_each(|entry| {
            let oldname = entry.file_stem().map(|c| c.to_string_lossy()).unwrap();
            let newname = oldname.replace(old_prefix, new_prefix);
            bar.inc(1);
            match mode {
                Mode::Move => {
                    std::fs::rename(entry, dst.join(newname)).with_context(|| "Fail to move video")
                }
                Mode::Copy => std::fs::copy(entry, dst.join(newname))
                    .map(|_| ())
                    .with_context(|| "Fail to copy vido"),
            }
        })?
    }

    Ok(())
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct KCDVideoHDR {
    #[deku(bytes_read = "4")]
    header: Vec<u8>,
    #[deku(bytes = "4")]
    count: u32,
    #[deku(
        bytes_read = "292 * count",
        map = "KCDVideoHDR::try_read_video_block",
        writer = "KCDVideoHDR::try_write_video_block(deku::output, &self.data)"
    )]
    data: Vec<VideoBlock>,
}

impl KCDVideoHDR {
    fn try_read_video_block(bytes: &[u8]) -> Result<Vec<VideoBlock>, DekuError> {
        Ok(bytes
            .chunks(292)
            .filter_map(|vals| VideoBlock::try_from(vals).ok())
            .collect())
    }
    fn try_write_video_block(
        output: &mut deku::bitvec::BitVec<u8, Msb0>,
        data: &Vec<VideoBlock>,
    ) -> Result<(), DekuError> {
        for v in data {
            let _ = v.to_bytes().map(|byte| byte.write(output, ()))?;
        }
        Ok(())
    }
    fn get_file_ext(&self) -> Option<String> {
        let mut ext: Vec<_> = self
            .data
            .iter()
            .take(2)
            .filter_map(
                |VideoBlock {
                     _head,
                     filepath,
                     _padding,
                 }| {
                    let p: &Path = filepath.as_ref();
                    p.extension().map(|ext| ext.to_string_lossy())
                },
            )
            .collect();
        ext.pop().map(|v| v.to_string())
    }

    fn rename(&mut self, prefix: &str) -> Result<()> {
        let ext = self.get_file_ext().unwrap_or("avi".to_string());
        self.data.par_iter_mut().enumerate().for_each(|(i, block)| {
            block.filepath = format!("{}\\{}{}.{}", prefix, prefix, i + 1, ext);
        });
        Ok(())
    }
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct VideoBlock {
    #[deku(bytes_read = "16")]
    _head: Vec<u8>,
    #[deku(
        bytes_read = "256",
        map = "VideoBlock::map_to_string",
        writer = "VideoBlock::write_to_string(deku::output, &self.filepath)"
    )]
    filepath: String,
    #[deku(bytes_read = "20")]
    _padding: Vec<u8>,
}

impl VideoBlock {
    fn map_to_string(data: &[u8]) -> Result<String, DekuError> {
        let out: String = data
            .iter()
            .cloned()
            .filter_map(|v| char::try_from(v).ok())
            .collect();
        Ok(out.trim_matches(char::from(0)).to_string())
    }
    fn write_to_string(
        output: &mut deku::bitvec::BitVec<u8, Msb0>,
        field: &str,
    ) -> Result<(), DekuError> {
        let val = field.as_bytes();
        let n = val.len();
        val.write(output, ())?;
        if n < 256 {
            let padding = vec![0u8; 256 - n];
            padding.write(output, ())?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::File,
        io::{Read, Write},
    };

    #[test]
    fn read_hdr() -> anyhow::Result<()> {
        let p = "/Users/chungkuanchen/Projects/kcd_rename/16-1_basal_nos-m4_230925-230927_3487-3488.0001.hdr";
        let mut file = File::open(p)?;
        let mut data: Vec<u8> = Vec::new();
        file.read_to_end(&mut data)?;

        let (_, kcd) = KCDVideoHDR::from_bytes((&data, 0))?;
        println!("{:?}", kcd);
        Ok(())
    }
    #[test]
    fn write_to_hdr() -> anyhow::Result<()> {
        let p = "/Users/chungkuanchen/Projects/kcd_rename/16-1_basal_nos-m4_230925-230927_3487-3488.0001.hdr";
        let mut file = File::open(p)?;
        let mut original: Vec<u8> = Vec::new();
        file.read_to_end(&mut original)?;

        let (_, kcd) = KCDVideoHDR::from_bytes((&original, 0))?;
        let mut out = File::create("./test_out.hdr")?;
        let kcd_bytes = kcd.to_bytes()?;
        out.write_all(&kcd_bytes)?;
        assert_eq!(kcd_bytes, original);
        Ok(())
    }

    #[test]
    fn test_bar() {
        let bar = indicatif::ProgressBar::new(1000);
        for _ in 0..1000 {
            bar.inc(1);
        }
    }
}
