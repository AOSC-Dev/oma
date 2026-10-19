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

use deb822_fast::FromDeb822Paragraph;

pub use crate::apt_lists::{IndexSource as Source, PackageEntry as Control};
use crate::apt_sources::SourceLookup;
use crate::arch::{Arch, PackageKey};
use crate::error::Error;

/// One version record of a package.
#[derive(Debug, Clone)]
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
    /// The group (package name).
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// One package per architecture.
    pub fn packages(&self) -> impl Iterator<Item = Package<'a>> + 'a {
        let state = self.state;
        let name = self.name;
        state.packages(name).keys().map(move |arch| Package {
            state,
            name,
            arch: arch.clone(),
        })
    }

    /// The package built for `arch`, if the group has one.
    pub fn package(&self, arch: &Arch) -> Option<Package<'a>> {
        self.state
            .packages(self.name)
            .get_key_value(arch.as_str())
            .map(|(arch, _)| Package {
                state: self.state,
                name: self.name,
                arch: arch.clone(),
            })
    }

    /// Apt's preferred package: the native build, else the first configured
    /// architecture, else whatever the group has (`FindPreferredPkg`).
    pub fn preferred(&self) -> Option<Package<'a>> {
        self.state.preferred_arch(self.name).map(|arch| Package {
            state: self.state,
            name: self.name,
            arch,
        })
    }
}

/// One package: a name and an architecture — apt's `pkgCache::Package`.
#[derive(Debug)]
pub struct Package<'a> {
    state: &'a AptState,
    name: &'a str,
    arch: Arch,
}

impl<'a> Package<'a> {
    /// The package name.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// The package architecture (never `all`).
    pub fn arch(&self) -> &Arch {
        &self.arch
    }

    /// The package identity.
    pub fn key(&self) -> PackageKey {
        PackageKey::new(self.name, self.arch.clone())
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
        self.state.installed_version(&self.key())
    }

    /// Whether the package is installed.
    pub fn is_installed(&self) -> bool {
        self.installed_version().is_some()
    }

    /// The name apt displays, `name[:arch]` — the architecture is omitted for
    /// the native build and for `Architecture: all` versions.
    pub fn fullname(&self, version: &Version, pretty: bool) -> String {
        let arch = version.arch();
        if pretty && (arch.is_all() || arch == &self.state.native) {
            self.name.to_string()
        } else {
            format!("{}:{}", self.name, arch)
        }
    }
}

/// One resolved query: the selected package plus its filtered versions.
#[derive(Debug)]
pub struct ResolvedPackage {
    /// The selected package identity.
    pub key: PackageKey,
    /// The versions this query selects, after version/branch filtering.
    pub versions: Vec<Version>,
    /// The architecture selector the query carried, if any.
    pub arch: Option<Arch>,
}

/// Result of resolving package queries.
#[derive(Debug)]
pub struct QueryResolution {
    /// Resolved queries, in query order.
    pub resolved: Vec<ResolvedPackage>,
    /// Queries that matched no package.
    pub no_match: Vec<String>,
}

/// A parsed package query: `name[:arch][=version|/branch]`.
#[derive(Debug, Clone, Copy)]
struct Query<'a> {
    name: &'a str,
    arch: Option<&'a str>,
    version: Option<&'a str>,
    branch: Option<&'a str>,
}

