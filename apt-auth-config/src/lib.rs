use std::{
    fs::{self, read_dir},
    io::{self},
    path::{Path, PathBuf},
    str::FromStr,
};

use rustix::process;
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Error)]
pub enum AuthConfigError {
    #[error("Failed to read dir: {path}")]
    ReadDir { path: PathBuf, err: io::Error },
    #[error("Failed to read dir entry")]
    DirEntry(std::io::Error),
    #[error("Failed to open file: {path}")]
    OpenFile { path: PathBuf, err: io::Error },
    #[error("Auth config file missing entry: {0}")]
    MissingEntry(&'static str),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AuthConfig {
    pub inner: Vec<AuthConfigEntry>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AuthConfigEntry {
    pub host: Box<str>,
    pub user: Box<str>,
    pub password: Box<str>,
}

impl FromStr for AuthConfigEntry {
    type Err = AuthConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let entry = s
            .split_ascii_whitespace()
            .filter(|x| !x.starts_with("#"))
            .collect::<Vec<_>>();

        let mut host = None;
        let mut login = None;
        let mut password = None;

        for (i, c) in entry.iter().enumerate() {
            if *c == "machine" {
                let Some(h) = entry.get(i + 1) else {
                    return Err(AuthConfigError::MissingEntry("machine"));
                };

                host = Some(h);
                continue;
            }

            if *c == "login" {
                let Some(l) = entry.get(i + 1) else {
                    return Err(AuthConfigError::MissingEntry("login"));
                };

                login = Some(l);
                continue;
            }

            if *c == "password" {
                let Some(p) = entry.get(i + 1) else {
                    return Err(AuthConfigError::MissingEntry("password"));
                };

                password = Some(p);
                continue;
            }
        }

        let Some(host) = host else {
            return Err(AuthConfigError::MissingEntry("machine"));
        };

        let Some(login) = login else {
            return Err(AuthConfigError::MissingEntry("login"));
        };

        let Some(password) = password else {
            return Err(AuthConfigError::MissingEntry("password"));
        };

        Ok(Self {
            host: (*host).into(),
            user: (*login).into(),
            password: (*password).into(),
        })
    }
}

impl AuthConfig {
    /// Read system auth.conf.d config (/etc/apt/auth.conf.d)
    ///
    /// Note that this function returns empty vector if run as a non-root user.
    pub fn system(sysroot: impl AsRef<Path>) -> Result<Self, AuthConfigError> {
        // 在 auth.conf.d 的使用惯例中
        // 配置文件的权限一般为 600，并且所有者为 root
        // 以普通用户身份下载文件时，会没有权限读取 auth 配置
        // 因此，在以普通用户访问时，不读取 auth 配置
        if !process::geteuid().is_root() {
            return Ok(Self { inner: vec![] });
        }

        let p = sysroot.as_ref().join("etc/apt/auth.conf.d");
        Self::from_path(p)
    }

    pub fn from_path(p: impl AsRef<Path>) -> Result<Self, AuthConfigError> {
        let mut v = vec![];

        for i in read_dir(p.as_ref()).map_err(|e| AuthConfigError::ReadDir {
            path: p.as_ref().to_path_buf(),
            err: e,
        })? {
            let i = i.map_err(AuthConfigError::DirEntry)?;

            if !i.path().is_file() {
                continue;
            }

            let s = fs::read_to_string(i.path()).map_err(|e| AuthConfigError::OpenFile {
                path: i.path().to_path_buf(),
                err: e,
            })?;

            let config = AuthConfig::from_str(&s)?;
            v.extend(config.inner);
        }

        Ok(Self { inner: v })
    }

    pub fn find(&self, url: &str) -> Option<&AuthConfigEntry> {
        let url = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))
            .unwrap_or(url);

        debug!("auth find url is: {}", url);

        self.inner.iter().find(|x| {
            let mut host = x.host.to_string();
            while host.ends_with('/') {
                host.pop();
            }

            let mut url = url.to_string();
            while url.ends_with('/') {
                url.pop();
            }

            host == url
        })
    }

    pub fn find_package_url(&self, url: &str) -> Option<&AuthConfigEntry> {
        let url = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))
            .unwrap_or(url);

        debug!("auth find package url is: {}", url);

        self.inner.iter().find(|x| url.starts_with(x.host.as_ref()))
    }
}

impl FromStr for AuthConfig {
    type Err = AuthConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut v = vec![];

        for i in s.lines().filter(|x| !x.starts_with('#')) {
            let entry = AuthConfigEntry::from_str(i)?;
            v.push(entry);
        }

        Ok(Self { inner: v })
    }
}

#[test]
fn test_config_parser() {
    let config = r#"machine esm.ubuntu.com/apps/ubuntu/ login bearer password qaq  # ubuntu-pro-client
machine esm.ubuntu.com/infra/ubuntu/ login bearer password qaq  # ubuntu-pro-client
"#;

    let config = AuthConfig::from_str(config).unwrap();

    assert_eq!(
        config,
        AuthConfig {
            inner: vec![
                AuthConfigEntry {
                    host: "esm.ubuntu.com/apps/ubuntu/".into(),
                    user: "bearer".into(),
                    password: "qaq".into(),
                },
                AuthConfigEntry {
                    host: "esm.ubuntu.com/infra/ubuntu/".into(),
                    user: "bearer".into(),
                    password: "qaq".into(),
                },
            ]
        }
    );
}
