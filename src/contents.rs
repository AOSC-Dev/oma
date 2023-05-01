use std::{
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
    time::SystemTime,
};

use anyhow::{anyhow, bail, Result};
use console::style;
use grep::{
    regex::RegexMatcherBuilder,
    searcher::{sinks::UTF8, Searcher},
};
use indicatif::ProgressBar;
use time::OffsetDateTime;

use crate::{db::APT_LIST_DISTS, download::oma_spinner, warn, ARCH};
use std::sync::atomic::Ordering;

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

pub fn find(kw: &str, is_list: bool, cnf: bool) -> Result<Vec<(String, String)>> {
    let arch = ARCH.get().unwrap();
    let kw = if Path::new(kw).is_absolute() {
        kw.strip_prefix('/').unwrap()
    } else {
        kw
    };

    let kw_escape = regex::escape(kw);

    let pattern = if is_list {
        format!(r"^\s*(.*?)\s+((?:\S*[,/])?{kw_escape}(?:,\S*|))\s*$")
    } else {
        format!(r"^(.*?{kw_escape}(?:.*[^\s])?)\s+(\S+)\s*$")
    };

    let dir = std::fs::read_dir(&*APT_LIST_DISTS)?;
    let mut paths = Vec::new();
    for i in dir.flatten() {
        if i.file_name()
            .to_str()
            .unwrap_or("")
            .ends_with(&format!("_Contents-{arch}"))
            || i.file_name()
                .to_str()
                .unwrap_or("")
                .ends_with("_Contents-all")
        {
            paths.push(i.path());
        }
    }

    for i in &paths {
        let m = OffsetDateTime::from(i.metadata()?.created()?);
        let now = OffsetDateTime::from(SystemTime::now());
        let delta = now - m;
        let delta = delta.as_seconds_f64() / 60.0 / 60.0 / 24.0;
        if delta > 7.0 {
            warn!(
                "Contents file: {} has not been updated for a week, so the search results may not be accurate, please use 'oma refresh' to refresh the database.",
                i.file_name().unwrap_or_default().to_string_lossy()
            );
        }
    }

    if paths.is_empty() {
        bail!(
            "Contents database does not exist!\nPlease use {} to refresh the contents.",
            style("oma refresh").green()
        );
    }

    // 如果安装了 ripgrep，则使用 rg 来进行搜索操作，因为 rg 的速度比 grep 快十倍
    let mut res = if which::which("rg").is_ok() {
        let mut res = vec![];

        let pb = if !cnf {
            let pb = ProgressBar::new_spinner();
            oma_spinner(&pb);
            Some(pb)
        } else {
            None
        };

        let mut cmd = Command::new("rg")
            .arg("--json")
            .arg("-e")
            .arg(pattern)
            .args(&paths)
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("Spawn rg failed, why: {e}"))?;

        {
            if let Some(ref pb) = pb {
                pb.set_message("Searching ...");
            }

            let stdout = cmd
                .stdout
                .as_mut()
                .expect("Unexpected error: can not get stdout, maybe you environment is broken?");

            let stdout_reader = BufReader::new(stdout);
            let stdout_lines = stdout_reader.lines();

            let mut count = 0;

            for i in stdout_lines.flatten() {
                if !i.is_empty() {
                    let line: RgJson = serde_json::from_str(&i)
                        .map_err(|e| anyhow!("BUG: Parse rg item {} failed, why: {e}, Please report to upstream: https://github.com/aosc-dev/oma", &i))?;
                    let data = line.data;
                    if line.t == Some("summary".to_owned()) {
                        let stats = data.stats;
                        if let Some(stats) = stats {
                            if stats.matched_lines == 0 {
                                bail!("Can't find any item for: {kw}");
                            }
                        }
                    }

                    if line.t == Some("match".to_owned()) {
                        let submatches = data.submatches;
                        if let Some(submatches) = submatches {
                            count += 1;

                            if let Some(x) = pb.as_ref() {
                                x.set_message(format!(
                                    "Searching, found {count} results so far ..."
                                ))
                            }

                            for j in submatches {
                                let m = j.m.text;
                                if let Some(l) = parse_line(&m, is_list, kw) {
                                    if cnf {
                                        let last = l.1.split_whitespace().last();
                                        if last != Some(kw)
                                            && last != Some(&format!("/{kw}"))
                                            && last != Some(&format!("./{kw}"))
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
            bail!("rg return not-zero code!");
        }

        if let Some(x) = pb {
            x.finish_and_clear()
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
                    let line = parse_line(line, is_list, kw);
                    if let Some(l) = line {
                        if cnf && l.1.split_whitespace().last() != Some(kw) {
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
    let mut split = line.split_whitespace();
    let file = split.next();
    let pkg_group = split.next();

    if file.and(pkg_group).is_some() {
        let pkg_group = pkg_group.unwrap();
        let split_group = pkg_group.split(',').collect::<Vec<_>>();

        // 比如 / admin/apt-file,admin/apt,...
        if split_group.len() != 1 {
            for i in split_group {
                if is_list && i.split('/').nth(1) == Some(kw) {
                    let file = prefix(file.unwrap());
                    let pkg = kw;
                    let s = format!("{kw}: {file}");

                    return Some((pkg.to_string(), s));
                } else if !is_list && i.contains(kw) && i.split('/').nth_back(0).is_some() {
                    let file = prefix(file.unwrap());
                    let pkg = i.split('/').nth_back(0).unwrap();
                    let s = format!("{pkg}: {file}");
                    return Some((pkg.to_string(), s));
                }
            }
        } else {
            // 比如 /usr/bin/apt admin/apt
            let pkg = pkg_group.split('/').nth_back(0);
            if let Some(pkg) = pkg {
                let file = prefix(file.unwrap());
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
