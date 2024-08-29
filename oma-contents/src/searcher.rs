use std::{
    borrow::Cow,
    fs,
    io::{BufRead, BufReader, Read, Seek},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        mpsc::{self, Sender},
        Arc,
    },
    thread,
};

use aho_corasick::AhoCorasick;
use flate2::bufread::GzDecoder;
use lzzzz::lz4f::BufReadDecompressor;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tracing::debug;
use zstd::Decoder;

use crate::{parser::single_line, OmaContentsError};

const ZSTD_MAGIC: &[u8] = &[40, 181, 47, 253];
const LZ4_MAGIC: &[u8] = &[0x04, 0x22, 0x4d, 0x18];
const GZIP_MAGIC: &[u8] = &[0x1F, 0x8B];

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Provides,
    Files,
    BinProvides,
    BinFiles,
}

const BIN_PREFIX: &str = "usr/bin";

impl Mode {
    fn paths(&self, dir: &Path) -> Result<Vec<PathBuf>, OmaContentsError> {
        use std::fs;

        #[cfg(feature = "aosc")]
        let contains_name = match self {
            Mode::Provides | Mode::Files => "_Contents-",
            Mode::BinProvides | Mode::BinFiles => "_BinContents-",
        };

        #[cfg(not(feature = "aosc"))]
        let contains_name = "_Contents-";

        let mut paths = vec![];

        for i in fs::read_dir(dir)
            .map_err(|e| OmaContentsError::FailedToOperateDirOrFile(dir.display().to_string(), e))?
            .flatten()
        {
            if i.file_name()
                .into_string()
                .is_ok_and(|x| x.contains(contains_name))
            {
                paths.push(i.path());
            }
        }

        if paths.is_empty() {
            return Err(OmaContentsError::ContentsNotExist);
        }

        Ok(paths)
    }
}

pub fn ripgrep_search(
    dir: impl AsRef<Path>,
    mode: Mode,
    query: &str,
    mut cb: impl FnMut((String, String)),
) -> Result<(), OmaContentsError> {
    let query = regex::escape(query);

    let query = if Path::new(&query).is_absolute() {
        Cow::Borrowed(query.strip_prefix('/').unwrap_or(&query))
    } else {
        Cow::Owned(query)
    };

    let (regex, is_list) = match mode {
        Mode::Provides | Mode::BinProvides => {
            (format!(r"^(.*?{query}(?:.*[^\s])?)\s+(\S+)\s*$"), false)
        }
        Mode::Files | Mode::BinFiles => (
            format!(r"^\s*(.*?)\s+((?:\S*[,/])?{query}(?:,\S*|))\s*$"),
            true,
        ),
    };

    let mut cmd = Command::new("rg")
        .arg("-N")
        .arg("-I")
        .args(mode.paths(dir.as_ref())?)
        .arg("--search-zip")
        .arg("-e")
        .arg(regex)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(OmaContentsError::ExecuteRgFailed)?;

    let stdout = cmd
        .stdout
        .as_mut()
        .expect("Unexpected error: can not get stdout, maybe you environment is broken?");

    let mut stdout_reader = BufReader::new(stdout);

    let mut has_result = false;

    let mut buffer = String::new();

    #[cfg(not(feature = "aosc"))]
    let is_bin = match mode {
        Mode::BinProvides | Mode::BinFiles => |file: &str| {
            return !file.starts_with(BIN_PREFIX);
        },
        _ => |_: &str| false,
    };

    while stdout_reader.read_line(&mut buffer).is_ok_and(|x| x > 0) {
        if let Some(line) = rg_filter_line(&buffer, is_list, &query) {
            #[cfg(not(feature = "aosc"))]
            if is_bin(&line.1) {
                continue;
            }

            cb(line);

            has_result = true;
        }

        buffer.clear();
    }

    if !has_result {
        return Err(OmaContentsError::NoResult);
    }

    if !cmd
        .wait()
        .map_err(OmaContentsError::FailedToWaitExit)?
        .success()
    {
        return Err(OmaContentsError::RgWithError);
    }

    Ok(())
}

