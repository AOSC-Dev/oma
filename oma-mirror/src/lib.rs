pub mod parser;

use std::{
    borrow::Cow,
    fs,
    io::{self},
    path::{Path, PathBuf},
};

use indexmap::{IndexMap, indexmap};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use tracing::debug;

use crate::parser::{
    MirrorConfig, MirrorConfigTemplate, MirrorsConfig, MirrorsConfigTemplate, TemplateParseError,
};

#[derive(Debug, Serialize, Deserialize)]
struct Status {
    mirror: IndexMap<Box<str>, Box<str>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Branch {
    desc: Box<str>,
    suites: Vec<Box<str>>,
}

impl Default for Status {
    fn default() -> Self {
        Self {
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
    #[snafu(display("Not allow apply empty mirrors settings"))]
    ApplyEmptySettings,
    #[snafu(display("Parse Error"))]
    ParseConfig { source: parser::TemplateParseError },
}

pub struct MirrorManager {
    status: Status,
    mirrors_data: OnceCell<MirrorsConfig>,
    status_file_path: PathBuf,
    mirrors_file_path: PathBuf,
    apt_status_file: PathBuf,
    apt_status_file_new: PathBuf,
    template: PathBuf,
    custom_mirrors_file_path: PathBuf,
}

impl MirrorManager {
    pub fn new(rootfs: impl AsRef<Path>) -> Result<Self, MirrorError> {
        let status_file_path = rootfs.as_ref().join("var/lib/apt/gen/status.json");
        let mirrors_file_path = rootfs
            .as_ref()
            .join("usr/share/repository-data/mirrors.toml");
        let custom_mirrors_file_path = rootfs.as_ref().join("etc/repository-data/mirrors.toml");
        let apt_status_file = rootfs.as_ref().join("etc/apt/sources.list");
        let apt_status_file_new = rootfs.as_ref().join("etc/apt/sources.list.d/aosc.sources");
        let template = rootfs
            .as_ref()
            .join("usr/share/repository-data/template.toml");

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
            template,
            custom_mirrors_file_path,
        })
    }

    fn try_mirrors(&self) -> Result<&MirrorsConfig, MirrorError> {
        self.mirrors_data
            .get_or_try_init(|| -> Result<MirrorsConfig, MirrorError> {
                let mut m = MirrorsConfig::parse_from_file(&self.mirrors_file_path)
                    .context(ParseConfigSnafu)?;

                if self.custom_mirrors_file_path.exists() {
                    let custom = MirrorsConfig::parse_from_file(&self.custom_mirrors_file_path)
                        .context(ParseConfigSnafu)?;

                    for k1 in m.0.keys() {
                        if custom.0.contains_key(k1) {
                            return Err(MirrorError::ParseConfig {
                                source: TemplateParseError::ConflictName {
                                    name: k1.to_owned(),
                                },
                            });
                        }
                    }

                    m.0.extend(custom.0);
                }

                Ok(m)
            })
    }

    pub fn set(&mut self, mirror_names: &[&str]) -> Result<(), MirrorError> {
        if mirror_names.is_empty() {
            return Err(MirrorError::ApplyEmptySettings);
        }

        let mirrors = &self.try_mirrors()?.0;

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

        let mirrors = &self.try_mirrors()?.0;

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

    pub fn remove(&mut self, mirror_name: &str) -> Result<bool, MirrorError> {
        if self.status.mirror.len() == 1 {
            return Err(MirrorError::ApplyEmptySettings);
        }

        if !self.status.mirror.contains_key(mirror_name) {
            return Ok(false);
        }

        self.status.mirror.shift_remove(mirror_name);

        Ok(true)
    }

    pub fn mirrors_iter(&self) -> Result<impl Iterator<Item = (&str, &MirrorConfig)>, MirrorError> {
        let mirrors = &self.try_mirrors()?.0;
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

        let template =
            MirrorsConfigTemplate::parse_from_file(&self.template).context(ParseConfigSnafu)?;
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
            for config in &template.config {
                write_sources_inner(&mut result, is_deb822, url, config, "stable");
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

pub fn write_sources_inner(
    result: &mut String,
    is_deb822: bool,
    url: &str,
    config: &MirrorConfigTemplate,
    branch: &str,
) {
    if !config.enabled {
        return;
    }

    let url = if !url.ends_with('/') {
        format!("{url}/").into()
    } else {
        Cow::Borrowed(url)
    };

    if !is_deb822 {
        result.push_str("deb ");

        let mut opts_str = vec![];

        if config.always_trusted {
            opts_str.push(Cow::Borrowed("trusted=yes"));
        }

        if !config.signed_by.is_empty() {
            opts_str.push(format!("signed-by={}", config.signed_by.join(",")).into());
        }

        if !config.architectures.is_empty() {
            opts_str.push(format!("arch={}", config.architectures.join(",")).into());
        }

        result.push_str(&format!("[{}] ", opts_str.join(" ")));
        result.push_str(&url);
        result.push_str("debs");
        result.push(' ');
        result.push_str(branch);
        result.push(' ');
        result.push_str(&config.components.join(" "));
        result.push('\n');
    } else {
        result.push_str(&format!(
            "Types: deb\nURIs: {url}debs\nSuites: {branch}\nComponents: {}\n",
            config.components.join(" ")
        ));

        if !config.signed_by.is_empty() {
            result.push_str("Signed-By:");

            if config.signed_by.len() == 1 {
                result.push(' ');
                result.push_str(&config.signed_by[0]);
                result.push('\n');
            } else {
                result.push('\n');
                for i in &config.signed_by {
                    result.push_str(&format!(" {i}\n"));
                }
            }
        }

        if config.always_trusted {
            result.push_str("Trusted: yes\n");
        }

        if !config.architectures.is_empty() {
            result.push_str(&format!(
                "Architectures: {}\n",
                config.architectures.join(" ")
            ));
        }

        result.push('\n');
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
