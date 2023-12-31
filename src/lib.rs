use anyhow::{bail, Context, Result};
use clap::ValueEnum;
use deku::{bitvec::Msb0, prelude::*};
use indicatif::{ProgressIterator as _, ProgressStyle};
use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum Mode {
    Copy,
    Move,
}
impl AsRef<str> for Mode {
    #[inline]
    fn as_ref(&self) -> &str {
        match self {
            Self::Copy => "Copy",
            Self::Move => "Move",
        }
    }
}
pub fn move_videos<P: AsRef<Path>>(src: P, dst: P, mode: Mode) -> Result<()> {
    let src_p = src.as_ref();
    let dst_p = dst.as_ref();

    let mut src_f = File::open(src_p)?;
    let mut dst_f = File::open(dst_p)?;
    let mut d1: Vec<u8> = Vec::new();
    let mut d2 = Vec::new();
    src_f.read_to_end(d1.as_mut())?;
    dst_f.read_to_end(d2.as_mut())?;
    let (_, hdr1) = KCDVideoHDR::from_bytes((&d1, 0))?;
    let (_, hdr2) = KCDVideoHDR::from_bytes((&d2, 0))?;
    let l1: Vec<PathBuf> = hdr1
        .data
        .iter()
        .filter_map(|s| {
            s.filepath
                .split('\\')
                .last()
                .map(|v| src_p.with_file_name(v))
        })
        .collect();
    let l2: Vec<PathBuf> = hdr2
        .data
        .iter()
        .filter_map(|s| {
            s.filepath
                .split('\\')
                .last()
                .map(|v| dst_p.with_file_name(v))
        })
        .collect();

    let bar_template = format!(
        "{} videos: {}",
        mode.as_ref(),
        "{bar:80.cyan/blue} {pos:>7}/{len:7} [{elapsed_precise}]"
    );
    let style = ProgressStyle::with_template(&bar_template);
    let tasks = l1.iter().zip(l2.iter()).progress_count(l1.len() as u64);

    let tasks = if let Ok(style) = style {
        tasks.with_style(style)
    } else {
        tasks
    };

    tasks.for_each(|(p1, p2)| match mode {
        Mode::Copy => {
            let _ = std::fs::copy(p1, p2).map(|_| ());
        }
        Mode::Move => {
            if !p2.is_file() {
                let _ = std::fs::rename(p1, p2);
            }
        }
    });

    println!(
        "Videos were renamed and {}d in: {}",
        mode.as_ref().to_lowercase(),
        dst_p.parent().unwrap().display(),
    );
    Ok(())
}

fn find_kcrmovie_position<P: AsRef<Path>>(p: P) -> Result<usize> {
    let file = File::open(p)?;
    let mut reader = BufReader::new(file);

    let pattern = "4B 43 52 4D 4F 56 49 45";
    let mut count = 0;
    let mut res = 0;
    while let Ok(buf) = reader.fill_buf() {
        let n = buf.len();
        if n == 0 {
            break;
        }
        if let Ok(pos) = patternscan::scan(std::io::Cursor::new(buf), pattern) {
            match pos.len() {
                0 => {
                    count += n;
                    reader.consume(n);
                }
                1 => {
                    res = count + pos[0];
                    break;
                }
                _ => {
                    reader.consume(n);
                    bail!("Invalid KCD file. (no `KCRMOIVE` or multiple kcd found)");
                }
            }
        }
    }
    Ok(res)
}

