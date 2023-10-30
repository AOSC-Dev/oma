use std::{
    borrow::Cow,
    fs::DirEntry,
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(feature = "no-rg-binary")]
use std::sync::atomic::AtomicUsize;

use chrono::{DateTime, Utc};

use winnow::{
    combinator::{separated0, separated_pair},
    error::ParserError,
    token::{tag, take_till0, take_until1},
    PResult, Parser,
};

type Result<T> = std::result::Result<T, OmaContentsError>;

#[cfg(feature = "no-rg-binary")]
#[derive(Debug, thiserror::Error)]
pub enum OmaContentsError {
    #[error("Contents does not exist")]
    ContentsNotExist,
    #[error(transparent)]
    LzzzErr(#[from] lzzzz::lz4f::Error),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error("rg parse failed: input: {}, err: {}", input, err)]
    RgParseFailed { input: String, err: String },
    #[error("Contents entry missing path list: {0}")]
    ContentsEntryMissingPathList(String),
    #[error("Command not found wrong argument")]
    CnfWrongArgument,
    #[error("Ripgrep exited with error")]
    RgWithError,
    #[error("")]
    NoResult,
}

#[cfg(not(feature = "no-rg-binary"))]
#[derive(Debug, thiserror::Error)]
pub enum OmaContentsError {
    #[error("Contents does not exist")]
    ContentsNotExist,
    #[error("Execute ripgrep failed: {0:?}")]
    ExecuteRgFailed(std::io::Error),
    #[error("Failed to read dir or file: {0}, kind: {1}")]
    FailedToReadDirOrFile(String, std::io::Error),
    #[error("Failed to get file {0} metadata: {1}")]
    FailedToGetFileMetadata(String, std::io::Error),
    #[error("Failed to wait ripgrep to exit: {0}")]
    FailedToWaitExit(std::io::Error),
    #[error("Contents entry missing path list: {0}")]
    ContentsEntryMissingPathList(String),
    #[error("Command not found wrong argument")]
    CnfWrongArgument,
    #[error("Ripgrep exited with error")]
    RgWithError,
    #[error("")]
    NoResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryMode {
    /// apt-file search (bool is only search binary)
    Provides(bool),
    /// apt-file list (bool is only search binary)
    ListFiles(bool),
    /// command-not-found mode (only search binary)
    CommandNotFound,
}

pub enum ContentsEvent {
    Progress(usize),
    ContentsMayNotBeAccurate,
    Done,
}

/// Find contents
/// keywords: search keyword
/// query_mode: Query Mode
/// dist_dir: where is contents dir
/// callback: callback progress
pub fn find<F>(
    keyword: &str,
    query_mode: QueryMode,
    dist_dir: &Path,
    arch: &str,
    callback: F,
    search_zip: bool,
) -> Result<Vec<(String, String)>>
where
    F: Fn(ContentsEvent) + Send + Sync + Clone + 'static,
{
    let callback = Arc::new(callback);
    let dir = std::fs::read_dir(dist_dir).map_err(|e| {
        OmaContentsError::FailedToReadDirOrFile(dist_dir.display().to_string(), e)
    })?;

    let mut paths = Vec::new();

    let contain_contents_names = &[
        Cow::Owned(format!("_Contents-{arch}")),
        Cow::Borrowed("_Contents-all"),
    ];

    #[cfg(feature = "aosc")]
    let contain_bincontents_names = &[
        Cow::Owned(format!("_BinContents-{arch}")),
        Cow::Borrowed("_BinContents-all"),
    ];

    for i in dir.flatten() {
        #[cfg(feature = "aosc")]
        if query_mode != QueryMode::CommandNotFound
            && query_mode != QueryMode::ListFiles(true)
            && query_mode != QueryMode::Provides(true)
        {
            if contain_contents_names
                .iter()
                .any(|x| dir_entry_contains_name(&i, x))
            {
                paths.push(i.path());
            }
        } else if contain_bincontents_names
            .iter()
            .any(|x| dir_entry_contains_name(&i, x))
        {
            paths.push(i.path());
        }
        #[cfg(not(feature = "aosc"))]
        if contain_contents_names
            .iter()
            .any(|x| dir_entry_contains_name(&i, x))
        {
            paths.push(i.path());
        }
    }

    if paths.is_empty() {
        return Err(OmaContentsError::ContentsNotExist);
    }

    let cc = callback.clone();

    let paths_ref = &paths;

    std::thread::scope(|s| {
        s.spawn(move || -> Result<()> {
            for i in paths_ref {
                let m = DateTime::from(i.metadata().and_then(|x| x.created()).map_err(|e| {
                    OmaContentsError::FailedToGetFileMetadata(i.display().to_string(), e)
                })?);
                let now = Utc::now();
                let delta = now - m;
                let delta = delta.num_seconds() / 60 / 60 / 24;
                if delta > 7 {
                    cc(ContentsEvent::ContentsMayNotBeAccurate);
                    break;
                }
            }

            Ok(())
        });
    });

    let mut res = search(&paths, search_zip, query_mode, keyword, callback)?;
    res.sort_unstable();

    Ok(res)
}

fn dir_entry_contains_name(entry: &DirEntry, name: &str) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|x| x.contains(name))
        .unwrap_or(false)
}

