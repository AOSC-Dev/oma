use derive_builder::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, Builder, Clone, Default, Serialize, Deserialize)]
#[builder(default)]
pub struct InstallEntry {
    name: String,
    #[builder(setter(into, strip_option))]
    old_version: Option<String>,
    new_version: String,
    #[builder(setter(into, strip_option))]
    old_size: Option<u64>,
    new_size: u64,
    pkg_urls: Vec<String>,
    #[builder(setter(into, strip_option))]
    checksum: Option<String>,
    arch: String,
    download_size: u64,
    op: InstallOperation,
    automatic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RemoveEntry {
    name: String,
    // FIXME: 删除软件包再删除配置文件会没有版本号
    version: Option<String>,
    size: u64,
    details: Vec<RemoveTag>,
    arch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RemoveTag {
    Purge,
    AutoRemove,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Default, Serialize, Deserialize)]
pub enum InstallOperation {
    #[default]
    Default,
    Install,
    ReInstall,
    Upgrade,
    Downgrade,
    Download,
}

impl InstallEntry {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn old_size(&self) -> Option<u64> {
        self.old_size
    }

    pub fn new_size(&self) -> u64 {
        self.new_size
    }

    pub fn old_version(&self) -> Option<&str> {
        self.old_version.as_deref()
    }

    pub fn new_version(&self) -> &str {
        &self.new_version
    }

    pub fn pkg_urls(&self) -> &[String] {
        &self.pkg_urls
    }

    pub fn checksum(&self) -> Option<&str> {
        self.checksum.as_deref()
    }

    pub fn arch(&self) -> &str {
        &self.arch
    }

    pub fn download_size(&self) -> u64 {
        self.download_size
    }

    pub fn op(&self) -> &InstallOperation {
        &self.op
    }

    pub fn automatic(&self) -> bool {
        self.automatic
    }
}

impl RemoveEntry {
    pub fn new(
        name: String,
        version: Option<String>,
        size: u64,
        details: Vec<RemoveTag>,
        arch: String,
    ) -> Self {
        Self {
            name,
            version,
            size,
            details,
            arch,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn details(&self) -> &[RemoveTag] {
        &self.details
    }

    pub fn arch(&self) -> &str {
        &self.arch
    }
}
