use std::io::{BufRead, BufReader};

use anyhow::{Context, Result};

use crate::{db::APT_LIST_DISTS, utils::get_arch_name};

pub struct Contents(Vec<(String, String)>);

impl Contents {
    pub fn new() -> Result<Self> {
        let dir = std::fs::read_dir(APT_LIST_DISTS)?;

        let arch = get_arch_name().context("Can not get ARCH!")?;

        let mut res = Vec::new();

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

                for i in reader.lines().flatten() {
                    let mut split = i.split_whitespace();
                    let file = split.next().context("Can not get file field!")?;
                    let packages = split.next().context("Can not get packages field!")?;
                    res.push((file.to_owned(), packages.to_owned()));
                }
            }
        }

        res.sort_by(|a, b| a.1.cmp(&b.1));

        let mut res_2 = vec![];

        for (file, packages) in res {
            let split = packages.split(',');

            let mut has_value = false;

            for i in split {
                has_value = true;
                res_2.push((
                    i.to_owned().split('/').nth(1).unwrap_or(i).to_string(),
                    file.clone(),
                ));
            }

            if !has_value {
                res_2.push((
                    file.to_owned()
                        .split('/')
                        .nth(1)
                        .unwrap_or(&file)
                        .to_string(),
                    packages,
                ))
            }
        }

        Ok(Self(res_2))
    }

    pub fn list_package(&self, pkg: &str) {
        let res = self
            .0
            .iter()
            .filter(|(x, _)| x == pkg);

        for (package, file) in res {
            println!("{package}: {file}");
        }
    }
}

#[test]
fn test() {
    let contents = Contents::new().unwrap();
    contents.list_package("apt")
}