#[cfg(not(feature = "no-rg-binary"))]
fn search<F>(
    paths: &[PathBuf],
    search_zip: bool,
    query_mode: QueryMode,
    kw: &str,
    callback: Arc<F>,
) -> Result<Vec<(String, String)>>
where
    F: Fn(ContentsEvent) + Send + Sync + Clone + 'static,
{
    use std::io::BufRead;
    use std::process::{Command, Stdio};
    let kw = if Path::new(kw).is_absolute() {
        // 当确定 keyword 是一个绝对路径时，则 strip / 肯定有值，所以直接 unwrap
        // 这在 Windows 上可能会崩溃，后面 oma 要跑在 Windows 上面再说吧……
        kw.strip_prefix('/').unwrap()
    } else {
        kw
    };

    let kw_escape = regex::escape(kw);

    let pattern = match query_mode {
        QueryMode::Provides(_) | QueryMode::CommandNotFound => {
            format!(r"^(.*?{kw_escape}(?:.*[^\s])?)\s+(\S+)\s*$")
        }
        QueryMode::ListFiles(_) => format!(r"^\s*(.*?)\s+((?:\S*[,/])?{kw_escape}(?:,\S*|))\s*$"),
    };

    let mut res = vec![];
    let mut cmd = Command::new("rg");
    cmd.arg("-N");
    cmd.arg("-I");
    cmd.arg("-e");
    cmd.arg(pattern);
    cmd.args(paths);
    if search_zip {
        cmd.arg("--search-zip");
    }
    cmd.stdout(Stdio::piped());

    let mut cmd = cmd.spawn().map_err(OmaContentsError::ExecuteRgFailed)?;
    {
        let stdout = cmd
            .stdout
            .as_mut()
            .expect("Unexpected error: can not get stdout, maybe you environment is broken?");

        let stdout_reader = BufReader::new(stdout);
        let stdout_lines = stdout_reader.lines();

        let mut count = 0;

        let mut has_res = false;

        for i in stdout_lines.flatten() {
            if let Some(line) = search_line(&i, matches!(query_mode, QueryMode::ListFiles(_)), kw) {
                count += 1;
                callback(ContentsEvent::Progress(count));
                if query_mode == QueryMode::CommandNotFound {
                    if !cfg!(feature = "aosc")
                        && !line.1.contains("/bin")
                        && !line.1.contains("/sbin")
                    {
                        continue;
                    }

                    let path_filename = line.1.split('/').last().ok_or_else(|| {
                        OmaContentsError::ContentsEntryMissingPathList(line.1.to_string())
                    })?;

                    if strsim::jaro_winkler(kw, path_filename).abs() < 0.9 {
                        continue;
                    }
                }
                if !res.contains(&line) {
                    res.push(line);
                }
                has_res = true;
            }
        }

        callback(ContentsEvent::Done);

        if !has_res {
            return Err(OmaContentsError::NoResult);
        }
    }

    if !cmd
        .wait()
        .map_err(OmaContentsError::FailedToWaitExit)?
        .success()
    {
        return Err(OmaContentsError::RgWithError);
    }
    Ok(res)
}

