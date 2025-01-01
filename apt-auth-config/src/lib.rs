mod parser;

use std::{
    fs::{self, read_dir},
    io::{self},
    path::{Path, PathBuf},
    str::FromStr,
};

use ahash::HashMap;
use parser::{line, multiline};
use thiserror::Error;
use url::Url;

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
pub struct AuthConfig(pub HashMap<Box<str>, AuthConfigEntry>);

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

        Ok(parse_entry_inner(parse).1)
    }
}

impl FromStr for AuthConfig {
    type Err = AuthConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = s;
        let parse = multiline(&mut s).map_err(|e| AuthConfigError::ParseError(e.to_string()))?;
        let mut res = HashMap::with_hasher(ahash::RandomState::new());

        for r in parse {
            let (k, v) = parse_entry_inner(r);
            res.insert(k, v);
        }

        Ok(AuthConfig(res))
    }
}

fn parse_entry_inner(input: Vec<(&str, &str)>) -> (Box<str>, AuthConfigEntry) {
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

    let machine: Box<str> = machine.unwrap().into();

    (
        machine.clone(),
        AuthConfigEntry {
            host: machine,
            user: login.unwrap().into(),
            password: password.unwrap().into(),
        },
    )
}

impl AuthConfig {
    /// Read system auth.conf.d config (/etc/apt/auth.conf.d)
    pub fn system(sysroot: impl AsRef<Path>) -> Result<Self, AuthConfigError> {
        let p = sysroot.as_ref().join("etc/apt/auth.conf.d");
        Self::from_path(p)
    }

    pub fn from_path(p: impl AsRef<Path>) -> Result<Self, AuthConfigError> {
        let mut v = HashMap::with_hasher(ahash::RandomState::new());

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
            v.extend(config.0);
        }

        Ok(Self(v))
    }

    pub fn get_match_auth(&self, url: Url) -> Option<&AuthConfigEntry> {
        let host = url.host_str()?;
        let path = url.path();
        let url_without_schema = [host, path].concat();

        self.0
            .values()
            .find(|x| url_without_schema.starts_with(&*x.host))
    }
}
