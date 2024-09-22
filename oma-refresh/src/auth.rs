use std::{
    fs::{self, read_dir},
    io::{self},
    path::{Path, PathBuf},
    str::FromStr,
};

use thiserror::Error;

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

pub struct AuthConfig {
    pub inner: Vec<AuthConfigEntry>,
}

pub struct AuthConfigEntry {
    pub host: String,
    pub user: String,
    pub password: String,
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
                    return Err(AuthConfigError::MissingEntry("login"));
                };

                password = Some(p);
                continue;
            }
        }

        if host.is_none() {
            return Err(AuthConfigError::MissingEntry("machine"));
        }

        if login.is_none() {
            return Err(AuthConfigError::MissingEntry("login"));
        }

        if password.is_none() {
            return Err(AuthConfigError::MissingEntry("password"));
        }

        Ok(Self {
            host: host.unwrap().to_string(),
            user: login.unwrap().to_string(),
            password: password.unwrap().to_string(),
        })
    }
}

impl AuthConfig {
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

            for i in s.lines().filter(|x| !x.starts_with('#')) {
                let entry = AuthConfigEntry::from_str(i)?;
                v.push(entry);
            }
        }

        Ok(Self { inner: v })
    }

    pub fn find(&self, machine: &str) -> Option<&AuthConfigEntry> {
        self.inner.iter().find(|x| x.host == machine)
    }
}
