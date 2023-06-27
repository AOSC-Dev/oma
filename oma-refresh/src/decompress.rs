use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use flate2::read::GzDecoder;
use oma_console::indicatif::{style::TemplateError, MultiProgress, ProgressBar};
use oma_fetch::FetchProgressBar;
use xz2::read::XzDecoder;

pub type DecompressResult<T> = std::result::Result<T, DecompressError>;

#[derive(Debug, thiserror::Error)]
pub enum DecompressError {
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error("Unsupport file type")]
    UnsupportedFileType,
    #[error(transparent)]
    TemplateError(#[from] TemplateError),
    #[error("Failed to get file name")]
    FileNameError,
}

pub enum OmaDecompresser {
    Gz(PathBuf),
    Xz(PathBuf),
    NoNeedtoDecompress,
}

impl OmaDecompresser {
    pub fn new(compress_file_path: PathBuf) -> Self {
        match compress_file_path.extension().and_then(|x| x.to_str()) {
            Some("gz") => Self::Gz(compress_file_path),
            Some("xz") => Self::Xz(compress_file_path),
            _ => Self::NoNeedtoDecompress,
        }
    }

    pub fn decompress(
        &self,
        bar: bool,
        count: usize,
        total: usize,
        extract_to: &Path,
        typ: &str,
    ) -> DecompressResult<()> {
        let bar = if bar {
            let mb = Arc::new(MultiProgress::new());

            Some(FetchProgressBar::new(
                mb,
                None,
                Some((count, total)),
                Some(typ.to_owned())
            ))
        } else {
            None
        };

        decompress(bar, extract_to, self)
    }
}

/// Extract database
fn decompress(
    fpb: Option<FetchProgressBar>,
    extract_to: &Path,
    typ: &OmaDecompresser,
) -> DecompressResult<()> {
    let compress_file = match typ {
        OmaDecompresser::Gz(p) => p,
        OmaDecompresser::Xz(p) => p,
        OmaDecompresser::NoNeedtoDecompress => return Ok(()),
    };

    let compress_f = std::fs::File::open(compress_file)?;
    let reader = std::io::BufReader::new(compress_f);

    let mut compress_file_no_ext = compress_file.clone();
    compress_file_no_ext.set_extension("");

    let filename = compress_file_no_ext
        .file_name()
        .and_then(|x| x.to_str())
        .ok_or_else(|| DecompressError::FileNameError)?;

    let pb = if let Some((mb, progress, msg)) = fpb.map(|x| (x.mb, x.progress, x.msg)) {
        let (style, inv) = oma_console::pb::oma_spinner(false)?;
        let pb = mb.add(ProgressBar::new_spinner().with_style(style));
        pb.enable_steady_tick(inv);

        let progress = if let Some((cur, total)) = progress {
            format!("({cur}/{total}) ")
        } else {
            "".to_string()
        };

        pb.set_message(format!("{progress}{}", msg.unwrap()));

        Some(pb)
    } else {
        None
    };

    let extract_path = extract_to.join(filename);

    let mut extract_f = std::fs::OpenOptions::new()
        .truncate(true)
        .write(true)
        .create(true)
        .open(extract_path)?;

    extract_f.set_len(0)?;

    let mut decompress: Box<dyn Read> = match typ {
        OmaDecompresser::Gz(_) => Box::new(GzDecoder::new(reader)),
        OmaDecompresser::Xz(_) => Box::new(XzDecoder::new(reader)),
        _ => return Err(DecompressError::UnsupportedFileType),
    };

    std::io::copy(&mut decompress, &mut extract_f)?;
    extract_f.flush()?;

    drop(extract_f);
    drop(decompress);

    if let Some(ref pb) = pb {
        pb.finish_and_clear();
    }

    Ok(())
}
