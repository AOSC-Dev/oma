use std::{
    fs, io,
    path::{Path, PathBuf},
};

use ahash::HashMap;
use serde::Deserialize;
use snafu::{ResultExt, Snafu};

#[derive(Debug, Deserialize)]
pub struct MirrorsConfigTemplate {
    pub config: Vec<MirrorConfigTemplate>,
}

#[derive(Debug, Deserialize)]
pub struct MirrorConfigTemplate {
    pub components: Vec<String>,
    #[serde(default, rename = "signed-by")]
    pub signed_by: Vec<String>,
    #[serde(default)]
    pub architectures: Vec<String>,
    #[serde(rename = "always-trusted", default)]
    pub always_trusted: bool,
    #[serde(default = "MirrorsConfigTemplate::default_enabled")]
    pub enabled: bool,
    #[serde(rename = "url-suffix")]
    pub url_suffix: Option<String>,
}

#[derive(Debug, Snafu)]
pub enum TemplateParseError {
    #[snafu(display("Failed to read file: {}", path.display()))]
    ReadFile { source: io::Error, path: PathBuf },
    #[snafu(transparent)]
    Parse { source: toml::de::Error },
    #[snafu(display("The ID of your custom mirror `{name}' conflicts with an existing mirror"))]
    ConflictName { name: Box<str> },
}

impl MirrorsConfigTemplate {
    const fn default_enabled() -> bool {
        true
    }

    pub fn parse_from_file(path: impl AsRef<Path>) -> Result<Self, TemplateParseError> {
        let s = fs::read(path.as_ref()).context(ReadFileSnafu {
            path: path.as_ref().to_path_buf(),
        })?;

        Self::parse_from_slice(&s)
    }

    pub fn parse_from_slice(slice: &[u8]) -> Result<Self, TemplateParseError> {
        Ok(toml::from_slice(slice)?)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct MirrorsConfig(pub HashMap<Box<str>, MirrorConfig>);

#[derive(Debug, Deserialize, Clone)]
pub struct MirrorConfig {
    pub description: HashMap<String, String>,
    pub url: Box<str>,
}

impl MirrorsConfig {
    pub fn parse_from_file(path: impl AsRef<Path>) -> Result<Self, TemplateParseError> {
        let s = fs::read(path.as_ref()).context(ReadFileSnafu {
            path: path.as_ref().to_path_buf(),
        })?;

        Self::parse_from_slice(&s)
    }

    pub fn parse_from_slice(slice: &[u8]) -> Result<Self, TemplateParseError> {
        Ok(toml::from_slice(slice)?)
    }
}
