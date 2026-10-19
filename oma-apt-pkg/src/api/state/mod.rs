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
use std::path::{Path, PathBuf};

use glob_match::glob_match;
use oma_logger::debug;

use deb822_fast::FromDeb822Paragraph;

use super::arch::{Arch, PackageKey};
use super::model::{Group, Package, Version};
use super::query::{Query, QueryResolution, ResolvedPackage, matches_version};
mod cache;
use super::{Control, Source};
use crate::apt_sources::SourceLookup;
use crate::error::Error;
use crate::internal::cache::CacheFile;
use cache::{AptStateData, load_state_cache, save_state_cache};

/// The unified package state: apt lists plus dpkg status.
#[derive(Debug)]
pub struct AptState {
    native: Arch,
    architectures: Vec<Arch>,
    /// name → arch → versions (apt's Group → Package → Version).
    groups: HashMap<String, HashMap<Arch, Vec<Version>>>,
    /// Package insertion order per group, which apt's `:any` follows.
    package_order: HashMap<String, Vec<Arch>>,
    /// name → arch → installed version. Nested so a package view can look
    /// itself up without building a [`PackageKey`].
    installed: HashMap<String, HashMap<Arch, String>>,
    status_path: PathBuf,
    files: Vec<CacheFile>,
}

impl AptState {
    /// Load the `Packages` indexes and merge the live dpkg status file.
    pub fn load(cfg: &crate::apt_config::AptConfig) -> Result<Self, Error> {
        let native = Arch::new(cfg.get("APT::Architecture", ""));
        if native.as_str().is_empty() {
            return Err(Error::Config("APT::Architecture is not set".to_string()));
        }

        let architectures: Vec<Arch> = cfg.architectures().into_iter().map(Arch::new).collect();
        let lists_dir = cfg.get_dir("Dir::State::lists", "var/lib/apt/lists");
        let cache_path = cfg.get_file("Dir::Cache::oma-aptdb", "var/cache/apt/oma-aptdb.bincode");
        let status_path = PathBuf::from(cfg.get_file("Dir::State::status", "var/lib/dpkg/status"));
        let lookup = SourceLookup::build(cfg);
        let archs = cfg.architectures();

        if let Some(data) = load_state_cache(&cache_path)
            && data.native_arch == native
            && data.architectures == architectures
            && crate::internal::cache::valid(&cache_path, &lists_dir, &lookup, &archs, &data.files)
        {
            debug!("oma package state cache hit: {cache_path}");
            let mut state = Self {
                native,
                architectures,
                groups: data.groups,
                package_order: data.package_order,
                installed: HashMap::new(),
                status_path,
                files: data.files,
            };
            state.merge_status()?;
            return Ok(state);
        }

        debug!("oma package state cache miss: {cache_path}");
        let (entries, sources) =
            crate::internal::lists::parse_apt_lists_dir_with_sources(&lists_dir, &lookup, &archs)?;

        let mut state = Self {
            native,
            architectures,
            groups: HashMap::new(),
            package_order: HashMap::new(),
            installed: HashMap::new(),
            status_path,
            files: crate::internal::cache::collect(&lists_dir, &lookup, &archs),
        };
        state.add_index(entries, sources);
        state.save_cache(&cache_path);
        state.merge_status()?;
        Ok(state)
    }

    /// The native architecture (`APT::Architecture`).
    pub fn native_arch(&self) -> &Arch {
        &self.native
    }

    /// The architectures the cache was built for.
    pub fn architectures(&self) -> &[Arch] {
        &self.architectures
    }

    /// The status file this state was built from.
    pub fn status_path(&self) -> &Path {
        &self.status_path
    }

    /// The Packages files this state was built from, for cache validation.
    pub(crate) fn files(&self) -> &[crate::internal::cache::CacheFile] {
        &self.files
    }

