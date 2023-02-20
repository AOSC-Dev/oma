use std::process::Command;

use anyhow::{bail, Context, Result};
use console::style;
use grep::{
    regex::RegexMatcherBuilder,
    searcher::{sinks::UTF8, Searcher},
};

use crate::{db::APT_LIST_DISTS, utils::get_arch_name};

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

pub fn find(kw: &str, is_list: bool) -> Result<()> {
    let arch = get_arch_name().context("Can not get ARCH!")?;
    let kw_escape = regex::escape(kw);

    let pattern = if is_list {
        format!(r"^\s*(.*?)\s+((?:\S*[,/])?{kw_escape}(?:,\S*|))\s*$")
    } else {
        format!(r"^(.*?{kw_escape}(?:.*[^\s])?)\s+(\S+)\s*$")
    };

    let dir = std::fs::read_dir(APT_LIST_DISTS)?;
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

    if paths.is_empty() {
        bail!(
            "Contents database does not exist!\nPlease use {} to refresh the contents.",
            style("oma refresh").green()
        );
    }

    // 如果安装了 ripgrep，则使用 rg 来进行搜索操作，因为 rg 的速度比 grep 快十倍
    let mut res = if which::which("rg").is_ok() {
        let mut res = vec![];

        let rg_runner = Command::new("rg")
            .arg("--json")
            .arg("-e")
            .arg(pattern)
            .args(&paths)
            .output()?;

        if !rg_runner.status.success() {
            for i in String::from_utf8_lossy(&rg_runner.stdout).split('\n') {
                if !i.is_empty() {
                    let line: RgJson = serde_json::from_str(i)?;
                    if line.t == Some("summary".to_owned()) {
                        let data = line.data;
                        let stats = data.stats;
                        if let Some(stats) = stats {
                            if stats.matched_lines == 0 {
                                bail!("Can't find any item for: {kw}");
                            }
                        }
                    }
                }
            }

            bail!("rg return non-zero code.")
        }

        let mut output = std::str::from_utf8(&rg_runner.stdout)?.split('\n');
        // 去除最后的空行
        output.nth_back(0);

        for i in output {
            let line: RgJson = serde_json::from_str(i)?;
            if line.t == Some("match".to_owned()) {
                let submatches = line.data.submatches;
                if let Some(submatches) = submatches {
                    for j in submatches {
                        let m = j.m.text;
                        let mut split = m.split_whitespace();
                        let file = split.next();
                        let pkg_group = split.next();

                        if file.and(pkg_group).is_some() {
                            let pkg_group = pkg_group.unwrap();
                            let split_group = pkg_group.split(',').collect::<Vec<_>>();

                            // 比如 / admin/apt-file,admin/api,...
                            if !split_group.is_empty() {
                                for i in split_group {
                                    if is_list && i.split('/').nth(1) == Some(kw) {
                                        let file = file_handle(file.unwrap());
                                        let s = format!("{kw}: {file}");
                                        if !res.contains(&s) {
                                            res.push(s);
                                        }
                                    } else if !is_list
                                        && i.contains(kw)
                                        && i.split('/').nth(1).is_some()
                                    {
                                        let file = file_handle(file.unwrap());
                                        let s =
                                            format!("{}: {}", i.split('/').nth(1).unwrap(), file);
                                        if !res.contains(&s) {
                                            res.push(s);
                                        }
                                    }
                                }
                            } else {
                                // 比如 /usr/bin/apt admin/apt
                                let pkg = pkg_group.split('/').nth(1);
                                if let Some(pkg) = pkg {
                                    let file = file_handle(file.unwrap());
                                    let s = format!("{pkg}: {file}");
                                    if !res.contains(&s) {
                                        res.push(s);
                                    }
                                }
                            }
                        }
                    }
                }
            }
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
                    let mut split = line.split_whitespace();
                    let file = split.next();
                    let package = split.next().and_then(|x| x.split('/').nth(1));
                    if file.and(package).is_some() {
                        let file = file_handle(file.unwrap());
                        let s = format!("{}: {}", package.unwrap(), file);
                        if !res.contains(&s) {
                            res.push(s);
                        }
                    }

                    Ok(true)
                }),
            )?;
        }

        res
    };

    res.sort();

    for i in res {
        println!("{i}");
    }

    Ok(())
}

fn file_handle(s: &str) -> String {
    let s = if s.starts_with("./") {
        s.strip_prefix('.').unwrap().to_string()
    } else if !s.starts_with("./") && !s.starts_with('/') {
        "/".to_owned() + s
    } else {
        s.to_owned()
    };

    s
}
