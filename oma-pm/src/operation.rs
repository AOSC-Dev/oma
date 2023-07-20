pub struct InstallEntry {
    name: String,
    old_version: Option<String>,
    new_version: String,
    old_size: Option<u64>,
    new_size: u64,
    pkg_urls: Vec<String>,
    checksum: String,
}

pub struct RemoveEntry {
    name: String,
    version: String,
    size: u64,
    details: Vec<RemoveTag>
}

pub enum RemoveTag {
    Purge,
    AutoRemove,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum OmaOperation {
    Install,
    ReInstall,
    Remove,
    Upgrade,
    Downgrade,
}

pub enum OperationEntry {
    Install(InstallEntry),
    Remove(RemoveEntry),
}

impl InstallEntry {
    pub fn new(
        name: String,
        old_version: Option<String>,
        new_version: String,
        old_size: Option<u64>,
        new_size: u64,
        pkg_urls: Vec<String>,
        checksum: String,
    ) -> Self {
        Self {
            name,
            old_version,
            new_version,
            old_size,
            new_size,
            pkg_urls,
            checksum,
        }
    }

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

    pub fn checksum(&self) -> &str {
        &self.checksum
    }
}

impl RemoveEntry {
    pub fn new(name: String, version: String, size: u64, details: Vec<RemoveTag>) -> Self {
        Self {
            name,
            version,
            size,
            details,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn details(&self) -> &[RemoveTag] {
        &self.details
    }
}