impl<'a> Query<'a> {
    fn parse(query: &'a str) -> Self {
        let (name, version) = match query.split_once('=') {
            Some((name, version)) => (name, Some(version)),
            None => (query, None),
        };
        let (name, branch) = match name.split_once('/') {
            Some((name, branch)) => (name, Some(branch)),
            None => (name, None),
        };
        let (name, arch) = match name.rsplit_once(':') {
            Some((name, arch)) => (name, Some(arch)),
            None => (name, None),
        };
        Self {
            name,
            arch,
            version,
            branch,
        }
    }
}

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
        let status_path = PathBuf::from(cfg.get_file("Dir::State::status", "var/lib/dpkg/status"));
        let lookup = SourceLookup::build(cfg);

        let (entries, sources) = crate::apt_lists::parse_apt_lists_dir_with_sources(
            &lists_dir,
            &lookup,
            &cfg.architectures(),
        )?;

        let mut state = Self {
            native,
            architectures,
            groups: HashMap::new(),
            package_order: HashMap::new(),
            installed: HashMap::new(),
            status_path,
        };
        state.add_index(entries, sources);
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

    /// The group with this package name, if any.
    pub fn group<'a>(&'a self, name: &str) -> Option<Group<'a>> {
        let (name, _) = self.groups.get_key_value(name)?;
        Some(Group { state: self, name })
    }

    /// The package built for `arch`, if any.
    pub fn package<'a>(&'a self, name: &str, arch: &Arch) -> Option<Package<'a>> {
        let (name, packages) = self.groups.get_key_value(name)?;
        let (arch, _) = packages.get_key_value(arch.as_str())?;
        Some(Package {
            state: self,
            name,
            arch: arch.clone(),
        })
    }

    /// All package groups, in no particular order.
    pub fn groups<'a>(&'a self) -> impl Iterator<Item = Group<'a>> + 'a {
        self.groups.keys().map(|name| Group { state: self, name })
    }

    /// The installed version of `key`, if the status file has one.
    pub fn installed_version<'a>(&'a self, key: &PackageKey) -> Option<&'a str> {
        self.installed.get(key).map(String::as_str)
    }

    /// Whether `key` is installed.
    pub fn is_installed(&self, key: &PackageKey) -> bool {
        self.installed.contains_key(key)
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
        let source = crate::apt_db::local_deb_source(path.as_ref());
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

    fn packages<'a>(&'a self, name: &str) -> &'a HashMap<Arch, Vec<Version>> {
        static EMPTY: std::sync::OnceLock<HashMap<Arch, Vec<Version>>> = std::sync::OnceLock::new();
        self.groups
            .get(name)
            .unwrap_or_else(|| EMPTY.get_or_init(HashMap::new))
    }

    fn preferred_arch(&self, name: &str) -> Option<Arch> {
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
        let Ok(paragraphs) = crate::dpkg::paragraphs(&self.status_path) else {
            return;
        };
        let source = Source::dpkg_status(self.status_path.display().to_string());

        for paragraph in paragraphs.filter_map(Result::ok) {
            let Ok(pkg) = crate::dpkg::DpkgPackage::from_paragraph(&paragraph) else {
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

fn matches_version(version: &Version, exact: Option<&str>, branch: Option<&str>) -> bool {
    if let Some(exact) = exact {
        return version.version() == Some(exact);
    }
    if let Some(branch) = branch {
        return version.sources.iter().any(|source| source.suite == branch);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn control(name: &str, version: &str, arch: &str, description: &str) -> Control {
        crate::deb::parse_control_entry(&format!(
            "Package: {name}\nVersion: {version}\nArchitecture: {arch}\nDescription: {description}\n\n"
        ))
        .unwrap()
    }

    fn source(arch: &str, suite: &str) -> Source {
        Source {
            base_url: "file:/repo".to_string(),
            suite: suite.to_string(),
            component: Some("main".to_string()),
            arch: Some(arch.to_string()),
        }
    }

    fn state(native: &str, archs: &[&str]) -> AptState {
        AptState {
            native: Arch::new(native),
            architectures: archs.iter().map(|arch| Arch::new(*arch)).collect(),
            groups: HashMap::new(),
            package_order: HashMap::new(),
            installed: HashMap::new(),
            status_path: PathBuf::new(),
        }
    }

    fn add(state: &mut AptState, name: &str, version: &str, arch: &str) {
        state.add_index(
            vec![control(name, version, arch, "test")],
            vec![source(arch, "stable")],
        );
    }

    fn versions(package: &Package<'_>) -> Vec<(String, String)> {
        package
            .versions()
            .iter()
            .map(|version| {
                (
                    version.version().unwrap_or_default().to_string(),
                    version.arch().as_str().to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn all_is_a_version_flag_not_a_package_arch() {
        let mut state = state("amd64", &["amd64", "i386"]);
        add(&mut state, "foo", "1", "amd64");
        add(&mut state, "foo", "2", "i386");
        add(&mut state, "foo", "3", "all");

        let group = state.group("foo").unwrap();
        let mut arches: Vec<_> = group.packages().map(|pkg| pkg.arch().to_string()).collect();
        arches.sort();
        assert_eq!(arches, vec!["amd64", "i386"]);

        let native = state.package("foo", &Arch::new("amd64")).unwrap();
        assert_eq!(
            versions(&native),
            vec![
                ("1".to_string(), "amd64".to_string()),
                ("3".to_string(), "all".to_string())
            ]
        );
        let foreign = state.package("foo", &Arch::new("i386")).unwrap();
        assert_eq!(
            versions(&foreign),
            vec![("2".to_string(), "i386".to_string())]
        );
    }

    #[test]
    fn resolve_uses_apt_architecture_views() {
        let mut state = state("amd64", &["amd64", "i386"]);
        add(&mut state, "foo", "1", "amd64");
        add(&mut state, "foo", "2", "i386");
        add(&mut state, "foo", "3", "all");
        add(&mut state, "bar", "1", "i386");
        add(&mut state, "baz", "1", "all");

        assert_eq!(
            state.resolve(vec!["foo".into()]).resolved[0]
                .key
                .arch
                .as_str(),
            "amd64"
        );
        for query in ["foo:native", "foo:any", "foo:all", "foo:amd64"] {
            let resolution = state.resolve(vec![query.into()]);
            assert_eq!(resolution.resolved[0].key.arch.as_str(), "amd64", "{query}");
        }
        let foreign = state.resolve(vec!["foo:i386".into()]);
        assert_eq!(foreign.resolved[0].key.arch.as_str(), "i386");

        // A package only a foreign architecture has falls back for a plain
        // query, but `:native`/`:all` do not.
        let fallback = state.resolve(vec!["bar".into()]);
        assert_eq!(fallback.resolved[0].key.arch.as_str(), "i386");
        for query in ["bar:native", "bar:all"] {
            assert_eq!(state.resolve(vec![query.into()]).no_match, vec![query]);
        }

        // An `all`-only package belongs to the native package.
        let all_only = state.resolve(vec!["baz".into()]);
        assert_eq!(all_only.resolved[0].key.arch.as_str(), "amd64");
        assert_eq!(all_only.resolved[0].versions[0].arch().as_str(), "all");
        assert_eq!(
            state.resolve(vec!["baz:i386".into()]).no_match,
            vec!["baz:i386"]
        );
    }

    #[test]
    fn version_and_branch_queries_filter_one_package() {
        let mut state = state("amd64", &["amd64"]);
        add(&mut state, "foo", "1", "amd64");
        state.add_index(
            vec![control("foo", "2", "amd64", "preview")],
            vec![source("amd64", "preview")],
        );

        let exact = state.resolve(vec!["foo=1".into()]);
        assert_eq!(exact.resolved[0].versions.len(), 1);
        assert_eq!(exact.resolved[0].versions[0].version(), Some("1"));

        let branch = state.resolve(vec!["foo/preview".into()]);
        assert_eq!(branch.resolved[0].versions.len(), 1);
        assert_eq!(branch.resolved[0].versions[0].version(), Some("2"));
    }

    #[test]
    fn status_merges_into_packages_and_adds_missing_ones() {
        let mut state = state("amd64", &["amd64", "i386"]);
        add(&mut state, "foo", "1", "amd64");

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("status");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(
            b"Package: foo\nVersion: 1\nArchitecture: amd64\nStatus: install ok installed\n\n\
Package: vendor-tool\nVersion: 7\nArchitecture: amd64\nStatus: install ok installed\n\n\
Package: vendor-tool\nVersion: 7\nArchitecture: i386\nStatus: install ok installed\n\n\
Package: removed-tool\nVersion: 1\nArchitecture: amd64\nStatus: deinstall ok config-files\n\n",
        )
        .unwrap();
        state.status_path = path;
        state.merge_status();

        let foo = state.package("foo", &Arch::new("amd64")).unwrap();
        assert_eq!(foo.installed_version(), Some("1"));
        assert!(
            foo.versions()
                .iter()
                .any(|version| version.sources.iter().any(Source::is_dpkg_status))
        );

        let plain = state.resolve(vec!["vendor-tool".into()]);
        assert_eq!(plain.resolved[0].key.arch.as_str(), "amd64");
        assert!(state.is_installed(&plain.resolved[0].key));
        assert_eq!(
            state.resolve(vec!["vendor-tool:i386".into()]).resolved[0]
                .key
                .arch
                .as_str(),
            "i386"
        );
        assert_eq!(
            state.resolve(vec!["removed-tool".into()]).no_match,
            vec!["removed-tool"]
        );
    }
}