    /// The group with this package name, if any.
    pub fn group<'a>(&'a self, name: &str) -> Option<Group<'a>> {
        let (name, _) = self.groups.get_key_value(name)?;
        Some(Group::new(self, name))
    }

    /// The package built for `arch`, if any.
    pub fn package<'a>(&'a self, name: &str, arch: &Arch) -> Option<Package<'a>> {
        let (name, packages) = self.groups.get_key_value(name)?;
        let (arch, _) = packages.get_key_value(arch.as_str())?;
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

    /// Move the Packages data into the cache payload and back, instead of
    /// cloning the whole group map just to serialize it.
    fn save_cache(&mut self, path: &str) {
        let data = AptStateData {
            groups: std::mem::take(&mut self.groups),
            package_order: std::mem::take(&mut self.package_order),
            native_arch: self.native.clone(),
            architectures: self.architectures.clone(),
            files: std::mem::take(&mut self.files),
        };
        if let Err(e) = save_state_cache(path, &data) {
            debug!("Failed to save oma package state cache: {e}");
        }
        self.groups = data.groups;
        self.package_order = data.package_order;
        self.files = data.files;
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
    /// is then resolved from the version that insertion recorded. Versions
    /// are borrowed from the state unless a `=version` or `/branch` filter
    /// applies, in which case only the matching records are cloned.
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
        let Some(versions) = self.packages(name).get(arch) else {
            return false;
        };
        let selected: Cow<'a, [Version]> = if parsed.version.is_none() && parsed.branch.is_none() {
            Cow::Borrowed(versions.as_slice())
        } else {
            Cow::Owned(
                versions
                    .iter()
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
        let Some(versions) = self
            .groups
            .get(&control.package)
            .and_then(|packages| packages.get(&package_arch))
        else {
            return false;
        };
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

    pub(crate) fn packages<'a>(&'a self, name: &str) -> &'a HashMap<Arch, Vec<Version>> {
        static EMPTY: std::sync::OnceLock<HashMap<Arch, Vec<Version>>> = std::sync::OnceLock::new();
        self.groups
            .get(name)
            .unwrap_or_else(|| EMPTY.get_or_init(HashMap::new))
    }

    pub(crate) fn preferred_arch<'a>(&'a self, name: &str) -> Option<&'a Arch> {
        let packages = self.packages(name);
        if let Some((arch, _)) = packages.get_key_value(self.native.as_str()) {
            return Some(arch);
        }
        for arch in &self.architectures {
            if let Some((found, _)) = packages.get_key_value(arch.as_str()) {
                return Some(found);
            }
        }
        packages.keys().next()
    }

    fn select_arch<'a>(&'a self, name: &str, qualifier: Option<&str>) -> Option<&'a Arch> {
        match qualifier {
            Some(Arch::ALL) | Some(Arch::NATIVE) => self
                .packages(name)
                .get_key_value(self.native.as_str())
                .map(|(arch, _)| arch),
            Some(Arch::ANY) => self.package_order.get(name).and_then(|order| order.first()),
            None => self.preferred_arch(name),
            Some(arch) => self
                .packages(name)
                .get_key_value(arch)
                .map(|(arch, _)| arch),
        }
    }

    fn insert_version(&mut self, name: &str, arch: Arch, version: Version) {
        let groups = &mut self.groups;
        let order = &mut self.package_order;
        let key = name.to_string();
        let packages = groups.entry(key.clone()).or_default();

        if !packages.contains_key(&arch) {
            order.entry(key).or_default().push(arch.clone());
        }
        push_or_merge(packages.entry(arch).or_default(), version);
    }

    fn add_index(&mut self, entries: Vec<Control>, sources: Vec<Source>) {
        for (control, source) in entries.into_iter().zip(sources) {
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
        }
    }

    fn merge_status(&mut self) -> Result<(), Error> {
        let paragraphs = crate::internal::dpkg::paragraphs(&self.status_path)?;
        let source = Source::dpkg_status(self.status_path.display().to_string());

        for paragraph in paragraphs {
            let paragraph = paragraph.map_err(crate::internal::dpkg::DpkgError::from)?;
            let pkg = crate::internal::dpkg::DpkgPackage::from_paragraph(&paragraph)
                .map_err(crate::internal::dpkg::DpkgError::Malformed)?;
            if !pkg.has_version() {
                continue;
            }
            let Some(version) = pkg.version.clone() else {
                continue;
            };
            let control = Control::from_paragraph(&paragraph)
                .map_err(crate::internal::dpkg::DpkgError::Malformed)?;
            let version_arch = Arch::new(
                pkg.architecture
                    .clone()
                    .unwrap_or_else(|| self.native.to_string()),
            );
            let package_arch = if version_arch.is_all() {
                self.native.clone()
            } else {
                version_arch.clone()
            };
            if !self
                .groups
                .get(&pkg.name)
                .is_some_and(|packages| packages.contains_key(&package_arch))
            {
                self.package_order
                    .entry(pkg.name.clone())
                    .or_default()
                    .push(package_arch.clone());
            }
            let versions = self
                .groups
                .entry(pkg.name.clone())
                .or_default()
                .entry(package_arch.clone())
                .or_default();

            if let Some(existing) = versions.iter_mut().find(|existing| {
                existing.version() == Some(version.as_str()) && existing.arch == version_arch
            }) {
                if !existing.sources.contains(&source) {
                    existing.sources.push(source.clone());
                }
            } else {
                versions.push(Version {
                    control,
                    sources: vec![source.clone()],
                    arch: version_arch,
                });
            }

            if pkg.selection_state().is_installed() {
                self.installed
                    .entry(pkg.name.clone())
                    .or_default()
                    .insert(package_arch, version);
            }
        }

        Ok(())
    }
}

fn push_or_merge(versions: &mut Vec<Version>, version: Version) {
    let new_is_local = version.sources.iter().any(Source::is_local_deb);
    if let Some(existing) = versions.iter_mut().find(|existing| {
        if existing.version() != version.version() || existing.arch != version.arch {
            return false;
        }
        if new_is_local {
            // Keep distinct local files separate, but coalesce re-inserting
            // the same file.
            existing
                .sources
                .iter()
                .any(|source| source.is_local_deb() && version.sources.contains(source))
        } else {
            // A local `.deb` keeps its own control record; never merge it
            // with an indexed version of the same identity.
            !existing.sources.iter().any(Source::is_local_deb)
        }
    }) {
        for source in version.sources {
            if !existing.sources.contains(&source) {
                existing.sources.push(source);
            }
        }
    } else {
        versions.push(version);
    }
}

#[cfg(test)]
mod tests;
