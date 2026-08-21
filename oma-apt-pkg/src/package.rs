//! Package-level view over the APT package database — one [`Package`] per
//! package name, mirroring apt's `pkgCache::PkgIterator` / rust-apt's
//! `Package` parent.
//!
//! A [`Package`] is the "parent" of the versions in the database: it borrows
//! the [`AptDb`] and a package name, and exposes package-level information —
//! name, installed state, candidate version, display fields — without the
//! caller having to reach into a specific [`PackageVersion`] first. Version
//! access still goes through [`Package::versions`] /
//! [`Package::candidate`].
//!
//! Since `AptDb`, `DpkgState` and `AptExtendedStates` are separate objects
//! (unlike apt's `Cache`, which bakes dpkg state in), the state methods take
//! `&DpkgState` / `&AptExtendedStates` as arguments.

use std::borrow::Cow;
use std::str::FromStr;

use crate::{AptDb, AptExtendedStates, DpkgState, PackageEntry, PackageVersion};

/// A package in the database: the per-name parent of its versions.
///
/// Construct via [`AptDb::package`] (one named package) or
/// [`AptDb::packages_iter`] (all packages).
#[derive(Debug)]
pub struct Package<'a> {
    apt_db: &'a AptDb,
    name: Cow<'a, str>,
}

impl<'a> Package<'a> {
    /// Build a package view. The name is borrowed when `name` already lives
    /// `'a` (e.g. a database map key) and owned otherwise — so iterating
    /// [`AptDb::packages_iter`] never allocates.
    pub(crate) fn new(apt_db: &'a AptDb, name: impl Into<Cow<'a, str>>) -> Self {
        Self {
            apt_db,
            name: name.into(),
        }
    }