pub fn modify_kcrmovie_text<P: AsRef<Path>>(kcd: P, hdr: P, mode: Mode) -> Result<PathBuf> {
    let kcd: &Path = kcd.as_ref();
    let hdr: &Path = hdr.as_ref();
    
    println!("Creating linking between KCD `{}` and HDR `{}`", kcd.display(), hdr.display());
    let file_stem = hdr
        .file_stem()
        .with_context(|| format!("output is not a valid name: {}", hdr.display()))?
        .to_string_lossy();

    let pos = find_kcrmovie_position(kcd)
        .with_context(|| format!("Input is not a valid kcd file: {}", kcd.display()))?;

    let hdr_tag: String = format!("{}\\{}.hdr", file_stem, file_stem);
    let hdr_buf = hdr_tag.as_bytes();
    if hdr_buf.len() > 256 {
        bail!("output name is too long, please reduce to < 120 charaters");
    }
    let mut header = vec![0u8; pos + 16];

    let in_file = File::open(kcd).with_context(|| "Fail to open input file")?;
    let mut reader = BufReader::new(in_file);

    reader.read_exact(header.as_mut())?;

    let out_kcd = kcd.with_extension("kcd.modify");
    let mut writer = BufWriter::new(File::create(&out_kcd)?);

    writer.write_all(&header)?;

    let padding = vec![0u8; 256 - hdr_buf.len()];

    // wrote new hdr path into binary file
    writer.write_all(hdr_buf)?;

    writer.write_all(&padding)?;

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
    match mode {
        Mode::Move => {
            std::fs::remove_file(kcd).with_context(|| "Fail to remove original KCD file")?;
        }
        Mode::Copy => (),
    }
    Ok(out_kcd)
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

    let kcd_str = kcd.to_string_lossy().replace('/', "\\");
    let kcd_bits = kcd_str.as_bytes();
    let n_bits = kcd_bits.len();
    if n_bits > 256 {
        bail!("Input path of KCD file is too long (>256)");
    }

    let padding = vec![0u8; 256 - n_bits];
    let out_file = File::create(raf.with_extension("raf.modify"))
        .with_context(|| "Fail to create modified RAF file")?;
    let mut writer = BufWriter::new(out_file);
    writer.write_all(&header)?;
    writer.write_all(kcd_bits)?;
    writer.write_all(padding.as_ref())?;
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
    println!(
        "New RAF file was save as:{}",
        &raf.with_extension("raf.modify").display()
    );
    Ok(())
}

pub fn modify_video_hdr<P: AsRef<Path>>(hdr: P, prefix: &str) -> Result<PathBuf> {
    // video folder which contains hdr file and videos
    let hdr = hdr.as_ref();
    let mut input = File::open(hdr).with_context(|| "Fail to open hdr file")?;
    let mut buf: Vec<u8> = Vec::new();
    input.read_to_end(buf.as_mut())?;

    let (_, mut hdr_data) =
        KCDVideoHDR::from_bytes((&buf, 0)).with_context(|| "Fail to parse kcd hdr file")?;

    hdr_data.rename(prefix)?;

    let new_hdr = &hdr.with_file_name(format!("{}.hdr", prefix));

    let mut output = File::create(new_hdr)
        .with_context(|| format!("Fail to create new hdr file: {}", new_hdr.display()))?;
    let kcd_bytes = hdr_data.to_bytes()?;
    output.write_all(&kcd_bytes)?;
    println!("New HDR file was save as:{}", new_hdr.display());
    Ok(new_hdr.to_path_buf())
}

