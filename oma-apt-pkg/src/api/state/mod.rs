//! The unified APT state: packages from `Packages` indexes, merged with the
//! live dpkg status file.
//!
//! APT's cache is organised by architecture, not by name: a [`Group`] holds
//! one [`Package`] per architecture, and each package owns its [`Version`]
//! list (`pkgCache::Group` → `pkgCache::Package` → `pkgCache::Version`).
//! `Architecture: all` is a version property, not a package architecture:
//! APT maps it to the native package (`pkgCacheGenerator::NewPackage`).
//!
//! [`AptState`] is the single entry point for both sources. Parsers for
//! `Packages` files and the dpkg status file stay private to the crate; callers
//! query packages and versions here.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;

use glob_match::glob_match;

use super::arch::{Arch, PackageKey};
use super::model::{Group, Package, Version};
use super::query::{Query, QueryResolution, ResolvedPackage, matches_version};

/// The unified package state: apt lists plus dpkg status.
#[derive(Debug)]
pub struct AptState {
    native: Arch,
    architectures: Vec<Arch>,
    /// name → per-arch versions (apt's Group → Package → Version). A group
    /// has a handful of architectures at most, so they live in a list.
    groups: HashMap<String, Vec<(Arch, Vec<Version>)>>,
    /// Package insertion order per group, which apt's `:any` follows.
    package_order: HashMap<String, Vec<Arch>>,
    /// name → arch → installed version. Nested so a package view can look
    /// itself up without building a [`PackageKey`].
    installed: HashMap<String, HashMap<Arch, String>>,
}

/// The versions of one package, possibly still only in the mapped cache.
impl AptState {
    /// Load the `Packages` indexes and merge the live dpkg status file.
    /// The native architecture (`APT::Architecture`).
    pub fn native_arch(&self) -> &Arch {
        &self.native
    }

    /// The architectures the cache was built for.
    pub fn architectures(&self) -> &[Arch] {
        &self.architectures
    }