    /// The package name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The display name of the representative (candidate) version,
    /// `name:arch` — the pretty form omits the `:arch` qualifier for the
    /// native architecture and `all` (see [`PackageEntry::fullname`]). To
    /// name a specific version (e.g. the one a query filtered to), use
    /// [`Self::fullname_of`].
    pub fn fullname(&self, pretty: bool) -> Cow<'_, str> {
        match self.representative() {
            Some(Cow::Borrowed(version)) => self.apt_db.fullname(&version.entry, pretty),
            Some(Cow::Owned(version)) => self
                .apt_db
                .fullname(&version.entry, pretty)
                .into_owned()
                .into(),
            None => Cow::Borrowed(self.name.as_ref()),
        }
    }

    /// The display name of a specific version, `name:arch` — uses the
    /// version's own architecture, unlike [`Self::fullname`] which takes
    /// the package-wide candidate's. Lets an architecture-filtered query
    /// (e.g. `foo:i386`) or an `--all` listing label every displayed block
    /// with the architecture that block actually shows.
    pub fn fullname_of<'b>(&self, version: &'b PackageVersion, pretty: bool) -> Cow<'b, str> {
        self.apt_db.fullname(&version.entry, pretty)
    }

    /// Number of distinct versions in the database (a version shared by
    /// several sources counts once).
    pub fn version_count(&self) -> usize {
        self.apt_db.version_count(&self.name)
    }

    /// All versions of this package.
    pub fn versions(&self) -> Cow<'a, [PackageVersion]> {
        self.apt_db.versions(&self.name)
    }

    /// The candidate version (highest version), like apt's
    /// `PkgIterator::CandidateVer`.
    pub fn candidate(&self) -> Option<Cow<'a, PackageVersion>> {
        match self.versions() {
            Cow::Borrowed(versions) => versions
                .iter()
                .max_by_key(|v| v.parsed_version())
                .map(Cow::Borrowed),
            Cow::Owned(versions) => versions
                .iter()
                .max_by_key(|v| v.parsed_version())
                .map(|v| Cow::Owned(v.clone())),
        }
    }

    /// The version matching `version` exactly, if present.
    pub fn get_version(&self, version: &str) -> Option<Cow<'a, PackageVersion>> {
        match self.versions() {
            Cow::Borrowed(versions) => versions
                .iter()
                .find(|v| v.entry.version.as_deref() == Some(version))
                .map(Cow::Borrowed),
            Cow::Owned(versions) => versions
                .into_iter()
                .find(|v| v.entry.version.as_deref() == Some(version))
                .map(Cow::Owned),
        }
    }

    /// Whether the package is currently installed.
    pub fn is_installed(&self, dpkg: &DpkgState) -> bool {
        dpkg.is_installed(&self.name)
    }

    /// The installed version string, if any.
    pub fn installed_version<'b>(&self, dpkg: &'b DpkgState) -> Option<&'b str> {
        dpkg.installed_version(&self.name)
    }

    /// Whether the package was installed automatically as a dependency.
    pub fn is_auto_installed(&self, dpkg: &DpkgState, ext: &AptExtendedStates) -> bool {
        dpkg.is_installed(&self.name) && ext.is_auto_installed(&self.name)
    }

    /// Whether a newer version than the installed one is available in the
    /// database. Uses proper Debian version comparison (epochs, `~`, ...),
    /// falling back to a directional string comparison (`cand > installed`)
    /// when a version fails to parse.
    pub fn is_upgradable(&self, dpkg: &DpkgState) -> bool {
        let Some(installed) = dpkg.installed_version(&self.name) else {
            return false;
        };
        let Some(candidate) = self.candidate() else {
            return false;
        };
        let Some(cand) = candidate.entry.version.as_deref() else {
            return false;
        };
        match (
            debversion::Version::from_str(cand),
            debversion::Version::from_str(installed),
        ) {
            (Ok(cand), Ok(installed)) => cand > installed,
            // Directional string fallback: only an unparsable candidate
            // that sorts above the installed string counts as an upgrade.
            _ => cand > installed,
        }
    }

    /// The architecture of the package (from the candidate version, falling
    /// back to the first version).
    pub fn arch(&self) -> Option<Cow<'a, str>> {
        self.version_field(|e| e.architecture.as_deref())
    }

    /// The section of the package (from the candidate version, falling back
    /// to the first version).
    pub fn section(&self) -> Option<Cow<'a, str>> {
        self.version_field(|e| e.section.as_deref())
    }

    /// The priority of the package (from the candidate version, falling back
    /// to the first version).
    pub fn priority(&self) -> Option<Cow<'a, str>> {
        self.version_field(|e| e.priority.as_deref())
    }

    /// The one-line summary (first line of the description), like apt's
    /// `Summary()`. From the candidate version, falling back to the first
    /// version.
    pub fn short_description(&self) -> Option<Cow<'a, str>> {
        self.version_field(|e| {
            e.description
                .as_deref()
                .map(|d| d.lines().next().unwrap_or(d))
        })
    }

    /// The candidate version, or the first version when there is no
    /// candidate — the "representative" version display fields are read
    /// from, mirroring apt's package-level convenience accessors.
    fn representative(&self) -> Option<Cow<'a, PackageVersion>> {
        match self.candidate() {
            Some(candidate) => Some(candidate),
            None => match self.versions() {
                Cow::Borrowed(versions) => versions.first().map(Cow::Borrowed),
                Cow::Owned(versions) => versions.into_iter().next().map(Cow::Owned),
            },
        }
    }

    /// Read a package-level field from the representative version, borrowing
    /// when the database is owned and owning when it comes from the
    /// memory-mapped archive.
    fn version_field(
        &self,
        pick: impl for<'x> Fn(&'x PackageEntry) -> Option<&'x str>,
    ) -> Option<Cow<'a, str>> {
        match self.representative()? {
            Cow::Borrowed(version) => pick(&version.entry).map(Cow::Borrowed),
            Cow::Owned(version) => pick(&version.entry).map(|s| Cow::Owned(s.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const STATUS: &str = "\
Package: fish
Version: 3.6
Architecture: amd64
Status: install ok installed

Package: apt
Version: 2.4
Architecture: amd64
Status: install ok installed

Package: zsh
Version: 5.9
Architecture: amd64
Status: install ok installed
";

    const EXTENDED: &str = "\
Package: fish
Architecture: amd64
Auto-Installed: 1
";

    fn entry(name: &str, version: &str) -> PackageEntry {
        PackageEntry {
            package: name.to_string(),
            version: Some(version.to_string()),
            architecture: Some("amd64".to_string()),
            section: Some("utils".to_string()),
            priority: Some("optional".to_string()),
            description: Some("A test package.\nLonger description.".to_string()),
            ..PackageEntry {
                package: String::new(),
                version: None,
                architecture: None,
                description: None,
                description_md5: None,
                maintainer: None,
                installed_size: None,
                depends: None,
                pre_depends: None,
                recommends: None,
                suggests: None,
                breaks: None,
                conflicts: None,
                replaces: None,
                provides: None,
                section: None,
                priority: None,
                homepage: None,
                multi_arch: None,
                filename: None,
                size: None,
                sha256: None,
            }
        }
    }

    fn db() -> AptDb {
        AptDb::from_entries(
            "amd64",
            vec![
                entry("fish", "3.6"),
                entry("fish", "3.7"),
                entry("apt", "2.5"),
                entry("zsh", "5.9"),
                entry("vim", "9.0"),
            ],
        )
    }

    fn write_status() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::File::create(dir.path().join("status"))
            .unwrap()
            .write_all(STATUS.as_bytes())
            .unwrap();
        std::fs::File::create(dir.path().join("extended_states"))
            .unwrap()
            .write_all(EXTENDED.as_bytes())
            .unwrap();
        dir
    }

    #[test]
    fn package_lookup() {
        let db = db();
        let fish = db.package("fish").unwrap();
        assert_eq!(fish.name(), "fish");
        assert!(db.package("nosuchpkg").is_none());
    }

    #[test]
    fn packages_iter_yields_all_names() {
        let db = db();
        let mut names = db
            .packages_iter()
            .map(|p| p.name().to_string())
            .collect::<Vec<_>>();
        names.sort();
        assert_eq!(names, vec!["apt", "fish", "vim", "zsh"]);
    }

    #[test]
    fn versions_and_candidate() {
        let db = db();
        let fish = db.package("fish").unwrap();
        assert_eq!(fish.version_count(), 2);
        assert_eq!(
            fish.candidate().unwrap().entry.version.as_deref(),
            Some("3.7")
        );
        assert_eq!(
            fish.get_version("3.6").unwrap().entry.version.as_deref(),
            Some("3.6")
        );
        assert!(fish.get_version("9.9").is_none());
    }

    #[test]
    fn fullname_pretty_and_plain() {
        let db = db();
        let fish = db.package("fish").unwrap();
        assert_eq!(fish.fullname(true), "fish");
        assert_eq!(fish.fullname(false), "fish:amd64");
    }

    #[test]
    fn display_fields_from_candidate() {
        let db = db();
        let fish = db.package("fish").unwrap();
        assert_eq!(fish.arch().as_deref(), Some("amd64"));
        assert_eq!(fish.section().as_deref(), Some("utils"));
        assert_eq!(fish.priority().as_deref(), Some("optional"));
        assert_eq!(fish.short_description().as_deref(), Some("A test package."));
    }

    #[test]
    fn installed_state_and_upgradable() {
        let dir = write_status();
        let dpkg = DpkgState::from_file(dir.path().join("status")).unwrap();
        let ext = AptExtendedStates::from_file_lazy(dir.path().join("extended_states"));

        let db = db();

        // fish: installed 3.6, candidate 3.7, auto-installed.
        let fish = db.package("fish").unwrap();
        assert!(fish.is_installed(&dpkg));
        assert_eq!(fish.installed_version(&dpkg), Some("3.6"));
        assert!(fish.is_auto_installed(&dpkg, &ext));
        assert!(fish.is_upgradable(&dpkg));

        // apt: installed 2.4, candidate 2.5, manually installed.
        let apt = db.package("apt").unwrap();
        assert!(apt.is_installed(&dpkg));
        assert!(!apt.is_auto_installed(&dpkg, &ext));
        assert!(apt.is_upgradable(&dpkg));

        // zsh: installed 5.9, candidate 5.9 → not upgradable.
        let zsh = db.package("zsh").unwrap();
        assert!(zsh.is_installed(&dpkg));
        assert!(!zsh.is_upgradable(&dpkg));

        // vim: not in dpkg status → not installed, not upgradable.
        let vim = db.package("vim").unwrap();
        assert!(!vim.is_installed(&dpkg));
        assert_eq!(vim.installed_version(&dpkg), None);
        assert!(!vim.is_upgradable(&dpkg));

        // The lazy status reader (used by single-package consumers like
        // `oma show`) must report the same installed versions and upgrade
        // status, not a silent "not upgradable" for every package.
        let lazy = DpkgState::from_file_lazy(dir.path().join("status"));
        let fish = db.package("fish").unwrap();
        assert!(fish.is_installed(&lazy));
        assert_eq!(fish.installed_version(&lazy), Some("3.6"));
        assert!(fish.is_upgradable(&lazy));
        let vim = db.package("vim").unwrap();
        assert_eq!(vim.installed_version(&lazy), None);
        assert!(!vim.is_upgradable(&lazy));
    }

    #[test]
    fn is_upgradable_falls_back_directionally() {
        let dir = tempfile::tempdir().unwrap();
        let status = dir.path().join("status");
        std::fs::write(
            &status,
            "Package: weird\nVersion: 5.0\nArchitecture: amd64\nStatus: install ok installed\n\n",
        )
        .unwrap();
        let dpkg = DpkgState::from_file(&status).unwrap();

        // A malformed candidate (`!` is not a valid Debian version char)
        // that sorts below the installed version is not an upgrade: the
        // string-comparison fallback is directional, not mere inequality.
        let db = AptDb::from_entries("amd64", vec![entry("weird", "1.0!")]);
        assert!(!db.package("weird").unwrap().is_upgradable(&dpkg));

        // A malformed candidate that sorts above the installed version is
        // still reported as an upgrade by the string fallback.
        let db = AptDb::from_entries("amd64", vec![entry("weird", "5.0!")]);
        assert!(db.package("weird").unwrap().is_upgradable(&dpkg));
    }
}
