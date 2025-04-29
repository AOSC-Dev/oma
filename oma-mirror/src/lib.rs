use std::{
    fs,
    io::{self},
    path::{Path, PathBuf},
};

use ahash::HashMap;
use indexmap::{IndexMap, indexmap};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use tracing::debug;

#[derive(Debug, Serialize, Deserialize)]
struct Status {
    branch: Box<str>,
    component: Vec<Box<str>>,
    mirror: IndexMap<Box<str>, Box<str>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Branch {
    desc: Box<str>,
    suites: Vec<Box<str>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Mirror {
    pub desc: Box<str>,
    pub url: Box<str>,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            branch: Box::from("stable"),
            component: vec![Box::from("main")],
            mirror: indexmap! { Box::from("origin") => Box::from("https://repo.aosc.io/") },
        }
    }
}

#[derive(Debug, Snafu)]
pub enum MirrorError {
    #[snafu(display("Failed to read file: {}", path.display()))]
    ReadFile { path: PathBuf, source: io::Error },
    #[snafu(display("Failed to parse file: {}", path.display()))]
    ParseJson {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[snafu(display("Failed to parse file: {}", path.display()))]
    ParseYaml {
        path: PathBuf,
        source: serde_yaml::Error,
    },
    #[snafu(display("mirror does not exist in mirrors file: {mirror_name}"))]
    MirrorNotExist { mirror_name: Box<str> },
    #[snafu(display("Serialize struct failed"))]
    SerializeJson { source: serde_json::Error },
    #[snafu(display("Failed to write to file"))]
    WriteFile { path: PathBuf, source: io::Error },
    #[snafu(display("Failed to create status file: {}", path.display()))]
    CreateFile { path: PathBuf, source: io::Error },
}

pub struct MirrorManager {
    status: Status,
    // branches_data: OnceCell<HashMap<Box<str>, Branch>>,
    // components_data: OnceCell<HashMap<Box<str>, Box<str>>>,
    mirrors_data: OnceCell<HashMap<Box<str>, Mirror>>,
    status_file_path: PathBuf,
    mirrors_file_path: PathBuf,
    apt_status_file: PathBuf,
    apt_status_file_new: PathBuf,
}

impl MirrorManager {
    pub fn new(rootfs: impl AsRef<Path>) -> Result<Self, MirrorError> {
        let status_file_path = rootfs.as_ref().join("var/lib/apt/gen/status.json");
        let mirrors_file_path = rootfs
            .as_ref()
            .join("usr/share/distro-repository-data/mirrors.yml");
        let apt_status_file = rootfs.as_ref().join("etc/apt/sources.list");
        let apt_status_file_new = rootfs.as_ref().join("etc/apt/sources.list.d/aosc.sources");

        let status: Status = if status_file_path.is_file() {
            let f = fs::read(&status_file_path).context(ReadFileSnafu {
                path: status_file_path.to_path_buf(),
            })?;
            match serde_json::from_slice(&f) {
                Ok(status) => status,
                Err(e) => {
                    debug!("{e}, creating new ...");
                    create_default_status(&status_file_path)?
                }
            }
        } else {
            create_default_status(&status_file_path)?
        };

        Ok(Self {
            status,
            // branches_data: OnceCell::new(),
            // components_data: OnceCell::new(),
            mirrors_data: OnceCell::new(),
            status_file_path,
            mirrors_file_path,
            apt_status_file,
            apt_status_file_new,
        })
    }

    // fn try_branches_data(&self) -> Result<&HashMap<Box<str>, Branch>, MirrorError> {
    //     self.branches_data
    //         .get_or_try_init(|| -> Result<HashMap<Box<str>, Branch>, MirrorError> {
    //             let f = fs::read(Self::BRANCHES_FILE).context(ReadFileSnafu {
    //                 path: Self::BRANCHES_FILE,
    //             })?;

    //             let branches_data: HashMap<Box<str>, Branch> =
    //                 serde_json::from_slice(&f).context(ParseSnafu {
    //                     path: Self::BRANCHES_FILE,
    //                 })?;

    //             Ok(branches_data)
    //         })
    // }

    // fn branches_data(&self) -> &HashMap<Box<str>, Branch> {
    //     self.branches_data.get().unwrap()
    // }

    // fn try_comps(&self) -> Result<&HashMap<Box<str>, Box<str>>, MirrorError> {
    //     self.components_data.get_or_try_init(
    //         || -> Result<HashMap<Box<str>, Box<str>>, MirrorError> {
    //             let f = fs::read(Self::COMPS_FILE).context(ReadFileSnafu {
    //                 path: Self::COMPS_FILE,
    //             })?;

    //             let comps: HashMap<Box<str>, Box<str>> =
    //                 serde_json::from_slice(&f).context(ParseSnafu {
    //                     path: Self::COMPS_FILE,
    //                 })?;

    //             Ok(comps)
    //         },
    //     )
    // }

