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

#[derive(Debug, PartialEq, Eq)]
pub struct AuthUrl {
    schema: Option<String>,
    host: String,
    port: Option<u16>,
    path: Option<String>,
}

impl From<&str> for AuthUrl {
    fn from(value: &str) -> Self {
        // extract schema
        let (schema, host_port_path) = value.split_once("://").unzip();
        let host_port_path = host_port_path.unwrap_or(value);

        // extract path
        let (host_port, mut path) = host_port_path.split_once("/").unzip();
        let host_port = host_port.unwrap_or(host_port_path);

        // extract port
        let (host, port) = host_port.split_once(":").unzip();
        let host = host.unwrap_or(host_port);
        let (host, port) = port
            .and_then(|port| port.parse::<u16>().ok())
            .map(|port| (host, Some(port)))
            .unwrap_or((host_port, None));

        // strip suffix
        if let Some(path) = &mut path {
            while let Some(x) = path.strip_suffix('/') {
                *path = x;
            }
        }

        Self {
            schema: schema.map(|schema| schema.to_string()),
            host: host.to_string(),
            port,
            path: path.map(|path| path.to_string()),
        }
    }
}

impl From<&Url> for AuthUrl {
    fn from(value: &Url) -> Self {
        let mut path = value.path();
        while let Some(x) = path.strip_suffix('/') {
            path = x;
        }

        AuthUrl {
            schema: Some(value.scheme().to_string()),
            host: value
                .host()
                .map(|host| host.to_string())
                .unwrap_or_default(),
            port: value.port_or_known_default(),
            path: Some(path.to_string()),
        }
    }
}

impl AuthUrl {
    fn test(&self, other: &Self) -> bool {
        if let Some(a) = &other.schema {
            if let Some(b) = &self.schema {
                if a != b {
                    return false;
                }
            } else if a != "https" && a != "tor+https" {
                return false;
            }
        }
        if self.port.is_some() && other.port.is_some() && self.port != other.port {
            return false;
        }
        if self.path.is_some() && other.path.is_some() && self.path != other.path {
            return false;
        }
        self.host == other.host
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
            .find(|(x, _)| x.test(&AuthUrl::from(url)))
            .map(|x| &x.1)
    }
}

#[cfg(test)]
mod test {
    use crate::*;

    #[test]
    fn test_auth_parse() {
        assert_eq!(
            AuthUrl::from("localhost"),
            AuthUrl {
                schema: None,
                host: "localhost".to_string(),
                port: None,
                path: None
            }
        );
        assert_eq!(
            AuthUrl::from("localhost:1234"),
            AuthUrl {
                schema: None,
                host: "localhost".to_string(),
                port: Some(1234),
                path: None
            }
        );
        assert_eq!(
            AuthUrl::from("ftp://localhost"),
            AuthUrl {
                schema: Some("ftp".to_string()),
                host: "localhost".to_string(),
                port: None,
                path: None
            }
        );
        assert_eq!(
            AuthUrl::from("ftp://localhost:123/something"),
            AuthUrl {
                schema: Some("ftp".to_string()),
                host: "localhost".to_string(),
                port: Some(123),
                path: Some("something".to_string())
            }
        );
        assert_eq!(
            AuthUrl::from("ftp://localhost:123/something///"),
            AuthUrl {
                schema: Some("ftp".to_string()),
                host: "localhost".to_string(),
                port: Some(123),
                path: Some("something".to_string())
            }
        );
    }

    #[test]
    fn test_auth_match() {
        assert!(AuthUrl::from("localhost").test(&AuthUrl::from("localhost")));
        assert!(!AuthUrl::from("localhost").test(&AuthUrl::from("ten.avaj")));

        assert!(AuthUrl::from("localhost").test(&AuthUrl::from("https://localhost")));
        assert!(AuthUrl::from("https://localhost").test(&AuthUrl::from("https://localhost")));
        assert!(AuthUrl::from("localhost").test(&AuthUrl::from("tor+https://localhost")));
        assert!(!AuthUrl::from("localhost")
            .test(&AuthUrl::from("aosctexttransferprotocol://localhost")));
        assert!(AuthUrl::from("attp://localhost").test(&AuthUrl::from("attp://localhost")));
        assert!(!AuthUrl::from("attp://localhost").test(&AuthUrl::from("http://localhost")));

        assert!(AuthUrl::from("localhost").test(&AuthUrl::from("https://localhost:456")));
        assert!(!AuthUrl::from("localhost:123").test(&AuthUrl::from("https://localhost:456")));
        assert!(AuthUrl::from("localhost:123").test(&AuthUrl::from("https://localhost:123")));

        assert!(
            AuthUrl::from("localhost:123/foo").test(&AuthUrl::from("https://localhost:123/foo"))
        );
        assert!(
            AuthUrl::from("localhost:123/bar").test(&AuthUrl::from("https://localhost:123/bar"))
        );
        assert!(
            !AuthUrl::from("localhost:123/foo").test(&AuthUrl::from("https://localhost:123/bar"))
        );
        assert!(AuthUrl::from("localhost:123").test(&AuthUrl::from("https://localhost:123/bar")));
        assert!(AuthUrl::from("localhost:123").test(&AuthUrl::from("https://localhost:123/foo")));
    }
}
