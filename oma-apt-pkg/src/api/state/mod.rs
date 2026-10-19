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
    /// (name, arch) → installed version.
    installed: HashMap<PackageKey, String>,
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
            state.merge_status();
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
        state.merge_status();
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
        Some(Package::new(self, name, arch.clone()))
    }

    /// All package groups, in no particular order.
    pub fn groups<'a>(&'a self) -> impl Iterator<Item = Group<'a>> + 'a {
        self.groups.keys().map(|name| Group::new(self, name))
    }

    /// The installed version of `key`, if the status file has one.
    pub fn installed_version<'a>(&'a self, key: &PackageKey) -> Option<&'a str> {
        self.installed.get(key).map(String::as_str)
    }

    /// Whether `key` is installed.
    pub fn is_installed(&self, key: &PackageKey) -> bool {
        self.installed.contains_key(key)
    }

    fn save_cache(&self, path: &str) {
        let data = AptStateData {
            groups: self.groups.clone(),
            package_order: self.package_order.clone(),
            files: self.files.clone(),
        };
        if let Err(e) = save_state_cache(path, &data) {
            debug!("Failed to save oma package state cache: {e}");
        }
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
    pub fn resolve(&mut self, queries: Vec<String>) -> QueryResolution {
        // Local `.deb` files insert into the state; do that first so the
        // view borrows below are immutable.
        let (debs, queries): (Vec<String>, Vec<String>) = queries
            .into_iter()
            .partition(|query| query.ends_with(".deb") && Path::new(query).is_file());
        let mut no_match = Vec::new();
        let mut deb_keys = Vec::new();
        for path in debs {
            match self.insert_deb(&path) {
                Ok(key) => deb_keys.push(key),
                Err(_) => no_match.push(path),
            }
        }

        let mut resolved = Vec::new();

        for key in deb_keys {
            let Some((_name, packages)) = self.groups.get_key_value(key.name()) else {
                continue;
            };
            let Some(versions) = packages.get(key.arch().as_str()) else {
                continue;
            };
            resolved.push(ResolvedPackage {
                key: key.clone(),
                versions: versions.clone(),
                arch: None,
            });
        }

        for query in queries {
            let matched = {
                let parsed = Query::parse(&query);
                let names: Vec<&str> = if parsed.name.contains('*')
                    || parsed.name.contains('?')
                    || parsed.name.contains('[')
                {
                    self.groups
                        .keys()
                        .filter(|name| glob_match(parsed.name, name))
                        .map(String::as_str)
                        .collect()
                } else {
                    self.groups
                        .get_key_value(parsed.name)
                        .map(|(name, _)| name.as_str())
                        .into_iter()
                        .collect()
                };

                let mut matched = false;
                for name in names {
                    let Some(arch) = self.select_arch(name, parsed.arch) else {
                        continue;
                    };
                    let Some(versions) = self.packages(name).get(arch.as_str()) else {
                        continue;
                    };
                    let selected: Vec<Version> = versions
                        .iter()
                        .filter(|version| matches_version(version, parsed.version, parsed.branch))
                        .cloned()
                        .collect();
                    if selected.is_empty() {
                        continue;
                    }
                    matched = true;
                    resolved.push(ResolvedPackage {
                        key: PackageKey::new(name, arch),
                        versions: selected,
                        arch: parsed.arch.map(Arch::new),
                    });
                }
                matched
            };

            if !matched {
                no_match.push(query);
            }
        }

        QueryResolution { resolved, no_match }
    }

    pub(crate) fn packages<'a>(&'a self, name: &str) -> &'a HashMap<Arch, Vec<Version>> {
        static EMPTY: std::sync::OnceLock<HashMap<Arch, Vec<Version>>> = std::sync::OnceLock::new();
        self.groups
            .get(name)
            .unwrap_or_else(|| EMPTY.get_or_init(HashMap::new))
    }

    pub(crate) fn preferred_arch(&self, name: &str) -> Option<Arch> {
        let packages = self.packages(name);
        if packages.contains_key(self.native.as_str()) {
            return Some(self.native.clone());
        }
        for arch in &self.architectures {
            if packages.contains_key(arch.as_str()) {
                return Some(arch.clone());
            }
        }
        packages.keys().next().cloned()
    }

    fn select_arch(&self, name: &str, qualifier: Option<&str>) -> Option<Arch> {
        match qualifier {
            Some(Arch::ALL) | Some(Arch::NATIVE) => self
                .packages(name)
                .get_key_value(self.native.as_str())
                .map(|(arch, _)| arch.clone()),
            Some(Arch::ANY) => self
                .package_order
                .get(name)
                .and_then(|order| order.first().cloned()),
            None => self.preferred_arch(name),
            Some(arch) => self
                .packages(name)
                .get_key_value(arch)
                .map(|(arch, _)| arch.clone()),
        }
    }

    fn insert_version(&mut self, name: &str, arch: Arch, version: Version) {
        if !self
            .groups
            .get(name)
            .is_some_and(|packages| packages.contains_key(&arch))
        {
            self.package_order
                .entry(name.to_string())
                .or_default()
                .push(arch.clone());
        }
        push_or_merge(
            self.groups
                .entry(name.to_string())
                .or_default()
                .entry(arch)
                .or_default(),
            version,
        );
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

    fn merge_status(&mut self) {
        let Ok(paragraphs) = crate::internal::dpkg::paragraphs(&self.status_path) else {
            return;
        };
        let source = Source::dpkg_status(self.status_path.display().to_string());

        for paragraph in paragraphs.filter_map(Result::ok) {
            let Ok(pkg) = crate::internal::dpkg::DpkgPackage::from_paragraph(&paragraph) else {
                continue;
            };
            if !pkg.has_version() {
                continue;
            }
            let Some(version) = pkg.version.clone() else {
                continue;
            };
            let Ok(control) = Control::from_paragraph(&paragraph) else {
                continue;
            };
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
            let key = PackageKey::new(pkg.name.clone(), package_arch.clone());
            let versions = self
                .groups
                .entry(pkg.name.clone())
                .or_default()
                .entry(package_arch)
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
                self.installed.insert(key, version);
            }
        }
    }
}

fn push_or_merge(versions: &mut Vec<Version>, version: Version) {
    if let Some(existing) = versions
        .iter_mut()
        .find(|existing| existing.version() == version.version() && existing.arch == version.arch)
    {
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
