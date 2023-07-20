pub struct InstallEntry {
    name: String,
    old_version: Option<String>,
    new_version: String,
    size: u64,
    pkg_urls: Vec<String>,
    checksum: String,
}

pub struct RemoveEntry {
    name: String,
    version: String,
    size: u64,
    details: Option<String>,
}

pub enum OmaOpration {
    Install(InstallEntry),
    ReInstall(InstallEntry),
    Remove(RemoveEntry),
    Upgrade(InstallEntry),
    Downgrade(InstallEntry),
}

impl InstallEntry {
    pub fn new(
        name: String,
        old_version: Option<String>,
        new_version: String,
        size: u64,
        pkg_urls: Vec<String>,
        checksum: String,
    ) -> Self {
        Self {
            name,
            old_version,
            new_version,
            size,
            pkg_urls,
            checksum,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn size(&self) -> u64 {
        self.size
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
    pub fn new(name: String, version: String, size: u64, details: Option<String>) -> Self {
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

    pub fn details(&self) -> Option<&str> {
        self.details.as_deref()
    }
}