#[cfg(feature = "no-rg-binary")]
fn search<F>(
    paths: &[PathBuf],
    search_zip: bool,
    query_mode: QueryMode,
    kw: &str,
    callback: Arc<F>,
) -> Result<Vec<(String, String)>>
where
    F: Fn(ContentsEvent) + Send + Sync + Clone + 'static,
{
    use rayon::prelude::*;

    let count = Arc::new(AtomicUsize::new(0));
    let cc = callback.clone();

    let query_mode = Arc::new(query_mode);

    let res = paths
        .par_iter()
        .filter_map(move |path| {
            search_inner(
                path,
                search_zip,
                query_mode.clone(),
                kw,
                cc.clone(),
                count.clone(),
            )
            .ok()
        })
        .flatten()
        .collect::<Vec<_>>();

    callback(ContentsEvent::Done);

    Ok(res)
}

#[cfg(feature = "no-rg-binary")]
fn search_inner<F>(
    path: &Path,
    search_zip: bool,
    query_mode: Arc<QueryMode>,
    kw: &str,
    callback: Arc<F>,
    count: Arc<AtomicUsize>,
) -> Result<Vec<(String, String)>>
where
    F: Fn(ContentsEvent) + Send + Sync + Clone + 'static,
{
    use flate2::bufread::GzDecoder;
    use lzzzz::lz4f::BufReadDecompressor;
    use std::{
        io::{BufRead, Read},
        sync::atomic::Ordering,
    };
    let f = std::fs::File::open(path)?;
    let contents_entry: Box<dyn Read> = match path.extension().and_then(|x| x.to_str()) {
        Some("gz") if search_zip => Box::new(GzDecoder::new(BufReader::new(f))),
        Some("lz4") if search_zip => Box::new(BufReadDecompressor::new(BufReader::new(f))?),
        Some(_) | None => Box::new(BufReader::new(f)),
    };

    let mut res = vec![];

    let reader = BufReader::new(contents_entry);

    let is_list = matches!(*query_mode, QueryMode::ListFiles(_));
    let is_cnf = matches!(*query_mode, QueryMode::CommandNotFound);
    let bin_only = match *query_mode {
        QueryMode::Provides(b) => b,
        QueryMode::ListFiles(b) => b,
        QueryMode::CommandNotFound => true,
    };

    let files_not_only_bin = |pkg: &str| is_list && pkg == kw;
    let provides_not_only_bin = |file: &str| !is_list && file.contains(kw);
    let files_bin_only = |file: &str| {
        bin_only
            && is_list
            && file.split('/').last().map(|x| x == kw).unwrap_or(false)
            && (file.contains("bin/") || file.contains("sbin/"))
    };

    let provides_bin_only = |file: &str| {
        bin_only
            && !is_list
            && file
                .split('/')
                .last()
                .map(|x| x.contains(kw))
                .unwrap_or(false)
            && (file.contains("bin/") || file.contains("sbin/"))
    };

    for i in reader.lines() {
        Some(()).and_then(|_| {
            let i = i.ok()?;
            let (file, pkgs) = single_line::<()>(&mut i.as_str()).ok()?;
            for (_, pkg) in pkgs {
                if files_not_only_bin(pkg)
                    || provides_not_only_bin(file)
                    || files_bin_only(file)
                    || provides_bin_only(file)
                {
                    count.fetch_add(1, Ordering::SeqCst);
                    callback(ContentsEvent::Progress(count.load(Ordering::SeqCst)));

                    if is_cnf
                        && file
                            .split('/')
                            .last()
                            .map(|x| strsim::jaro_winkler(kw, x).abs() < 0.9)
                            .unwrap_or(true)
                    {
                        continue;
                    }

                    let file = prefix(file);
                    let r = (pkg.to_string(), file);
                    if !res.contains(&r) {
                        res.push(r);
                    }
                }
            }

            Some(())
        });
    }

    Ok(res)
}

