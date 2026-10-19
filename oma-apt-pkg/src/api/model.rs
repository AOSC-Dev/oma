use std::borrow::Cow;

use super::arch::{Arch, PackageKey};
use super::state::AptState;
use super::{Control, Source};

/// One version record of a package.
#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Version {
    /// The raw control fields, as apt's `pkgCache::Version` keeps them.
    pub control: Control,
    /// Every index (or the status file) this version was found in.
    pub sources: Vec<Source>,
    /// `all` for an `Architecture: all` version, otherwise the package arch.
    pub arch: Arch,
}

impl Version {
    /// The version string, if the record has one.
    pub fn version(&self) -> Option<&str> {
        self.control.version.as_deref()
    }

    /// The package name this version belongs to.
    pub fn package(&self) -> &str {
        &self.control.package
    }

    /// Whether this is an `Architecture: all` version.
    pub fn is_all(&self) -> bool {
        self.arch.is_all()
    }

    /// The architecture apt would display for this version (`all` or the
    /// package architecture).
    pub fn arch(&self) -> &Arch {
        &self.arch
    }

    /// The version string parsed for Debian version ordering.
    pub fn parsed_version(&self) -> Option<debversion::Version> {
        self.version()?.parse().ok()
    }
}

/// All packages sharing one package name — apt's `pkgCache::Group`.
#[derive(Debug)]
pub struct Group<'a> {
    state: &'a AptState,
    name: &'a str,
}

impl<'a> Group<'a> {
    pub(crate) fn new(state: &'a AptState, name: &'a str) -> Self {
        Self { state, name }
    }

    /// The group (package name).
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// One package per architecture.
    pub fn packages(&self) -> impl Iterator<Item = Package<'a>> + 'a {
        let state = self.state;
        let name = self.name;
        state
            .packages(name)
            .keys()
            .map(move |arch| Package::new(state, name, arch))
    }

    /// The package built for `arch`, if the group has one.
    pub fn package(&self, arch: &Arch) -> Option<Package<'a>> {
        self.state
            .packages(self.name)
            .get_key_value(arch.as_str())
            .map(|(arch, _)| Package::new(self.state, self.name, arch))
    }

    /// Apt's preferred package: the native build, else the first configured
    /// architecture, else whatever the group has (`FindPreferredPkg`).
    pub fn preferred(&self) -> Option<Package<'a>> {
        self.state
            .preferred_arch(self.name)
            .map(|arch| Package::new(self.state, self.name, arch))
    }
}

/// One package: a name and an architecture — apt's `pkgCache::Package`.
#[derive(Debug)]
pub struct Package<'a> {
    state: &'a AptState,
    name: &'a str,
    arch: &'a Arch,
}

impl<'a> Package<'a> {
    pub(crate) fn new(state: &'a AptState, name: &'a str, arch: &'a Arch) -> Self {
        Self { state, name, arch }
    }

    /// The package name.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// The package architecture (never `all`).
    pub fn arch(&self) -> &'a Arch {
        self.arch
    }

    /// The package identity.
    pub fn key(&self) -> PackageKey {
        PackageKey::new(self.name, (*self.arch).clone())
    }

    /// Every version of this package, newest first.
    pub fn versions(&self) -> &'a [Version] {
        self.state
            .packages(self.name)
            .get(self.arch.as_str())
            .map(|versions| versions.as_slice())
            .unwrap_or(&[])
    }

    /// Number of versions.
    pub fn version_count(&self) -> usize {
        self.versions().len()
    }

    /// The version apt would install: whatever the caller has already
    /// selected, or the highest one for a bare package view.
    pub fn candidate(&self) -> Option<&'a Version> {
        self.versions()
            .iter()
            .max_by(|a, b| a.parsed_version().cmp(&b.parsed_version()))
    }

    /// The installed version, if the status file marks one.
    pub fn installed_version(&self) -> Option<&'a str> {
        self.state.installed_version_of(self.name, self.arch)
    }

    /// Whether the package is installed.
    pub fn is_installed(&self) -> bool {
        self.installed_version().is_some()
    }

    /// The name apt displays, `name[:arch]` — the architecture is omitted for
    /// the native build and for `Architecture: all` versions.
    pub fn fullname(&self, version: &Version, pretty: bool) -> Cow<'a, str> {
        let arch = version.arch();
        if pretty && (arch.is_all() || arch == self.state.native_arch()) {
            Cow::Borrowed(self.name)
        } else {
            Cow::Owned(format!("{}:{}", self.name, arch))
        }
    }
}
