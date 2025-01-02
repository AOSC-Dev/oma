use std::{
    fs::read_dir,
    io::{self},
    path::{Path, PathBuf},
};

use ahash::HashMap;
use netrc::{Authenticator, Netrc};
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
    #[error(transparent)]
    ParseError(#[from] netrc::Error),
}

#[derive(Debug, Eq)]
pub struct AuthUrl {
    schema: Option<String>,
    host_and_path: String,
}

impl From<&str> for AuthUrl {
    fn from(value: &str) -> Self {
        let split_url = value
            .split_once("https://")
            .or_else(|| value.split_once("http://"));

        let (schema, host_and_path) = if let Some((schema, host_and_path)) = split_url {
            (Some(schema.to_string()), host_and_path.to_string())
        } else {
            (None, value.to_string())
        };

        Self {
            schema,
            host_and_path,
        }
    }
}

impl AuthUrl {
    fn drop_suffix(&self) -> &str {
        let mut res = self.host_and_path.as_str();

        while let Some(x) = res.strip_suffix('/') {
            res = x;
        }

        res
    }
}

impl From<&Url> for AuthUrl {
    fn from(value: &Url) -> Self {
        let mut host_and_path = String::new();
        let schema = value.scheme().to_string();

        if let Some(host) = value.host_str() {
            host_and_path.push_str(host);
        }

        host_and_path.push_str(value.path());

        AuthUrl {
            schema: Some(schema),
            host_and_path,
        }
    }
}

impl PartialEq for AuthUrl {
    fn eq(&self, other: &Self) -> bool {
        if let Some((a, b)) = self.schema.as_ref().zip(other.schema.as_ref()) {
            return a == b && self.drop_suffix() == other.drop_suffix();
        }

        self.drop_suffix() == other.drop_suffix()
    }
}

#[derive(Debug)]
pub struct AuthConfig(pub HashMap<String, Authenticator>);

impl AuthConfig {
    /// Read system auth.conf.d config (/etc/apt/auth.conf.d)
    pub fn system(etc_apt_dir: impl AsRef<Path>) -> Result<Self, AuthConfigError> {
        Self::from_path(etc_apt_dir)
    }

    pub fn from_path(p: impl AsRef<Path>) -> Result<Self, AuthConfigError> {
        let mut v = vec![];

        let auth_conf = p.as_ref().join("auth.conf");

        if auth_conf.exists() {
            let config = Netrc::from_file(&auth_conf)?;
            v.push(config);
        }

        let auth_conf_d = p.as_ref().join("auth.conf.d");

        if auth_conf_d.exists() {
            for i in read_dir(p.as_ref()).map_err(|e| AuthConfigError::ReadDir {
                path: p.as_ref().to_path_buf(),
                err: e,
            })? {
                let i = i.map_err(AuthConfigError::DirEntry)?;

                if !i.path().is_file() {
                    continue;
                }

                let config = Netrc::from_file(&i.path())?;
                v.push(config);
            }
        }

        let v = v
            .into_iter()
            .flat_map(|x| x.hosts)
            .collect::<HashMap<_, _>>();

        Ok(Self(v))
    }
}
