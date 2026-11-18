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
use std::sync::OnceLock;

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
use crate::internal::cache::from_archived;
use cache::{AptStateData, StateCache, save_state_cache};

/// The unified package state: apt lists plus dpkg status.
#[derive(Debug)]
pub struct AptState {
    native: Arch,
    architectures: Vec<Arch>,
    /// name → per-arch versions (apt's Group → Package → Version). Versions
    /// read from the mapped cache are deserialized on first access. A group
    /// has a handful of architectures at most, so they live in a list: one
    /// small allocation per package instead of a hash table.
    groups: HashMap<String, Vec<(Arch, Versions)>>,
    /// Package insertion order per group, which apt's `:any` follows.
    package_order: HashMap<String, Vec<Arch>>,
    /// name → arch → installed version. Nested so a package view can look
    /// itself up without building a [`PackageKey`].
    installed: HashMap<String, HashMap<Arch, String>>,
    status_path: PathBuf,
    files: Vec<CacheFile>,
    /// The mapped package cache backing every group still to be
    /// deserialized.
    archive: Option<StateCache>,
}

/// The versions of one package, possibly still only in the mapped cache.
#[derive(Debug)]
enum Versions {
    /// Backed by the cache archive: deserialized on first access, with the
    /// dpkg status records collected meanwhile merged in.
    Mapped {
        cell: OnceLock<Vec<Version>>,
        pending: Vec<Pending>,
    },
    /// Owned: built from the indexes (cache miss), or present only in the
    /// status file or a local `.deb`.
    Owned(Vec<Version>),
}

/// A dpkg status record waiting for its package to be deserialized.
#[derive(Debug)]
enum Pending {
    /// The status file carries a version no index has: keep the record.
    Version(Box<Version>),
    /// The version is already in the indexes; only its status source is new.
    Source { version: String, source: Source },
}

impl Versions {
    /// The versions, deserializing a mapped package on first access.
    fn get(&self, load: impl FnOnce() -> Vec<Version>) -> &[Version] {
        match self {
            Self::Owned(versions) => versions,
            Self::Mapped { cell, pending } => cell.get_or_init(|| {
                let mut versions = load();
                for record in pending {
                    match record {
                        Pending::Version(version) => {
                            push_or_merge(&mut versions, (**version).clone())
                        }
                        Pending::Source { version, source } => {
                            if let Some(existing) = versions
                                .iter_mut()
                                .find(|existing| existing.version() == Some(version.as_str()))
                                && !existing.sources.contains(source)
                            {
                                existing.sources.push(source.clone());
                            }
                        }
                    }
                }
                versions
            }),
        }
    }

    /// Merge one record, keeping a still-unmapped package in `pending` so
    /// the deserialization sees it.
    fn push(&mut self, version: Version) {
        match self {
            Self::Owned(versions) => push_or_merge(versions, version),
            Self::Mapped { cell, pending } => match cell.get_mut() {
                Some(versions) => push_or_merge(versions, version),
                None => pending.push(Pending::Version(Box::new(version))),
            },
        }
    }
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

        if let Some(archive) = StateCache::open(&cache_path) {
            let files = archive.files();
            let cache_matches = {
                let data = archive.archived();
                from_archived::<Arch>(&data.native_arch) == native
                    && data
                        .architectures
                        .iter()
                        .map(from_archived::<Arch>)
                        .eq(architectures.iter().cloned())
                    && crate::internal::cache::valid(
                        &cache_path,
                        &lists_dir,
                        &lookup,
                        cfg,
                        &archs,
                        &files,
                    )
            };

            if cache_matches {
                debug!("oma package state cache hit: {cache_path}");
                let mut state =
                    Self::from_cache(archive, native, architectures, status_path, files);
                state.merge_status()?;
                return Ok(state);
            }
        }

        debug!("oma package state cache miss: {cache_path}");
        let (entries, sources) = crate::internal::lists::parse_apt_lists_dir_with_sources(
            &lists_dir, &lookup, cfg, &archs,
        )?;

        let mut data = AptStateData {
            groups: HashMap::new(),
            package_order: HashMap::new(),
            native_arch: native.clone(),
            architectures: architectures.clone(),
            files: crate::internal::cache::collect(&lists_dir, &lookup, cfg, &archs),
        };
        add_index(&mut data, &native, entries, sources);
        if let Err(e) = save_state_cache(&cache_path, &data) {
            debug!("Failed to save oma package state cache: {e}");
        }