/// Parse contents line
#[cfg(not(feature = "no-rg-binary"))]
fn search_line(mut line: &str, is_list: bool, kw: &str) -> Option<(String, String)> {
    let (file, pkgs) = single_line::<()>(&mut line).ok()?;

    if pkgs.len() != 1 {
        for (_, pkg) in pkgs {
            if is_list || pkg.contains(kw) {
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
fn pkg_split<'a, E: ParserError<&'a str>>(input: &mut &'a str) -> PResult<(&'a str, &'a str), E> {
    separated_pair(take_till0('/'), pkg_name_sep, second_single).parse_next(input)
}

#[inline]
fn pkg_name_sep<'a, E: ParserError<&'a str>>(input: &mut &'a str) -> PResult<(), E> {
    tag("/").void().parse_next(input)
}

#[inline]
fn second_single<'a, E: ParserError<&'a str>>(input: &mut &'a str) -> PResult<&'a str, E> {
    take_till0(|c| c == ',' || c == '\n').parse_next(input)
}

#[inline]
fn second<'a, E: ParserError<&'a str>>(input: &mut &'a str) -> PResult<Vec<(&'a str, &'a str)>, E> {
    separated0(pkg_split, ',').parse_next(input)
}

#[inline]
fn first<'a, E: ParserError<&'a str>>(input: &mut &'a str) -> PResult<&'a str, E> {
    take_until1("   ").parse_next(input)
}

#[inline]
fn sep<'a, E: ParserError<&'a str>>(input: &mut &'a str) -> PResult<(), E> {
    tag("   ").void().parse_next(input)
}

type ContentsLine<'a> = (&'a str, Vec<(&'a str, &'a str)>);
// #[cfg(feature = "no-rg-binary")]
// type ContentsLines<'a> = Vec<(&'a str, Vec<(&'a str, &'a str)>)>;

#[inline]
fn single_line<'a, E: ParserError<&'a str>>(input: &mut &'a str) -> PResult<ContentsLine<'a>, E> {
    separated_pair(first, sep, second).parse_next(input)
}

// #[cfg(feature = "no-rg-binary")]
// #[inline]
// fn multi_line<'a, E: ParserError<&'a str>>(input: &mut &'a str) -> PResult<ContentsLines<'a>, E> {
//     use winnow::combinator::{repeat, terminated};
//     repeat(1.., terminated(single_line, tag("\n"))).parse_next(input)
// }

#[inline]
fn prefix(s: &str) -> String {
    "/".to_owned() + s
}

#[test]
fn test_pkg_name() {
    let a = &mut "admin/apt-file\n/   qaq/qaq\n";
    let res = pkg_split::<()>(a);

    assert_eq!(res, Ok(("admin", "apt-file")))
}

#[test]
fn test_second_single() {
    let a = &mut "admin/apt-file\n/   qaq/qaq\n";
    let res = second_single::<()>(a);
    assert_eq!(res, Ok("admin/apt-file"));

    let b = &mut "admin/apt-file";
    let res = second_single::<()>(b);
    assert_eq!(res, Ok("admin/apt-file"));
}

#[test]
fn test_second() {
    let a = &mut "admin/apt-file,admin/apt\n";
    let res = second::<()>(a);
    assert_eq!(res, Ok(vec![("admin", "apt-file"), ("admin", "apt")]));

    let b = &mut "admin/apt-file\n";
    let res = second::<()>(b);
    assert_eq!(res, Ok(vec![("admin", "apt-file")]));
}

#[test]
fn test_single_line() {
    let a = &mut "opt/32/libexec   devel/gcc+32,devel/llvm+32,gnome/gconf+32,libs/gdk-pixbuf+32\nopt/32/share   devel/llvm+32,libs/alsa-plugins+32,x11/libxkbcommon+32,x11/mesa+32,x11/virtualgl+32";
    let res = single_line::<()>(a);
    assert_eq!(
        res,
        Ok((
            "opt/32/libexec",
            vec![
                ("devel", "gcc+32"),
                ("devel", "llvm+32"),
                ("gnome", "gconf+32"),
                ("libs", "gdk-pixbuf+32")
            ]
        ))
    );

    let b = &mut "/   admin/apt-file\n";
    let res = single_line::<()>(b);
    assert_eq!(res, Ok(("/", vec![("admin", "apt-file")])));
}

// #[cfg(feature = "no-rg-binary")]
// #[test]
// fn test_multiple_lines() {
//     let a = &mut "opt/32/libexec   devel/gcc+32,devel/llvm+32,gnome/gconf+32\nopt/32/share   devel/llvm+32,libs/alsa-plugins+32\n";
//     let res = multi_line::<()>(a);

//     assert_eq!(
//         res,
//         Ok(vec![
//             (
//                 "opt/32/libexec",
//                 vec![
//                     ("devel", "gcc+32"),
//                     ("devel", "llvm+32"),
//                     ("gnome", "gconf+32"),
//                 ]
//             ),
//             (
//                 "opt/32/share",
//                 vec![("devel", "llvm+32"), ("libs", "alsa-plugins+32")]
//             )
//         ])
//     )
// }