pub fn clone_kcd_with_videos(input: PathBuf, label: String, mode: Mode) -> Result<()> {
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
    let from_hdr = modify_video_hdr(&hdr, &label)?;
    let to_hdr = new_video_folder.join(from_hdr.file_name().unwrap());
    std::fs::rename(
        &from_hdr,
        &to_hdr,
    )?;
    let new_kcd_name = modify_kcrmovie_text(&kcd, &from_hdr, Mode::Copy)?;
    std::fs::rename(&new_kcd_name, new_kcd_name.with_file_name(format!("{}.kcd", &label)))?;
    move_videos(&hdr, &to_hdr, mode)
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct KCDVideoHDR {
    #[deku(bytes_read = "4")]
    header: Vec<u8>,
    #[deku(bytes = "4")]
    pub count: u32,
    #[deku(
        bytes_read = "292 * count",
        map = "KCDVideoHDR::try_read_video_block",
        writer = "KCDVideoHDR::try_write_video_block(deku::output, &self.data)"
    )]
    pub data: Vec<VideoBlock>,
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
    fn rename(&mut self, prefix: &str) -> Result<()> {
        if prefix.len() > 120 {
            bail!("Prefix is too long (<= 120 charaters")
        }
        self.data.iter_mut().for_each(|block| {
            let filepath_s: Vec<&str> = block.filepath.split('\\').collect();
            if let Some(&old_prefix) = filepath_s.first() {
                let new_filepath = block.filepath.replace(old_prefix, prefix);
                block.filepath = new_filepath;
            }
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
    use indicatif::{ProgressIterator, ProgressStyle};

    use super::*;

    use std::{
        fs::File,
        io::{Read, Write},
    };

    #[test]
    fn read_hdr() -> anyhow::Result<()> {
        let p = "./16-1_basal_nos-m4_230925-230927_3487-3488.0001.hdr";
        let mut file = File::open(p)?;
        let mut data: Vec<u8> = Vec::new();
        file.read_to_end(&mut data)?;

        let (_, kcd) = KCDVideoHDR::from_bytes((&data, 0))?;
        println!("{:?}", kcd);
        Ok(())
    }
    #[test]
    fn write_to_hdr() -> anyhow::Result<()> {
        let p = "./16-1_basal_nos-m4_230925-230927_3487-3488.0001.hdr";
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
    fn test_bar() -> anyhow::Result<()> {
        let mode = Mode::Move;
        let style = ProgressStyle::with_template(&format!(
            "{} videos: {}",
            mode.as_ref(),
            "{bar:80.cyan/blue} {pos:>7}/{len:7} [{elapsed_precise}]"
        ))?;
        (0..100)
            .progress_count(100)
            .with_style(style)
            .try_for_each(|_| {
                std::thread::sleep(std::time::Duration::from_millis(10));
                Ok(())
            })
    }
    #[test]
    fn test_kcrmovie_position() -> Result<()> {
        let pos = find_kcrmovie_position("./test.0001.kcd")?;
        println!("{pos}");
        Ok(())
    }

    #[test]
    fn test_retrieve() -> Result<()> {
        let src = r"D:\EEG_test\D_211001-211002_Chat-Cre_Flowerpot_372_375_376_373.0001\D_211001-211002_Chat-Cre_Flowerpot_372_375_376_373.0001.hdr";
        let dst = r"D:\EEG_test\abc.0001\abc.0001.hdr";
        let src_p: &Path = src.as_ref();
        let dst_p: &Path = dst.as_ref();

        let mut src_f = File::open(src_p)?;
        let mut dst_f = File::open(dst_p)?;
        let mut d1: Vec<u8> = Vec::new();
        let mut d2 = Vec::new();
        src_f.read_to_end(d1.as_mut())?;
        dst_f.read_to_end(d2.as_mut())?;
        let (_, hdr1) = KCDVideoHDR::from_bytes((&d1, 0))?;
        let (_, hdr2) = KCDVideoHDR::from_bytes((&d2, 0))?;
        let l1: Vec<&str> = hdr1
            .data
            .iter()
            .filter_map(|s| s.filepath.split('\\').last())
            .collect();
        let l2: Vec<&str> = hdr2
            .data
            .iter()
            .filter_map(|s| s.filepath.split('\\').last())
            .collect();

        let bar_template = format!(
            "{} videos: {}",
            Mode::Move.as_ref(),
            "{bar:80.cyan/blue} {pos:>7}/{len:7} [{elapsed_precise}]"
        );
        let style = ProgressStyle::with_template(&bar_template);
        let tasks = l1.iter().zip(l2.iter()).progress_count(l1.len() as u64);

        let tasks = if let Ok(style) = style {
            tasks.with_style(style)
        } else {
            tasks
        };

        tasks.for_each(|(v1, v2)| {
            dbg!((v1, v2));
            let p1 = src_p.parent().unwrap().join(v1);
            let p2 = dst_p.parent().unwrap().join(v2);
            dbg!(p1.is_file(), p2);
        });
        Ok(())
    }
}