        let mut state = Self::from_data(data, status_path);
        state.merge_status()?;
        Ok(state)
    }

    /// Index the mapped cache, leaving every package to be deserialized on
    /// first access.
    fn from_cache(
        archive: StateCache,
        native: Arch,
        architectures: Vec<Arch>,
        status_path: PathBuf,
        files: Vec<CacheFile>,
    ) -> Self {
        let (groups, package_order) = {
            let data = archive.archived();

            let mut groups = HashMap::with_capacity(data.groups.len());
            for (name, packages) in data.groups.iter() {
                let arches = packages
                    .iter()
                    .map(|(arch, _)| {
                        (
                            from_archived::<Arch>(arch),
                            Versions::Mapped {
                                cell: OnceLock::new(),
                                pending: Vec::new(),
                            },
                        )
                    })
                    .collect();
                groups.insert(from_archived::<String>(name), arches);
            }

            let package_order = data
                .package_order
                .iter()
                .map(|(name, order)| {
                    (
                        from_archived::<String>(name),
                        order.iter().map(from_archived::<Arch>).collect(),
                    )
                })
                .collect();

            (groups, package_order)
        };

        Self {
            native,
            architectures,
            groups,
            package_order,
            installed: HashMap::new(),
            status_path,
            files,
            archive: Some(archive),
        }
    }

    /// Take a freshly built state directly, without round-tripping it
    /// through the cache.
    fn from_data(data: AptStateData, status_path: PathBuf) -> Self {
        let AptStateData {
            groups,
            package_order,
            native_arch,
            architectures,
            files,
        } = data;

        let groups = groups
            .into_iter()
            .map(|(name, packages)| {
                (
                    name,
                    packages
                        .into_iter()
                        .map(|(arch, versions)| (arch, Versions::Owned(versions)))
                        .collect::<Vec<_>>(),
                )
            })
            .collect();

        Self {
            native: native_arch,
            architectures,
            groups,
            package_order,
            installed: HashMap::new(),
            status_path,
            files,
            archive: None,
        }
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
        // `:all` names the `Architecture: all` variant: it resolves to the
        // native package (APT attaches `all` versions there), but must not
        // also return that package's native builds.
        let all_only = parsed.arch == Some(Arch::ALL);
        // Locally inserted `.deb` records describe a file, not an apt
        // archive: a normal package query must not see them even when both
        // are present in one invocation.
        let has_local = versions.iter().any(Version::is_local_deb);
        let selected: Cow<'a, [Version]> =
            if !has_local && !all_only && parsed.version.is_none() && parsed.branch.is_none() {
                Cow::Borrowed(versions)
            } else {
                Cow::Owned(
                    versions
                        .iter()
                        .filter(|version| !version.is_local_deb())
                        .filter(|version| !all_only || version.is_all())
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
        let Some((_, versions)) = packages
            .iter()
            .find(|(candidate, _)| candidate.as_str() == arch.as_str())
        else {
            return &[];
        };
        versions.get(|| self.deserialize(name, arch))
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

    /// Deserialize one package from the mapped cache.
    fn deserialize(&self, name: &str, arch: &Arch) -> Vec<Version> {
        let Some(archive) = &self.archive else {
            return Vec::new();
        };
        let data = archive.archived();
        let Some(packages) = data.groups.get(name) else {
            return Vec::new();
        };
        // A group has a handful of architectures at most; the archived key
        // type is not a `str`, so compare them one by one.
        let Some(versions) = packages
            .iter()
            .find(|(candidate, _)| from_archived::<Arch>(candidate) == *arch)
            .map(|(_, versions)| versions)
        else {
            return Vec::new();
        };

        versions.iter().map(from_archived::<Version>).collect()
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
            packages.push((arch, Versions::Owned(vec![version])));
        }
    }

    /// Record that the status file carries `version` for `name:arch`, when
    /// the mapped index already has it: only the status source is new.
    /// Returns whether the record could be deferred.
    fn defer_status_source(
        &mut self,
        name: &str,
        arch: &Arch,
        version: &str,
        source: &Source,
    ) -> bool {
        if !self.index_has_version(name, arch, version) {
            return false;
        }
        let Some(packages) = self.groups.get_mut(name) else {
            return false;
        };
        let Some((_, slot)) = packages
            .iter_mut()
            .find(|(candidate, _)| candidate.as_str() == arch.as_str())
        else {
            return false;
        };

        match slot {
            Versions::Owned(_) => false,
            Versions::Mapped { cell, pending } => {
                if cell.get_mut().is_some() {
                    return false;
                }
                pending.push(Pending::Source {
                    version: version.to_string(),
                    source: source.clone(),
                });
                true
            }
        }
    }

    /// Whether the mapped index carries `version` for `name:arch`.
    fn index_has_version(&self, name: &str, arch: &Arch, version: &str) -> bool {
        let Some(archive) = &self.archive else {
            return false;
        };
        let data = archive.archived();
        let Some(packages) = data.groups.get(name) else {
            return false;
        };
        let Some(versions) = packages
            .iter()
            .find(|(candidate, _)| from_archived::<Arch>(candidate) == *arch)
            .map(|(_, versions)| versions)
        else {
            return false;
        };

        versions.iter().any(|entry| match &entry.control.version {
            rkyv::option::ArchivedOption::Some(value) => value.as_str() == version,
            rkyv::option::ArchivedOption::None => false,
        })
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
            // Parsing the whole control paragraph for every installed
            // package dominates the merge; when the indexes already carry
            // the version, its status source alone is enough.
            if !self.defer_status_source(&pkg.name, &package_arch, &version, &source) {
                let control = Control::from_paragraph(&paragraph)
                    .map_err(crate::internal::dpkg::DpkgError::Malformed)?;
                self.insert_version(
                    &pkg.name,
                    package_arch.clone(),
                    Version {
                        control,
                        sources: vec![source.clone()],
                        arch: version_arch,
                    },
                );
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

/// Fold the parsed `Packages` records into the cache payload.
fn add_index(data: &mut AptStateData, native: &Arch, entries: Vec<Control>, sources: Vec<Source>) {
    for (control, source) in entries.into_iter().zip(sources) {
        let version_arch = Arch::new(
            control
                .architecture
                .clone()
                .unwrap_or_else(|| native.to_string()),
        );
        let package_arch = if version_arch.is_all() {
            native.clone()
        } else {
            version_arch.clone()
        };
        let name = control.package.clone();
        let version = Version {
            control,
            sources: vec![source],
            arch: version_arch,
        };

        if !data
            .groups
            .get(&name)
            .is_some_and(|packages| packages.contains_key(&package_arch))
        {
            data.package_order
                .entry(name.clone())
                .or_default()
                .push(package_arch.clone());
        }
        push_or_merge(
            data.groups
                .entry(name)
                .or_default()
                .entry(package_arch)
                .or_default(),
            version,
        );
    }
}

fn has_indexed_version(versions: &[Version]) -> bool {
    versions.iter().any(|version| !version.is_local_deb())
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
