use std::{
    io::BufReader,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

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
    WhichError(#[from] which::Error),
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
    #[error(transparent)]
    WhichError(#[from] which::Error),
    #[error("Execute ripgrep failed: {0}")]
    ExecuteRgFailed(String),
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

#[derive(Debug, PartialEq, Eq)]
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
    let dir = std::fs::read_dir(dist_dir)?;
    let paths = Arc::new(Mutex::new(Vec::new()));
    for i in dir.flatten() {
        #[cfg(feature = "aosc")]
        if query_mode != QueryMode::CommandNotFound
            && query_mode != QueryMode::ListFiles(true)
            && query_mode != QueryMode::Provides(true)
        {
            if i.file_name()
                .to_str()
                .map(|x| x.contains(&format!("Contents-{arch}")))
                .unwrap_or(false)
                || i.file_name()
                    .to_str()
                    .map(|x| x.contains("_Contents-all"))
                    .unwrap_or(false)
            {
                paths.lock().unwrap().push(i.path());
            }
        } else if i
            .file_name()
            .to_str()
            .map(|x| x.ends_with(&format!("_BinContents-{arch}")))
            .unwrap_or(false)
            || i.file_name()
                .to_str()
                .map(|x| x.ends_with("_BinContents-all"))
                .unwrap_or(false)
        {
            paths.lock().unwrap().push(i.path());
        }
        #[cfg(not(feature = "aosc"))]
        if i.file_name()
            .to_str()
            .map(|x| x.contains(&format!("Contents-{arch}")))
            .unwrap_or(false)
            || i.file_name()
                .to_str()
                .map(|x| x.contains("_Contents-all"))
                .unwrap_or(false)
        {
            paths.lock().unwrap().push(i.path());
        }
    }

    let pc = paths.clone();

    {
        if paths.lock().unwrap().is_empty() {
            return Err(OmaContentsError::ContentsNotExist);
        }
    }

    let cc = callback.clone();

    std::thread::spawn(move || -> Result<()> {
        for i in &*pc.lock().unwrap() {
            let m = DateTime::from(i.metadata()?.created()?);
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

    let mut res = search(paths, search_zip, query_mode, keyword, callback)?;
    res.sort_unstable();

    Ok(res)
}

#[cfg(not(feature = "no-rg-binary"))]
fn search<F>(
    paths: Arc<Mutex<Vec<PathBuf>>>,
    search_zip: bool,
    query_mode: QueryMode,
    kw: &str,
    callback: F,
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
    which::which("rg")?;
    let mut cmd = Command::new("rg");
    cmd.arg("-N");
    cmd.arg("-I");
    cmd.arg("-e");
    cmd.arg(pattern);
    cmd.args(&*paths.lock().unwrap());
    if search_zip {
        cmd.arg("--search-zip");
    }
    cmd.stdout(Stdio::piped());

    let mut cmd = cmd
        .spawn()
        .map_err(|e| OmaContentsError::ExecuteRgFailed(e.to_string()))?;
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

        if !has_res {
            return Err(OmaContentsError::NoResult);
        }

        callback(ContentsEvent::Done);
    }
    if !cmd.wait()?.success() {
        return Err(OmaContentsError::RgWithError);
    }
    Ok(res)
}

#[cfg(feature = "no-rg-binary")]
fn search<F>(
    paths: Arc<Mutex<Vec<PathBuf>>>,
    search_zip: bool,
    query_mode: QueryMode,
    kw: &str,
    callback: F,
) -> Result<Vec<(String, String)>>
where
    F: Fn(ContentsEvent) + Send + Sync + Clone + 'static,
{
    use rayon::prelude::*;
    let paths = &*paths.lock().unwrap();

    let callback = Arc::new(callback);

    let contents = paths
        .par_iter()
        .filter_map(move |path| read_contents(path, search_zip).ok())
        .collect::<Vec<_>>();

    let is_list = matches!(query_mode, QueryMode::ListFiles(_));
    let mut res = vec![];
    let mut count = 0;

    for i in contents {
        Some(()).and_then(|_| {
            let contents_entry = multi_line::<()>(&mut i.as_str()).ok()?;

            for i in contents_entry {
                for (_, pkg) in i.1 {
                    if (is_list && pkg == kw) || (!is_list && i.0.contains(kw)) {
                        count += 1;
                        callback(ContentsEvent::Progress(count));
                        let file = prefix(i.0);
                        let r = (pkg.to_string(), file);
                        if !res.contains(&r) {
                            res.push(r);
                        }
                    }
                }
            }

            Some(())
        });
    }

    callback(ContentsEvent::Done);

    Ok(res)
}

#[cfg(feature = "no-rg-binary")]
fn read_contents(path: &Path, search_zip: bool) -> Result<String> {
    use flate2::bufread::GzDecoder;
    use lzzzz::lz4f::BufReadDecompressor;
    use std::io::Read;
    let f = std::fs::File::open(path)?;
    let mut contents_entry: Box<dyn Read> = match path.extension().and_then(|x| x.to_str()) {
        Some("gz") if search_zip => Box::new(GzDecoder::new(BufReader::new(f))),
        Some("lz4") if search_zip => Box::new(BufReadDecompressor::new(BufReader::new(f))?),
        Some(_) | None => Box::new(BufReader::new(f)),
    };

    let mut s = String::new();
    contents_entry.read_to_string(&mut s)?;

    Ok(s)
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
#[cfg(feature = "no-rg-binary")]
type ContentsLines<'a> = Vec<(&'a str, Vec<(&'a str, &'a str)>)>;

#[inline]
fn single_line<'a, E: ParserError<&'a str>>(input: &mut &'a str) -> PResult<ContentsLine<'a>, E> {
    separated_pair(first, sep, second).parse_next(input)
}

#[cfg(feature = "no-rg-binary")]
#[inline]
fn multi_line<'a, E: ParserError<&'a str>>(input: &mut &'a str) -> PResult<ContentsLines<'a>, E> {
    use winnow::combinator::{repeat, terminated};
    repeat(1.., terminated(single_line, tag("\n"))).parse_next(input)
}

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

#[cfg(feature = "no-rg-binary")]
#[test]
fn test_multiple_lines() {
    let a = &mut "opt/32/libexec   devel/gcc+32,devel/llvm+32,gnome/gconf+32\nopt/32/share   devel/llvm+32,libs/alsa-plugins+32\n";
    let res = multi_line::<()>(a);

    assert_eq!(
        res,
        Ok(vec![
            (
                "opt/32/libexec",
                vec![
                    ("devel", "gcc+32"),
                    ("devel", "llvm+32"),
                    ("gnome", "gconf+32"),
                ]
            ),
            (
                "opt/32/share",
                vec![("devel", "llvm+32"), ("libs", "alsa-plugins+32")]
            )
        ])
    )
}
