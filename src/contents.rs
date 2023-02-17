use std::io::BufReader;

use anyhow::{Context, Result};
use grep::{
    regex::RegexMatcher,
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

    let matcher = RegexMatcher::new(&pattern)?;

    for i in dir.flatten() {
        let filename = i
            .file_name()
            .to_str()
            .context("Can not get filename!")?
            .to_string();

        if filename.ends_with(&format!("_Contents-{arch}"))
            || filename.ends_with(&format!("_Contents-all"))
        {
            let f = std::fs::File::open(i.path())?;
            let reader = BufReader::new(f);
            Searcher::new().search_reader(
                &matcher,
                reader,
                UTF8(|_, line| {
                    let mut split = line.split_whitespace();
                    let file = split.next();
                    let package = split.next().and_then(|x| x.split('/').nth(1));
                    if file.and(package).is_some() {
                        println!("{}: {}", package.unwrap(), file.unwrap());
                    }

                    Ok(true)
                }),
            )?;
        }
    }

    Ok(())
}
