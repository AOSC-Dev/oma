use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use snafu::{ResultExt, Snafu};

#[derive(Debug, Deserialize)]
pub struct MirrorsConfigTemplate {
    pub config: Vec<MirrorConfig>,
}

#[derive(Debug, Deserialize)]
pub struct MirrorConfig {
    pub components: Vec<String>,
    #[serde(default, rename = "signed-by")]
    pub signed_by: Vec<String>,
    #[serde(default)]
    pub architectures: Vec<String>,
    #[serde(rename = "always-trusted", default)]
    pub always_trusted: bool,
}

#[derive(Debug, Snafu)]
pub enum TemplateParseError {
    #[snafu(display("Failed to read file: {}", path.display()))]
    ReadFile { source: io::Error, path: PathBuf },
    #[snafu(display("Failed to parse string to MirrorConfigTemplate"))]
    Parse { source: toml::de::Error },
}

impl MirrorsConfigTemplate {
    pub fn parse_from_file(path: impl AsRef<Path>) -> Result<Self, TemplateParseError> {
        let s = fs::read(path.as_ref()).context(ReadFileSnafu {
            path: path.as_ref().to_path_buf(),
        })?;

        Self::parse_from_slice(&s)
    }

    pub fn parse_from_slice(slice: &[u8]) -> Result<Self, TemplateParseError> {
        toml::from_slice(slice).context(ParseSnafu)
    }
}
