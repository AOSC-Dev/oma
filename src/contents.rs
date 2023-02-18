use std::process::Command;

use anyhow::{bail, Context, Result};
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
struct Data {
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

    let pattern = if is_list {
        format!(r"^\s*(.*?)\s+((?:\S*[,/])?{}(?:,\S*|))\s*$", kw)
    } else {
        format!(r"^(.*?{}(?:.*[^\s])?)\s+(\S+)\s*$", kw)
    };

    // 如果安装了 ripgrep，则使用 rg 来进行搜索操作，因为 rg 的速度比 grep 快十倍
    let res = if which::which("rg").is_ok() {
        let mut res = vec![];
        let mut paths = Vec::new();
        let dir = std::fs::read_dir(APT_LIST_DISTS)?;
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

        let rg_runner = Command::new("rg")
            .arg("--json")
            .arg("-e")
            .arg(pattern)
            .args(paths)
            .output()?;

        if !rg_runner.status.success() {
            bail!("Can't find any item for: {kw}");
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
                        let pkg = split.next().and_then(|x| x.split('/').nth(1));
                        if file.and(pkg).is_some() {
                            let s = format!("{}: {}", pkg.unwrap(), file.unwrap());
                            if !res.contains(&s) {
                                res.push(s);
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

        let dir = std::fs::read_dir(APT_LIST_DISTS)?;

        for i in dir.flatten() {
            let filename = i
                .file_name()
                .to_str()
                .context("Can not get filename!")?
                .to_string();

            if filename.ends_with(&format!("_Contents-{arch}"))
                || filename.ends_with(&"_Contents-all".to_string())
            {
                searcher.search_path(
                    matcher.clone(),
                    i.path(),
                    UTF8(|_, line| {
                        let mut split = line.split_whitespace();
                        let file = split.next();
                        let package = split.next().and_then(|x| x.split('/').nth(1));
                        if file.and(package).is_some() {
                            let s = format!("{}: {}", package.unwrap(), file.unwrap());
                            if !res.contains(&s) {
                                res.push(s);
                            }
                        }

                        Ok(true)
                    }),
                )?;
            }
        }

        res
    };

    for i in res {
        println!("{i}");
    }

    Ok(())
}
