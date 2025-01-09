use std::{
    fs::read_dir,
    io::{self},
    path::{Path, PathBuf},
};

pub use netrc::Authenticator;
use netrc::Netrc;
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

impl AuthUrl {
    fn drop_suffix(&self) -> &str {
        let mut res = self.host_and_path.as_str();

        while let Some(x) = res.strip_suffix('/') {
            res = x;
        }

        res
    }
}

impl From<&str> for AuthUrl {
    fn from(value: &str) -> Self {
        if let Ok(url) = Url::parse(value) {
            AuthUrl {
                schema: Some(url.scheme().to_string()),
                host_and_path: {
                    let mut s = String::new();
                    if let Some(host) = url.host_str() {
                        s.push_str(host);
                    }
                    s.push_str(url.path());
                    s
                },
            }
        } else {
            AuthUrl {
                schema: None,
                host_and_path: value.to_string(),
            }
        }
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
pub struct AuthConfig(pub Vec<(AuthUrl, Authenticator)>);

impl AuthConfig {
    /// Read system auth.conf.d config (/etc/apt/auth.conf.d)
    pub fn system(sysroot: impl AsRef<Path>) -> Result<Self, AuthConfigError> {
        Self::from_path(sysroot.as_ref().join("etc/apt"))
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
            for i in read_dir(auth_conf_d).map_err(|e| AuthConfigError::ReadDir {
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
            .map(|x| (AuthUrl::from(x.0.as_str()), x.1))
            .collect::<Vec<_>>();

        Ok(Self(v))
    }

    pub fn find(&self, url: &str) -> Option<&Authenticator> {
        self.0
            .iter()
            .find(|x| AuthUrl::from(url) == x.0)
            .map(|x| &x.1)
    }
}