    // fn comps(&self) -> &HashMap<Box<str>, Box<str>> {
    //     self.components_data.get().unwrap()
    // }

    fn try_mirrors(&self) -> Result<&HashMap<Box<str>, Mirror>, MirrorError> {
        self.mirrors_data
            .get_or_try_init(|| -> Result<HashMap<Box<str>, Mirror>, MirrorError> {
                let f = fs::read(&self.mirrors_file_path).context(ReadFileSnafu {
                    path: self.mirrors_file_path.to_path_buf(),
                })?;

                let mirrors: HashMap<Box<str>, Mirror> =
                    serde_yaml::from_slice(&f).context(ParseYamlSnafu {
                        path: self.mirrors_file_path.to_path_buf(),
                    })?;

                Ok(mirrors)
            })
    }

    pub fn set(&mut self, mirror_names: &[&str]) -> Result<(), MirrorError> {
        let mirrors = self.try_mirrors()?;

        for i in mirror_names {
            if !mirrors.contains_key(*i) {
                return Err(MirrorError::MirrorNotExist {
                    mirror_name: Box::from(*i),
                });
            }
        }

        self.status.mirror.clear();
        for i in mirror_names {
            self.add(i)?;
        }

        Ok(())
    }

    pub fn add(&mut self, mirror_name: &str) -> Result<bool, MirrorError> {
        if self.status.mirror.contains_key(mirror_name) {
            return Ok(false);
        }

        let mirrors = self.try_mirrors()?;

        let mirror_url = if let Some(mirror) = mirrors.get(mirror_name) {
            mirror.url.clone()
        } else {
            return Err(MirrorError::MirrorNotExist {
                mirror_name: mirror_name.into(),
            });
        };

        self.status.mirror.insert(mirror_name.into(), mirror_url);

        Ok(true)
    }

    pub fn remove(&mut self, mirror_name: &str) -> bool {
        if !self.status.mirror.contains_key(mirror_name) {
            return false;
        }

        self.status.mirror.shift_remove(mirror_name);

        true
    }

    pub fn mirrors_iter(&self) -> Result<impl Iterator<Item = (&str, &Mirror)>, MirrorError> {
        let mirrors = self.try_mirrors()?;
        let iter = mirrors.iter().map(|x| (x.0.as_ref(), x.1));

        Ok(iter)
    }

    pub fn enabled_mirrors(&self) -> &IndexMap<Box<str>, Box<str>> {
        &self.status.mirror
    }

    pub fn set_order(&mut self, order: &[usize]) {
        let mut new = IndexMap::new();
        for i in order {
            let (k, v) = self.status.mirror.get_index(*i).unwrap();
            new.insert(k.to_owned(), v.to_owned());
        }

        self.status.mirror = new;
    }

    pub fn write_status(&self, custom_mirror_tips: Option<&str>) -> Result<(), MirrorError> {
        fs::write(
            &self.status_file_path,
            serde_json::to_vec(&self.status).context(SerializeJsonSnafu)?,
        )
        .context(WriteFileSnafu {
            path: self.status_file_path.to_path_buf(),
        })?;

        let mut result = String::new();

        let tips = custom_mirror_tips.unwrap_or("# Generate by oma-mirror, DO NOT EDIT!");
        result.push_str(tips);
        result.push('\n');

        let is_deb822 = if self.apt_status_file_new.exists() {
            if self.apt_status_file.exists() {
                let bak = self.apt_status_file.with_file_name("sources.list.bak");
                fs::rename(&self.apt_status_file, &bak).context(WriteFileSnafu { path: bak })?;
            }

            true
        } else {
            false
        };

        for (_, url) in &self.status.mirror {
            if !is_deb822 {
                result.push_str("deb ");
                result.push_str(url);

                if !url.ends_with('/') {
                    result.push('/');
                }

                result.push_str("debs");
                result.push(' ');
                result.push_str(&self.status.branch);
                result.push(' ');
                result.push_str(&self.status.component.join(" "));
                result.push('\n');
            } else {
                result.push_str(&format!(
                    "Types: deb\nURIs: {}debs\nSuites: {}\nComponents: {}\n\n",
                    url,
                    &self.status.branch,
                    &self.status.component.join(" ")
                ));
            }
        }

        let path = if is_deb822 {
            &self.apt_status_file_new
        } else {
            &self.apt_status_file
        };

        fs::write(path, result).context(WriteFileSnafu {
            path: path.display().to_string(),
        })?;

        Ok(())
    }
}

fn create_default_status(path: &Path) -> Result<Status, MirrorError> {
    debug!("Creating status file ... ");
    fs::create_dir_all(path.parent().unwrap()).context(CreateFileSnafu {
        path: path.to_path_buf(),
    })?;

    let mut f = fs::File::create(path).context(CreateFileSnafu {
        path: path.to_path_buf(),
    })?;

    let status = Status::default();
    serde_json::to_writer(&mut f, &status).unwrap();

    Ok(status)
}
