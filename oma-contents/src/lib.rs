use std::{
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
    time::SystemTime,
};

// use console::style;
use grep::{
    regex::RegexMatcherBuilder,
    searcher::{sinks::UTF8, Searcher},
};

use oma_console::{info, warn};
use time::OffsetDateTime;

// use crate::{db::APT_LIST_DISTS, download::oma_spinner, fl, info, warn, ARCH};
// use std::sync::atomic::Ordering;

use serde::Deserialize;

#[derive(Deserialize)]
struct RgJson {
    #[serde(rename = "type")]
    t: Option<String>,
    data: Data,
}

#[derive(Deserialize)]
struct Stats {
    matched_lines: u64,
}

#[derive(Deserialize)]
struct Data {
    stats: Option<Stats>,
    submatches: Option<Vec<Submatches>>,
}

#[derive(Deserialize)]
struct Submatches {
    #[serde(rename = "match")]
    m: MatchValue,
}

#[derive(Deserialize)]
struct MatchValue {
    text: String,
}

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
    Provides(bool),
    ListFiles(bool),
    CommandNotFound,
}

pub fn find<F>(
    keyword: &str,
    query_mode: QueryMode,
    dist_dir: &Path,
    arch: &str,
    callback: F,
) -> Result<Vec<(String, String)>>
where
    F: Fn(usize),
{
    let kw = if Path::new(keyword).is_absolute() {
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
    let mut paths = Vec::new();
    for i in dir.flatten() {
        if query_mode != QueryMode::CommandNotFound
            && query_mode != QueryMode::ListFiles(true)
            && query_mode != QueryMode::Provides(true)
        {
            if i.file_name()
                .to_str()
                .unwrap_or("")
                .ends_with(&format!("Contents-{}", arch))
                || i.file_name()
                    .to_str()
                    .unwrap_or("")
                    .ends_with("_Contents-all")
            {
                paths.push(i.path());
            }
        } else if i
            .file_name()
            .to_str()
            .unwrap_or("")
            .ends_with(&format!("_BinContents-{}", arch))
            || i.file_name()
                .to_str()
                .unwrap_or("")
                .ends_with("_BinContents-all")
        {
            paths.push(i.path());
        }
    }

    let pc = paths.clone();

    if paths.is_empty() {
        return Err(OmaContentsError::ContentsNotExist);
    }

    std::thread::spawn(move || -> Result<()> {
        for i in pc {
            let m = OffsetDateTime::from(i.metadata()?.modified()?);
            let now = OffsetDateTime::from(SystemTime::now());
            let delta = now - m;
            let delta = delta.as_seconds_f64() / 60.0 / 60.0 / 24.0;
            if delta > 7.0 {
                warn!("contents-may-not-be-accurate-1");
                info!("contents-may-not-be-accurate-2");
                break;
            }
        }

        Ok(())
    });

    // 如果安装了 ripgrep，则使用 rg 来进行搜索操作，因为 rg 的速度比 grep 快十倍
    let mut res = if which::which("rg").is_ok() {
        let mut res = vec![];

        let mut cmd = Command::new("rg")
            .arg("--json")
            .arg("-e")
            .arg(pattern)
            .args(&paths)
            .stdout(Stdio::piped())
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

            for i in stdout_lines.flatten() {
                if !i.is_empty() {
                    let line: RgJson =
                        serde_json::from_str(&i).map_err(|e| OmaContentsError::RgParseFailed {
                            input: i,
                            err: e.to_string(),
                        })?;
                    let data = line.data;
                    if line.t == Some("summary".to_owned()) {
                        let stats = data.stats;
                        if let Some(stats) = stats {
                            if stats.matched_lines == 0 {
                                return Err(OmaContentsError::NoResult);
                            }
                        }
                    }

                    if line.t == Some("match".to_owned()) {
                        let submatches = data.submatches;
                        if let Some(submatches) = submatches {
                            count += 1;

                            callback(count);

                            let search_bin_name = if query_mode == QueryMode::CommandNotFound {
                                kw.split('/').last()
                            } else {
                                None
                            };

                            for j in submatches {
                                let m = j.m.text;
                                if let Some(l) = parse_line(
                                    &m,
                                    matches!(query_mode, QueryMode::ListFiles(_)),
                                    kw,
                                ) {
                                    if query_mode == QueryMode::CommandNotFound {
                                        let last = l.1.split_whitespace().last();
                                        let bin_name = last
                                            .and_then(|x| x.split('/').last())
                                            .ok_or_else(|| {
                                                OmaContentsError::ContentsEntryMissingPathList(
                                                    l.1.to_string(),
                                                )
                                            })?;

                                        if strsim::jaro_winkler(
                                            search_bin_name.ok_or_else(|| {
                                                OmaContentsError::CnfWrongArgument
                                            })?,
                                            bin_name,
                                        )
                                        .abs()
                                            < 0.9
                                        {
                                            continue;
                                        }
                                    }
                                    if !res.contains(&l) {
                                        res.push(l);
                                    }
                                }
                            }
                        }
                    }
                }
            }
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

        for i in &paths {
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

fn parse_line(line: &str, is_list: bool, kw: &str) -> Option<(String, String)> {
    let contents_white = " ".repeat(3);
    let mut split = line.split(contents_white.as_str());
    let file = split.next();
    let pkg_group = split.next();

    if let Some((file, pkg_group)) = file.zip(pkg_group) {
        let split_group = pkg_group.split(',').collect::<Vec<_>>();

        // 比如 / admin/apt-file,admin/apt,...
        if split_group.len() != 1 {
            for i in split_group {
                if is_list && i.split('/').nth(1) == Some(kw) {
                    let file = prefix(file);
                    let pkg = kw;
                    let s = format!("{kw}: {file}");

                    return Some((pkg.to_string(), s));
                } else if !is_list && i.contains(kw) && i.split('/').nth_back(0).is_some() {
                    let file = prefix(file);
                    let pkg = i.split('/').nth_back(0).unwrap();
                    let s = format!("{pkg}: {file}");
                    return Some((pkg.to_string(), s));
                }
            }
        } else {
            // 比如 /usr/bin/apt admin/apt
            let pkg = pkg_group.split('/').nth_back(0);
            if let Some(pkg) = pkg {
                let file = prefix(file);
                let s = format!("{pkg}: {file}");
                return Some((pkg.to_string(), s));
            }
        }
    }

    None
}

fn prefix(s: &str) -> String {
    "/".to_owned() + s
}
