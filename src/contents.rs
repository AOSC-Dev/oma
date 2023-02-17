use anyhow::{Context, Result};
use grep::{
    regex::RegexMatcherBuilder,
    searcher::{sinks::UTF8, Searcher},
};

use crate::{db::APT_LIST_DISTS, utils::get_arch_name};

pub fn find(kw: &str, is_list: bool) -> Result<()> {
    let dir = std::fs::read_dir(APT_LIST_DISTS)?;

    let arch = get_arch_name().context("Can not get ARCH!")?;

    let pattern = if is_list {
        format!(r"^\s*(.*?)\s+((?:\S*[,/])?{}(?:,\S*|))\s*$", kw)
    } else {
        format!(r"^(.*?{}(?:.*[^\s])?)\s+(\S+)\s*$", kw)
    };

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

    for i in res {
        println!("{i}");
    }

    Ok(())
}