pub fn pure_search(
    path: impl AsRef<Path>,
    mode: Mode,
    query: &str,
    mut cb: impl FnMut((String, String)) + Sync + Send,
) -> Result<(), OmaContentsError> {
    let paths = mode.paths(path.as_ref())?;
    let ac = AhoCorasick::new([query])?;
    let query = Arc::from(query);

    let (tx, rx) = mpsc::channel();

    let worker = thread::spawn(move || {
        paths
            .par_iter()
            .map(move |path| pure_search_contents_from_path(path, &query, mode, &ac, &tx))
            .collect::<Result<(), OmaContentsError>>()
    });

    loop {
        if let Ok(v) = rx.try_recv() {
            cb(v)
        }

        if worker.is_finished() {
            return worker.join().unwrap();
        }
    }
}

fn pure_search_contents_from_path(
    path: &Path,
    query: &str,
    mode: Mode,
    ac: &AhoCorasick,
    tx: &Sender<(String, String)>,
) -> Result<(), OmaContentsError> {
    let mut f = fs::File::open(path)
        .map_err(|e| OmaContentsError::FailedToOperateDirOrFile(path.display().to_string(), e))?;

    let mut buf = [0; 4];
    f.read_exact(&mut buf).ok();
    f.rewind().map_err(|e| {
        debug!("{e}");
        OmaContentsError::IllegalFile(path.display().to_string())
    })?;

    let ext = path.extension().and_then(|x| x.to_str());

    let contents_reader: &mut dyn Read = match ext {
        Some("zst") => {
            check_file_magic_4bytes(buf, path, ZSTD_MAGIC)?;
            // https://github.com/gyscos/zstd-rs/issues/281
            &mut Decoder::new(BufReader::new(f)).unwrap()
        }
        Some("lz4") => {
            check_file_magic_4bytes(buf, path, LZ4_MAGIC)?;
            &mut BufReadDecompressor::new(BufReader::new(f))?
        }
        Some("gz") => {
            if buf[..2] != *GZIP_MAGIC {
                return Err(OmaContentsError::IllegalFile(path.display().to_string()));
            }
            &mut GzDecoder::new(BufReader::new(f))
        }
        _ => &mut BufReader::new(f),
    };

    let reader = BufReader::new(contents_reader);

    let can_next: &dyn Fn(&str, &str, &str) -> bool = match mode {
        Mode::Provides => &|_pkg: &str, file: &str, _query: &str| ac.is_match(file),
        Mode::Files => &|pkg: &str, _file: &str, query: &str| pkg == query,
        Mode::BinProvides => &|_pkg: &str, file: &str, _query: &str| {
            ac.is_match(file) && file.starts_with(BIN_PREFIX)
        },
        Mode::BinFiles => {
            &|pkg: &str, file: &str, query: &str| pkg == query && file.starts_with(BIN_PREFIX)
        }
    };

    pure_search_foreach_result(can_next, reader, query, tx);

    Ok(())
}

#[inline]
fn check_file_magic_4bytes(
    buf: [u8; 4],
    path: &Path,
    magic: &[u8],
) -> Result<(), OmaContentsError> {
    if buf != magic {
        return Err(OmaContentsError::IllegalFile(path.display().to_string()));
    }

    Ok(())
}

fn pure_search_foreach_result(
    next: impl Fn(&str, &str, &str) -> bool,
    mut reader: BufReader<&mut dyn Read>,
    query: &str,
    tx: &Sender<(String, String)>,
) {
    let mut buffer = String::new();

    while reader.read_line(&mut buffer).is_ok_and(|x| x > 0) {
        let (file, pkgs) = match single_line::<()>(&mut buffer.as_str()) {
            Ok(line) => line,
            Err(_) => continue,
        };

        for (_, pkg) in pkgs {
            if next(pkg, file, query) {
                let line = (pkg.to_string(), prefix(file));

                tx.send(line).unwrap();
            }
        }

        buffer.clear();
    }
}

fn rg_filter_line(mut line: &str, is_list: bool, query: &str) -> Option<(String, String)> {
    let (file, pkgs) = single_line::<()>(&mut line).ok()?;

    debug!("file: {file}, pkgs: {pkgs:?}");

    if pkgs.len() != 1 {
        for (_, pkg) in pkgs {
            if pkg == query || !is_list {
                let file = prefix(file);
                return Some((pkg.to_string(), file));
            }
        }
    } else {
        // 比如 /usr/bin/apt admin/apt
        let (_, pkg) = pkgs[0];
        let file = prefix(file);
        return Some((pkg.to_string(), file));
    }

    None
}

#[inline]
fn prefix(s: &str) -> String {
    if s.starts_with('/') {
        s.to_string()
    } else {
        "/".to_owned() + s
    }
}