    /// The group with this package name, if any.
    pub fn group<'a>(&'a self, name: &str) -> Option<Group<'a>> {
        let (name, _) = self.groups.get_key_value(name)?;
        Some(Group::new(self, name))
    }

    /// The package built for `arch`, if any.
    pub fn package<'a>(&'a self, name: &str, arch: &Arch) -> Option<Package<'a>> {
        let (name, packages) = self.groups.get_key_value(name)?;
        let (arch, _) = packages
            .iter()
            .find(|(candidate, _)| candidate.as_str() == arch.as_str())?;
        Some(Package::new(self, name, arch))
    }

    /// All package groups, in no particular order.
    pub fn groups<'a>(&'a self) -> impl Iterator<Item = Group<'a>> + 'a {
        self.groups.keys().map(|name| Group::new(self, name))
    }

    /// The installed version of `key`, if the status file has one.
    pub fn installed_version<'a>(&'a self, key: &PackageKey) -> Option<&'a str> {
        self.installed_version_of(key.name(), key.arch())
    }

    /// The installed version of `name:arch`, without building a key.
    pub fn installed_version_of<'a>(&'a self, name: &str, arch: &Arch) -> Option<&'a str> {
        self.installed.get(name)?.get(arch).map(String::as_str)
    }

    /// Whether `key` is installed.
    pub fn is_installed(&self, key: &PackageKey) -> bool {
        self.installed
            .get(key.name())
            .is_some_and(|packages| packages.contains_key(key.arch()))
    }

    /// Whether `name:arch` is installed, without building a key.
    pub fn is_installed_of(&self, name: &str, arch: &Arch) -> bool {
        self.installed
            .get(name)
            .is_some_and(|packages| packages.contains_key(arch))
    }

    /// Insert one local `.deb` and return its key, so a query can resolve it.
    pub fn insert_deb(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<PackageKey, crate::deb::DebError> {
        let control = crate::deb::parse_deb(path.as_ref())?;
        let version_arch = Arch::new(
            control
                .architecture
                .clone()
                .unwrap_or_else(|| self.native.to_string()),
        );
        let package_arch = if version_arch.is_all() {
            self.native.clone()
        } else {
            version_arch.clone()
        };
        let key = PackageKey::new(control.package.clone(), package_arch.clone());
        let source = crate::deb::source_for(path.as_ref());
        let name = control.package.clone();
        self.insert_version(
            &name,
            package_arch,
            Version {
                control,
                sources: vec![source],
                arch: version_arch,
            },
        );
        Ok(key)
    }

    /// Resolve package queries the way apt resolves a package name.
    ///
    /// A local `.deb` path must first be added with [`Self::insert_deb`]; it
    /// is then resolved from the version that insertion recorded. Non-path
    /// queries deliberately ignore locally inserted versions, so mixing a
    /// `.deb` path with a package name does not expose the local record as a
    /// repository version. Versions are borrowed from the state unless a
    /// `=version` or `/branch` filter (or local records) requires a filtered
    /// copy.
    pub fn resolve<'a>(&'a self, queries: Vec<String>) -> QueryResolution<'a> {
        let mut resolved = Vec::new();
        let mut no_match = Vec::new();

        for query in queries {
            if query.ends_with(".deb") && Path::new(&query).is_file() {
                if !self.resolve_deb(&query, &mut resolved) {
                    no_match.push(query);
                }
                continue;
            }

            let parsed = Query::parse(&query);
            let glob =
                parsed.name.contains('*') || parsed.name.contains('?') || parsed.name.contains('[');

            let matched = if glob {
                let mut matched = false;
                for name in self
                    .groups
                    .keys()
                    .filter(|name| glob_match(parsed.name, name))
                {
                    matched |= self.resolve_name(name, parsed, &mut resolved);
                }
                matched
            } else {
                self.groups
                    .get_key_value(parsed.name)
                    .is_some_and(|(name, _)| self.resolve_name(name, parsed, &mut resolved))
            };

            if !matched {
                no_match.push(query);
            }
        }

        QueryResolution { resolved, no_match }
    }

    /// Resolve one package name for `parsed`, appending the selected version
    /// group. Returns whether anything matched.
    fn resolve_name<'a>(
        &'a self,
        name: &str,
        parsed: Query<'_>,
        resolved: &mut Vec<ResolvedPackage<'a>>,
    ) -> bool {
        let Some(arch) = self.select_arch(name, parsed.arch) else {
            return false;
        };
        let versions = self.versions_of(name, arch);
        // Locally inserted `.deb` records describe a file, not an apt
        // archive: a normal package query must not see them even when both
        // are present in one invocation.
        let has_local = versions.iter().any(Version::is_local_deb);
        let selected: Cow<'a, [Version]> =
            if !has_local && parsed.version.is_none() && parsed.branch.is_none() {
                Cow::Borrowed(versions)
            } else {
                Cow::Owned(
                    versions
                        .iter()
                        .filter(|version| !version.is_local_deb())
                        .filter(|version| matches_version(version, parsed.version, parsed.branch))
                        .filter_map(|version| {
                            // A branch query reports only the sources of that
                            // branch; a version may also be in other suites.
                            let Some(branch) = parsed.branch else {
                                return Some(version.clone());
                            };
                            let mut version = version.clone();
                            version.sources.retain(|source| source.suite == branch);
                            (!version.sources.is_empty()).then_some(version)
                        })
                        .collect(),
                )
            };
        if selected.is_empty() {
            return false;
        }

        resolved.push(ResolvedPackage {
            key: PackageKey::new(name, (*arch).clone()),
            versions: selected,
            arch: parsed.arch.map(Arch::new),
        });
        true
    }

    /// Resolve a local `.deb` that the caller already inserted into the state.
    fn resolve_deb<'a>(&'a self, path: &str, resolved: &mut Vec<ResolvedPackage<'a>>) -> bool {
        let Ok(control) = crate::deb::parse_deb(path) else {
            return false;
        };
        let version_arch = Arch::new(
            control
                .architecture
                .clone()
                .unwrap_or_else(|| self.native.to_string()),
        );
        let package_arch = if version_arch.is_all() {
            self.native.clone()
        } else {
            version_arch.clone()
        };
        let key = PackageKey::new(control.package.clone(), package_arch.clone());
        let source = crate::deb::source_for(path);
        let versions = self.versions_of(&control.package, &package_arch);
        if versions.is_empty() {
            return false;
        }
        let Some(index) = versions.iter().position(|version| {
            version.version() == control.version.as_deref()
                && version.arch == version_arch
                && version.sources.iter().any(|candidate| {
                    candidate.is_local_deb() && candidate.base_url == source.base_url
                })
        }) else {
            return false;
        };

        resolved.push(ResolvedPackage {
            key,
            versions: Cow::Borrowed(&versions[index..index + 1]),
            arch: None,
        });
        true
    }

    /// Package architectures in insertion order (apt's group package order).
    /// Status-only packages are appended during the status merge.
    pub(crate) fn package_order(&self, name: &str) -> &[Arch] {
        self.package_order
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// The versions of `name:arch`, deserializing a mapped package on first
    /// access.
    pub(crate) fn versions_of<'a>(&'a self, name: &str, arch: &Arch) -> &'a [Version] {
        let Some(packages) = self.groups.get(name) else {
            return &[];
        };
        packages
            .iter()
            .find(|(candidate, _)| candidate.as_str() == arch.as_str())
            .map(|(_, versions)| versions.as_slice())
            .unwrap_or(&[])
    }

    /// The architectures `name` has, in no particular order.
    pub(crate) fn archs_of<'a>(&'a self, name: &str) -> impl Iterator<Item = &'a Arch> + 'a {
        self.groups
            .get(name)
            .into_iter()
            .flat_map(|packages| packages.iter().map(|(arch, _)| arch))
    }

    /// The `name:arch` package architecture, if the group has one.
    pub(crate) fn arch_of<'a>(&'a self, name: &str, arch: &str) -> Option<&'a Arch> {
        self.groups
            .get(name)?
            .iter()
            .find(|(candidate, _)| candidate.as_str() == arch)
            .map(|(arch, _)| arch)
    }

    pub(crate) fn preferred_arch<'a>(&'a self, name: &str) -> Option<&'a Arch> {
        if let Some(arch) = self.arch_of(name, self.native.as_str())
            && has_indexed_version(self.versions_of(name, arch))
        {
            return Some(arch);
        }
        for configured in &self.architectures {
            if let Some(arch) = self.arch_of(name, configured.as_str())
                && has_indexed_version(self.versions_of(name, arch))
            {
                return Some(arch);
            }
        }
        self.groups
            .get(name)?
            .iter()
            .map(|(arch, _)| arch)
            .find(|arch| has_indexed_version(self.versions_of(name, arch)))
    }

    fn select_arch<'a>(&'a self, name: &str, qualifier: Option<&str>) -> Option<&'a Arch> {
        match qualifier {
            Some(Arch::ALL) | Some(Arch::NATIVE) => self
                .arch_of(name, self.native.as_str())
                .filter(|arch| has_indexed_version(self.versions_of(name, arch))),
            Some(Arch::ANY) => self.package_order.get(name).and_then(|order| {
                order
                    .iter()
                    .find(|arch| has_indexed_version(self.versions_of(name, arch)))
            }),
            None => self.preferred_arch(name),
            Some(arch) => self
                .arch_of(name, arch)
                .filter(|arch| has_indexed_version(self.versions_of(name, arch))),
        }
    }

    fn insert_version(&mut self, name: &str, arch: Arch, version: Version) {
        let key = name.to_string();
        let packages = self.groups.entry(key.clone()).or_default();

        if let Some((_, versions)) = packages
            .iter_mut()
            .find(|(candidate, _)| candidate.as_str() == arch.as_str())
        {
            versions.push(version);
        } else {
            self.package_order
                .entry(key)
                .or_default()
                .push(arch.clone());
            packages.push((arch, vec![version]));
        }
    }
}

fn has_indexed_version(versions: &[Version]) -> bool {
    versions.iter().any(|version| !version.is_local_deb())
}
