#[derive(Debug, PartialEq, Eq, Hash)]
pub struct InstallEntry {
    name: String,
    old_version: Option<String>,
    new_version: String,
    old_size: Option<u64>,
    new_size: u64,
    pkg_urls: Vec<String>,
    checksum: String,
    arch: String,
    download_size: u64,
    op: InstallOperation,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct RemoveEntry {
    name: String,
    version: String,
    size: u64,
    details: Vec<RemoveTag>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum RemoveTag {
    Purge,
    AutoRemove,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum InstallOperation {
    Install,
    ReInstall,
    Remove,
    Upgrade,
    Downgrade,
    Download
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
        arch: String,
        download_size: u64,
        op: InstallOperation
    ) -> Self {
        Self {
            name,
            old_version,
            new_version,
            old_size,
            new_size,
            pkg_urls,
            checksum,
            arch,
            download_size,
            op
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

    pub fn arch(&self) -> &str {
        &self.arch
    }

    pub fn download_size(&self) -> u64 {
        self.download_size
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
