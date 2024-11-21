use std::{
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

use flate2::bufread::GzDecoder;
use lzzzz::lz4f::BufReadDecompressor;
use memchr::memmem;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tracing::debug;
use zstd::Decoder;

use crate::{parser::single_line, OmaContentsError};

const ZSTD_MAGIC: &[u8] = &[40, 181, 47, 253];
const LZ4_MAGIC: &[u8] = &[0x04, 0x22, 0x4d, 0x18];
const GZIP_MAGIC: &[u8] = &[0x1F, 0x8B];

#[derive(Debug, Clone, Copy)]
/// Mode
///
/// `Mode` is an enumeration that represents different search modes for the package manager.
/// Each variant corresponds to a specific type of search operation.
pub enum Mode {
    /// Search for packages that provide a specific capability.
    Provides,
    /// Search for files within packages.
    Files,
    /// Search for source packages that provide a specific capability.
    ProvidesSrc,
    /// Search for files within source packages.
    FilesSrc,
    /// Search for binary packages that provide a specific capability.
    BinProvides,
    /// Search for files within binary packages.
    BinFiles,
}

const BIN_PREFIX: &str = "usr/bin";
#[cfg(not(feature = "aosc"))]
const BIN_PREFIX_WITH_PREFIX: &str = "/usr/bin";

impl Mode {
    fn paths(&self, dir: &Path) -> Result<Vec<PathBuf>, OmaContentsError> {
        use std::fs;

        #[cfg(feature = "aosc")]
        let contains_name = match self {
            Mode::FilesSrc | Mode::ProvidesSrc => |x: &str| x.contains("_Contents-source"),
            Mode::Provides | Mode::Files => {
                |x: &str| x.contains("_Contents-") && !x.contains("_Contents-source")
            }
            Mode::BinProvides | Mode::BinFiles => |x: &str| x.contains("_BinContents-"),
        };

        #[cfg(not(feature = "aosc"))]
        let contains_name = match self {
            Mode::FilesSrc | Mode::ProvidesSrc => |x: &str| x.contains("_Contents-source"),
            Mode::Provides | Mode::Files | Mode::BinFiles | Mode::BinProvides => {
                |x: &str| x.contains("_Contents-") && !x.contains("_Contents-source")
            }
        };

        let mut paths = vec![];

        for i in fs::read_dir(dir)
            .map_err(|e| OmaContentsError::FailedToOperateDirOrFile(dir.display().to_string(), e))?
            .flatten()
        {
            if i.file_name().to_str().is_some_and(contains_name) {
                paths.push(i.path());
            }
        }

        if paths.is_empty() {
            return Err(OmaContentsError::ContentsNotExist);
        }

        Ok(paths)
    }
}

/// Perform a search using ripgrep
///
/// This function performs a search using the `ripgrep` command-line tool based on the specified mode and query.
/// It processes the search results and invokes a callback function for each match found.
///
/// # Arguments
///
/// * `dir` - A reference to a `Path` that specifies the directory to search in.
/// * `mode` - A `Mode` enum value that specifies the type of search operation to perform.
/// * `query` - A string slice that contains the search query.
/// * `cb` - A mutable callback function that takes a tuple of two strings (the matched line and the matched part) as its argument.
///
/// # Returns
///
/// Returns a `Result` which is `Ok(())` if the search is successful, or an `OmaContentsError` if an error occurs.
pub fn ripgrep_search(
    dir: impl AsRef<Path>,
    mode: Mode,
    query: &str,
    mut cb: impl FnMut((String, String)),
) -> Result<(), OmaContentsError> {
    let query = regex::escape(query);
    let query = strip_path_prefix(&query);

    let (regex, is_list) = match mode {
        Mode::Provides | Mode::ProvidesSrc | Mode::BinProvides => {
            (format!(r"^(.*?{query}(?:.*[^\s])?)\s+(\S+)\s*$"), false)
        }
        Mode::Files | Mode::FilesSrc | Mode::BinFiles => (
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
            return !file.starts_with(BIN_PREFIX_WITH_PREFIX);
        },
        _ => |_: &str| false,
    };

    while stdout_reader.read_line(&mut buffer).is_ok_and(|x| x > 0) {
        if let Some(lines) = rg_filter_line(&buffer, is_list, query) {
            for i in lines {
                #[cfg(not(feature = "aosc"))]
                if is_bin(&i.1) {
                    buffer.clear();
                    continue;
                }

                cb(i);
            }
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

fn strip_path_prefix(query: &str) -> &str {
    if Path::new(query).is_absolute() {
        query.strip_prefix('/').unwrap_or(query)
    } else {
        query
    }
}
/// Perform a pure search
///
/// This function performs a search directly on the file contents.
///
/// # Arguments
///
/// * `path` - A reference to a `Path` that specifies the directory or file to search in.
/// * `mode` - A `Mode` enum value that specifies the type of search operation to perform.
/// * `query` - A string slice that contains the search query.
/// * `cb` - A mutable callback function that takes a tuple of two strings (the matched line and the matched part) as its argument. The callback must implement `Sync` and `Send` traits.
///
/// # Returns
///
/// Returns a `Result` which is `Ok(())` if the search is successful, or an `OmaContentsError` if an error occurs.
pub fn pure_search(
    path: impl AsRef<Path>,
    mode: Mode,
    query: &str,
    mut cb: impl FnMut((String, String)) + Sync + Send,
) -> Result<(), OmaContentsError> {
    let paths = mode.paths(path.as_ref())?;
    let query = strip_path_prefix(query);
    let query = Arc::from(query);

    let (tx, rx) = mpsc::channel();

    let worker = thread::spawn(move || {
        paths
            .par_iter()
            .map(move |path| pure_search_contents_from_path(path, &query, mode, &tx))
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

    let can_next = match mode {
        Mode::Provides | Mode::ProvidesSrc => |_pkg: &str, file: &str, query: &str| {
            memmem::find(file.as_bytes(), query.as_bytes()).is_some()
        },
        Mode::Files | Mode::FilesSrc => |pkg: &str, _file: &str, query: &str| pkg == query,
        Mode::BinProvides => |_pkg: &str, file: &str, query: &str| {
            memmem::find(file.as_bytes(), query.as_bytes()).is_some()
                && file.starts_with(BIN_PREFIX)
        },
        Mode::BinFiles => {
            |pkg: &str, file: &str, query: &str| pkg == query && file.starts_with(BIN_PREFIX)
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
        let (file, pkgs) = match single_line(&buffer) {
            Some(line) => line,
            None => continue,
        };

        for pkg in pkgs {
            if let Some(pkg) = pkg_name(pkg) {
                if next(pkg, file, query) {
                    let line = (pkg.to_string(), prefix(file));

                    tx.send(line).unwrap();
                }
            }
        }

        buffer.clear();
    }
}

fn rg_filter_line(line: &str, is_list: bool, query: &str) -> Option<Vec<(String, String)>> {
    let (file, pkgs) = single_line(line)?;

    debug!("file: {file}, pkgs: {pkgs:?}");

    if pkgs.len() != 1 {
        let mut res = vec![];
        for pkg in pkgs {
            let Some(pkg) = pkg_name(pkg) else {
                continue;
            };

            if pkg == query || !is_list {
                let file = prefix(file);
                res.push((pkg.to_string(), file));
            }
        }

        Some(res)
    } else {
        // 比如 /usr/bin/apt admin/apt
        let pkg = pkgs[0];
        let pkg = pkg_name(pkg)?;
        let file = prefix(file);

        Some(vec![(pkg.to_string(), file)])
    }
}

fn pkg_name(pkg: &str) -> Option<&str> {
    pkg.split('/').last()
}

#[inline]
fn prefix(s: &str) -> String {
    if s.starts_with('/') {
        s.to_string()
    } else {
        "/".to_owned() + s
    }
}

#[test]
fn test_rg_filter_line() {
    let s = "usr/bin/yakuake   Trinity/yakuake-trinity,utils/yakuake";
    let res = rg_filter_line(s, false, "yakuake");
    assert_eq!(
        res,
        Some(vec![
            (
                "yakuake-trinity".to_string(),
                "/usr/bin/yakuake".to_string()
            ),
            ("yakuake".to_string(), "/usr/bin/yakuake".to_string())
        ])
    );

    let s = "usr/bin/yakuake   Trinity/yakuake-trinity";
    let res = rg_filter_line(s, false, "yakuake");
    assert_eq!(
        res,
        Some(vec![(
            "yakuake-trinity".to_string(),
            "/usr/bin/yakuake".to_string()
        )])
    );
}
