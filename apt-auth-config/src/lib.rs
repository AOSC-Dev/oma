mod parser;

use std::{
    fs::{self, read_dir},
    io::{self},
    path::{Path, PathBuf},
    str::FromStr,
};

use parser::{line, multiline};
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
    #[error("Parse failed, unknown line: {0}")]
    ParseError(String),
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
        let mut s = s;
        let parse = line(&mut s).map_err(|e| AuthConfigError::ParseError(e.to_string()))?;

        Ok(parse_entry_inner(parse))
    }
}

impl FromStr for AuthConfig {
    type Err = AuthConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = s;
        let parse = multiline(&mut s).map_err(|e| AuthConfigError::ParseError(e.to_string()))?;
        let mut res = vec![];

        for r in parse {
            res.push(parse_entry_inner(r));
        }

        Ok(AuthConfig { inner: res })
    }
}

fn parse_entry_inner(input: Vec<(&str, &str)>) -> AuthConfigEntry {
    let mut machine = None;
    let mut login = None;
    let mut password = None;

    for i in input {
        match i.0 {
            "machine" => machine = Some(i.1),
            "login" => login = Some(i.1),
            "password" => password = Some(i.1),
            x => panic!("unexcept {x}"),
        }
    }

    AuthConfigEntry {
        host: machine.unwrap().into(),
        user: login.unwrap().into(),
        password: password.unwrap().into(),
    }
}

impl AuthConfig {
    /// Read system auth.conf.d config (/etc/apt/auth.conf.d)
    pub fn system(sysroot: impl AsRef<Path>) -> Result<Self, AuthConfigError> {
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

            let config: AuthConfig = s.parse()?;
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

        self.inner.iter().find_map(|x| {
            let mut host = x.host.to_string();
            while host.ends_with('/') {
                host.pop();
            }

            let mut url = url.to_string();
            while url.ends_with('/') {
                url.pop();
            }

            if host == url {
                Some(x)
            } else {
                None
            }
        })
    }

    pub fn find_package_url(&self, url: &str) -> Option<&AuthConfigEntry> {
        let url = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))
            .unwrap_or(url);

        debug!("auth find package url is: {}", url);

        self.inner.iter().find_map(|x| {
            if url.starts_with(x.host.as_ref()) {
                Some(x)
            } else {
                None
            }
        })
    }
}
