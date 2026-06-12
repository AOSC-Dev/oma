use std::{
    fs::read_dir,
    io,
    path::{Path, PathBuf},
};

pub use netrc::Authenticator;
use netrc::Netrc;
use thiserror::Error;
use url::Url;

pub mod reqwuest;

#[derive(Debug, Error)]
pub enum AuthConfigError {
    #[error("Failed to read dir: {path}")]
    ReadDir { path: PathBuf, err: io::Error },
    #[error("Failed to read dir entry")]
    DirEntry(#[source] io::Error),
    #[error("Failed to open file: {path}")]
    OpenFile {
        path: PathBuf,
        #[source]
        err: io::Error,
    },
    #[error(transparent)]
    ParseError(#[from] netrc::Error),
}

#[derive(Debug, Eq, Clone, PartialEq)]
pub struct AuthUrl {
    schema: Option<String>,
    host: String,
    port: Option<u16>,
    path: String,
}

impl AuthUrl {
    fn match_score(&self, request_url: &Url) -> Option<usize> {
        if let Some(ref conf_schema) = self.schema {
            if conf_schema.to_lowercase() != request_url.scheme().to_lowercase() {
                return None;
            }
        } else {
            // If protocol is not specified, the entry only matches https and tor+https
            let req_schema = request_url.scheme().to_lowercase();
            if req_schema != "https" && req_schema != "tor+https" {
                return None;
            }
        }

        let req_host = request_url.host_str()?.to_lowercase();
        if self.host != req_host {
            return None;
        }

        if let Some(conf_port) = self.port {
            let req_port = request_url.port().unwrap_or(match request_url.scheme() {
                "http" => 80,
                "https" => 443,
                _ => return None,
            });
            if conf_port != req_port {
                return None;
            }
        }

        // A machine token with a path matches if the path in the URI starts with the path given in the token.
        let req_path = request_url.path();
        if req_path.starts_with(&self.path) {
            Some(self.path.len())
        } else {
            None
        }
    }

    fn match_score_loose(&self, request_url: &AuthUrl) -> Option<usize> {
        if self.host != request_url.host {
            return None;
        }

        if let Some(conf_port) = self.port {
            if Some(conf_port) != request_url.port {
                return None;
            }
        }

        if request_url.path.starts_with(&self.path) {
            Some(self.path.len())
        } else {
            None
        }
    }
}

impl From<&str> for AuthUrl {
    fn from(value: &str) -> Self {
        if let Ok(url) = Url::parse(value) {
            return AuthUrl {
                schema: Some(url.scheme().to_string()),
                host: url.host_str().unwrap_or_default().to_lowercase(),
                port: url.port(),
                path: url.path().to_string(),
            };
        }

        let mut remaining = value;
        let mut schema = None;

        if let Some(pos) = remaining.find("://") {
            schema = Some(remaining[..pos].to_string());
            remaining = &remaining[pos + 3..];
        }

        let (host_port, path_part) = match remaining.find('/') {
            Some(pos) => (&remaining[..pos], &remaining[pos..]),
            None => (remaining, ""),
        };

        let (host, port) = match host_port.get(..host_port.rfind(':').unwrap_or(host_port.len())) {
            Some(h) if host_port.contains(':') => {
                let p_str = &host_port[host_port.rfind(':').unwrap() + 1..];
                (h, p_str.parse::<u16>().ok())
            }
            _ => (host_port, None),
        };

        let mut path = path_part.to_string();
        if path.is_empty() && value.contains('/') {
            path.push('/');
        }

        AuthUrl {
            schema,
            host: host.to_lowercase(),
            port,
            path,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AuthConfig(pub Vec<(AuthUrl, Authenticator)>);

impl AuthConfig {
    pub fn system(sysroot: impl AsRef<Path>) -> Result<Self, AuthConfigError> {
        Self::from_path(sysroot.as_ref().join("etc/apt"))
    }

    pub fn from_path(p: impl AsRef<Path>) -> Result<Self, AuthConfigError> {
        let mut v = vec![];
        let base_path = p.as_ref();

        let auth_conf = base_path.join("auth.conf");
        if auth_conf.exists() {
            let config = Netrc::from_file(&auth_conf).map_err(|e| AuthConfigError::OpenFile {
                path: auth_conf.clone(),
                err: io::Error::new(io::ErrorKind::Other, e),
            })?;
            v.push(config);
        }

        let auth_conf_d = base_path.join("auth.conf.d");
        if auth_conf_d.exists() {
            let entries = read_dir(&auth_conf_d).map_err(|e| AuthConfigError::ReadDir {
                path: auth_conf_d.clone(),
                err: e,
            })?;

            for entry in entries {
                let entry = entry.map_err(AuthConfigError::DirEntry)?;
                let path = entry.path();

                if !path.is_file() {
                    continue;
                }

                let config = Netrc::from_file(&path).map_err(|e| AuthConfigError::OpenFile {
                    path: path.clone(),
                    err: io::Error::new(io::ErrorKind::Other, e),
                })?;
                v.push(config);
            }
        }

        let v = v
            .into_iter()
            .flat_map(|x| x.hosts)
            .map(|(k, v)| (AuthUrl::from(k.as_str()), v))
            .collect::<Vec<_>>();

        Ok(Self(v))
    }

    pub fn find(&self, url: &Url) -> Option<&Authenticator> {
        self.0
            .iter()
            .filter_map(|(config_url, auth)| config_url.match_score(url).map(|score| (score, auth)))
            .max_by_key(|(score, _)| *score)
            .map(|(_, auth)| auth)
    }

    pub fn find_str(&self, url: &str) -> Option<&Authenticator> {
        if let Ok(parsed_url) = Url::parse(url) {
            return self.find(&parsed_url);
        }

        let request_auth_url = AuthUrl::from(url);

        self.0
            .iter()
            .filter_map(|(config_url, auth)| {
                config_url
                    .match_score_loose(&request_auth_url)
                    .map(|score| (score, auth))
            })
            .max_by_key(|(score, _)| *score)
            .map(|(_, auth)| auth)
    }
}
