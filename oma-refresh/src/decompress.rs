use std::{path::Path, io::{Read, Write}};

use flate2::read::GzDecoder;
use oma_console::indicatif::ProgressBar;
use oma_fetch::FetchProgressBar;
use xz2::read::XzDecoder;

pub type Result<T> = std::result::Result<T, DecompressError>;

#[derive(Debug, thiserror::Error)]
pub enum DecompressError {
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error("Unsupport file type")]
    UnsupportedFileType,

}

/// Extract database
fn decompress(
    compress_file_path: &Path,
    name: &str,
    fpb: Option<FetchProgressBar>,
    path: &Path,
    typ: String,
) -> Result<()> {
    if compress_file_path == path {
        return Ok(());
    }

    let compress_f = std::fs::File::open(compress_file_path)?;
    let reader = std::io::BufReader::new(compress_f);

    // let pb = if let Some(mb) = fpb.map(|x| x.mb) {
    //     Some(mb.add(ProgressBar::new_spinner().with_style(oma_console::pb::oma_spinner(false)?)))
    // } else {
    //     None
    // };

    // let progress = fpb;

    // let progress = if let Some((cur, total)) = progress {
    //     format!("({cur}/{total}) ")
    // } else {
    //     "".to_string()
    // };

    // pb.set_message(format!("{progress}{} {typ}", fl!("decompressing")));

    let mut extract_f = std::fs::OpenOptions::new()
        .truncate(true)
        .write(true)
        .create(true)
        .open(path)?;

    extract_f.set_len(0)?;

    let mut decompress: Box<dyn Read> = if name.ends_with(".gz") {
        Box::new(GzDecoder::new(reader))
    } else if name.ends_with(".xz") {
        Box::new(XzDecoder::new(reader))
    } else {
        return Err(DecompressError::UnsupportedFileType);
    };

    std::io::copy(&mut decompress, &mut extract_f)?;
    extract_f.flush()?;

    drop(extract_f);
    drop(decompress);

    Ok(())
}
