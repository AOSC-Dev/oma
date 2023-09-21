use std::{
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Utc};
use grep::{
    regex::RegexMatcherBuilder,
    searcher::{sinks::UTF8, Searcher},
};

use winnow::{
    combinator::{opt, separated0, separated_pair},
    error::ParserError,
    token::{tag, take_till0, take_until1},
    PResult, Parser,
};

type Result<T> = std::result::Result<T, OmaContentsError>;

#[derive(Debug, thiserror::Error)]
pub enum OmaContentsError {
    #[error("Contents does not exist")]
    ContentsNotExist,
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
    #[error(transparent)]
    GrepBuilderError(#[from] grep::regex::Error),
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
    let callback = Arc::new(callback);
    let kw = if Path::new(keyword).is_absolute() {
        // 当确定 keyword 是一个绝对路径时，则 strip / 肯定有值，所以直接 unwrap
        // 这在 Windows 上可能会崩溃，后面 oma 要跑在 Windows 上面再说吧……
        keyword.strip_prefix('/').unwrap()
    } else {
        keyword
    };

    let kw_escape = regex::escape(kw);

    let pattern = match query_mode {
        QueryMode::Provides(_) | QueryMode::CommandNotFound => {
            format!(r"^(.*?{kw_escape}(?:.*[^\s])?)\s+(\S+)\s*$")
        }
        QueryMode::ListFiles(_) => format!(r"^\s*(.*?)\s+((?:\S*[,/])?{kw_escape}(?:,\S*|))\s*$"),
    };

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
            paths.push(i.path());
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
            let m = DateTime::from(i.metadata()?.modified()?);
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

    // 如果安装了 ripgrep，则使用 rg 来进行搜索操作，因为 rg 的速度比 grep 快十倍
    let mut res = if which::which("rg").is_ok() {
        let mut res = vec![];

        let mut cmd = Command::new("rg");
        cmd.arg("-N");
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

            let search_bin_name = if query_mode == QueryMode::CommandNotFound {
                kw.split('/').last()
            } else {
                None
            };

            for i in stdout_lines.flatten() {
                if let Some(line) =
                    parse_line(&i, matches!(query_mode, QueryMode::ListFiles(_)), kw)
                {
                    count += 1;
                    callback(ContentsEvent::Progress(count));
                    if query_mode == QueryMode::CommandNotFound {
                        let last = line.1.split_whitespace().last();
                        if !cfg!(feature = "aosc")
                            && last
                                .map(|x| !x.contains("/usr/bin") && !x.contains("/usr/sbin"))
                                .unwrap_or(true)
                        {
                            continue;
                        }
                        let bin_name = last.and_then(|x| x.split('/').last()).ok_or_else(|| {
                            OmaContentsError::ContentsEntryMissingPathList(line.1.to_string())
                        })?;

                        if strsim::jaro_winkler(
                            search_bin_name.ok_or_else(|| OmaContentsError::CnfWrongArgument)?,
                            bin_name,
                        )
                        .abs()
                            < 0.9
                        {
                            continue;
                        }
                    }
                    if !res.contains(&line) {
                        res.push(line);
                    }
                }
            }

            callback(ContentsEvent::Done);
        }

        if !cmd.wait()?.success() {
            return Err(OmaContentsError::RgWithError);
        }

        res
    } else {
        // 如果没有 rg，则 fallback 到使用 grep 库，缺点是比较慢
        let mut matcher = RegexMatcherBuilder::new();

        matcher
            .case_smart(true)
            .case_insensitive(false)
            .multi_line(false)
            .unicode(false)
            .octal(false)
            .word(false);

        let matcher = matcher.build(&pattern)?;
        let mut searcher = Searcher::new();
        let mut res = Vec::new();

        for i in &*paths.lock().unwrap() {
            searcher.search_path(
                matcher.clone(),
                i,
                UTF8(|_, line| {
                    let line = parse_line(line, matches!(query_mode, QueryMode::ListFiles(_)), kw);
                    if let Some(l) = line {
                        if query_mode == QueryMode::CommandNotFound
                            && l.1.split_whitespace().last() != Some(kw)
                        {
                            return Ok(true);
                        }
                        if !res.contains(&l) {
                            res.push(l);
                        }
                    }

                    Ok(true)
                }),
            )?;
        }

        res
    };

    res.sort();

    Ok(res)
}

/// Parse contents line
fn parse_line(mut line: &str, is_list: bool, kw: &str) -> Option<(String, String)> {
    let (file, pkgs) = single_line::<()>(&mut line).ok()?;

    if pkgs.len() != 1 {
        for (_, pkg) in pkgs {
            if is_list {
                let file = prefix(file);
                let s = format!("{kw}: {file}");

                return Some((pkg.to_string(), s));
            } else if !is_list && pkg.contains(kw) {
                let file = prefix(file);
                let s = format!("{pkg}: {file}");
                return Some((pkg.to_string(), s));
            }
        }
    } else {
        // 比如 /usr/bin/apt admin/apt
        let (_, pkg) = pkgs[0];
        let file = prefix(file);
        let s = format!("{pkg}: {file}");
        return Some((pkg.to_string(), s));
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
    let (res, _) = (take_till0(','), opt('\n')).parse_next(input)?;

    Ok(res)
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

#[inline]
fn single_line<'a, E: ParserError<&'a str>>(
    input: &mut &'a str,
) -> PResult<(&'a str, Vec<(&'a str, &'a str)>), E> {
    separated_pair(first, sep, second).parse_next(input)
}

#[inline]
fn prefix(s: &str) -> String {
    "/".to_owned() + s
}

#[test]
fn test_pkg_name() {
    let a = &mut "admin/apt-file\n";
    let res = pkg_split::<()>(a);

    assert_eq!(res, Ok(("admin", "apt-file")))
}

#[test]
fn test_second_single() {
    let a = &mut "admin/apt-file,admin/apt\n";
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

    let b = &mut "admin/apt-file";
    let res = second::<()>(b);
    assert_eq!(res, Ok(vec![("admin", "apt-file")]));
}

#[test]
fn test_single_line() {
    let a = &mut "/   admin/apt-file,admin/apt\n";
    let res = single_line::<()>(a);
    assert_eq!(
        res,
        Ok(("/", vec![("admin", "apt-file"), ("admin", "apt")]))
    );

    let b = &mut "/   admin/apt-file";
    let res = single_line::<()>(b);
    assert_eq!(res, Ok(("/", vec![("admin", "apt-file")])));
}
