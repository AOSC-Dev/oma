use std::fmt::Display;

use bon::{builder, Builder};
use oma_utils::human_bytes::HumanBytes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmaOperation {
    pub install: Vec<InstallEntry>,
    pub remove: Vec<RemoveEntry>,
    pub disk_size: (Box<str>, u64),
    pub autoremovable: (u64, u64),
    pub total_download_size: u64,
}

impl Display for OmaOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut install = vec![];
        let mut upgrade = vec![];
        let mut reinstall = vec![];
        let mut downgrade = vec![];
        let mut remove = vec![];
        let mut purge = vec![];

        for ins in &self.install {
            let name = ins.name();
            let arch = ins.arch();
            let version = ins.new_version();
            match ins.op() {
                InstallOperation::Default | InstallOperation::Download => unreachable!(),
                InstallOperation::Install => {
                    if !ins.automatic() {
                        install.push(format!("{name}:{arch} ({version})"));
                    } else {
                        install.push(format!("{name}:{arch} ({version}, automatic)"));
                    }
                }
                InstallOperation::ReInstall => {
                    reinstall.push(format!("{name}:{arch} ({version})"));
                }
                InstallOperation::Upgrade => {
                    // Upgrade 的情况下 old_version 的值肯定存在，因此直接 unwreap
                    upgrade.push(format!(
                        "{name}:{arch} ({}, {version})",
                        ins.old_version().unwrap()
                    ));
                }
                InstallOperation::Downgrade => {
                    downgrade.push(format!("{name}:{arch} ({version})"));
                }
            }
        }

        for rm in &self.remove {
            let tags = rm.details();
            let name = rm.name();
            let version = rm.version();
            let arch = rm.arch();

            let mut s = format!("{name}:{arch}");
            if let Some(ver) = version {
                s.push_str(&format!(" ({ver})"));
            }

            if tags.contains(&RemoveTag::Purge) {
                purge.push(s);
            } else {
                remove.push(s);
            }
        }

        if !install.is_empty() {
            writeln!(f, "Install: {}", install.join(", "))?;
        }

        if !upgrade.is_empty() {
            writeln!(f, "Upgrade: {}", upgrade.join(", "))?;
        }

        if !reinstall.is_empty() {
            writeln!(f, "ReInstall: {}", reinstall.join(", "))?;
        }

        if !downgrade.is_empty() {
            writeln!(f, "Downgrade: {}", downgrade.join(", "))?;
        }

        if !remove.is_empty() {
            writeln!(f, "Remove: {}", remove.join(", "))?;
        }

        if !purge.is_empty() {
            writeln!(f, "Purge: {}", purge.join(", "))?;
        }

        let (symbol, n) = &self.disk_size;
        writeln!(f, "Size-delta: {symbol}{}", HumanBytes(n.to_owned()))?;

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Default, Serialize, Deserialize, Builder)]
pub struct InstallEntry {
    name: String,
    name_without_arch: String,
    old_version: Option<String>,
    new_version: String,
    old_size: Option<u64>,
    new_size: u64,
    pkg_urls: Vec<String>,
    sha256: Option<String>,
    md5: Option<String>,
    sha512: Option<String>,
    arch: String,
    download_size: u64,
    op: InstallOperation,
    #[builder(default)]
    automatic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RemoveEntry {
    name: String,
    version: Option<String>,
    size: u64,
    details: Vec<RemoveTag>,
    arch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RemoveTag {
    Purge,
    AutoRemove,
    Resolver,
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

    pub fn name_without_arch(&self) -> &str {
        &self.name_without_arch
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

    pub fn sha256(&self) -> Option<&str> {
        self.sha256.as_deref()
    }

    pub fn md5(&self) -> Option<&str> {
        self.md5.as_deref()
    }

    pub fn sha512(&self) -> Option<&str> {
        self.sha512.as_deref()
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
