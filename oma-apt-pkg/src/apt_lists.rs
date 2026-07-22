use std::collections::HashMap;
use std::path::Path;

use deb822_fast::{Deb822, FromDeb822, FromDeb822Paragraph};
use serde::{Deserialize, Serialize};

/// Errors that can occur when parsing APT list files.
#[derive(Debug, thiserror::Error)]
pub enum AptListsError {
    #[error("Failed to read file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse deb822 data in {path}: {detail}")]
    Parse { path: String, detail: String },
    #[error("No Packages files found in directory: {0}")]
    NoPackagesFiles(String),
}

/// A single package entry from a Packages file (one paragraph).
#[derive(Debug, Clone, Serialize, Deserialize, FromDeb822)]
pub struct PackageEntry {
    pub package: String,
    pub version: Option<String>,
    pub architecture: Option<String>,
    pub description: Option<String>,
    #[deb822(field = "Description-md5")]
    pub description_md5: Option<String>,
    pub maintainer: Option<String>,
    #[deb822(field = "Installed-Size")]
    pub installed_size: Option<u64>,
    pub depends: Option<String>,
    #[deb822(field = "Pre-Depends")]
    pub pre_depends: Option<String>,
    pub recommends: Option<String>,
    pub suggests: Option<String>,
    pub breaks: Option<String>,
    pub conflicts: Option<String>,
    pub replaces: Option<String>,
    pub provides: Option<String>,
    pub section: Option<String>,
    pub priority: Option<String>,
    pub homepage: Option<String>,
    #[deb822(field = "Multi-Arch")]
    pub multi_arch: Option<String>,
    pub filename: Option<String>,
    pub size: Option<u64>,
    #[deb822(field = "SHA256")]
    pub sha256: Option<String>,
}

/// Parsed contents of a single Packages file.
#[derive(Debug, Clone)]
pub struct PackagesFile {
    /// The source filename (e.g. `archive_dists_sid_main_binary-amd64_Packages`).
    pub source: String,
    pub entries: Vec<PackageEntry>,
}

/// Scan `/var/lib/apt/lists/` and parse all `*_Packages` files.
///
/// Returns a flat list of all package entries across all repos/components/archs.
pub fn parse_apt_lists_dir(path: impl AsRef<Path>) -> Result<Vec<PackageEntry>, AptListsError> {
    let dir = path.as_ref();
    let mut all_packages = Vec::new();

    let mut found_any = false;

    for entry in std::fs::read_dir(dir).map_err(AptListsError::Io)? {
        let entry = entry.map_err(AptListsError::Io)?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        if !name.ends_with("_Packages") {
            continue;
        }

        found_any = true;
        let entries = parse_single_packages_file(entry.path())?;
        all_packages.extend(entries);
    }

    if !found_any {
        return Err(AptListsError::NoPackagesFiles(
            dir.to_string_lossy().to_string(),
        ));
    }

    Ok(all_packages)
}

/// Parse a single `*_Packages` file (deb822 format).
pub fn parse_single_packages_file(
    path: impl AsRef<Path>,
) -> Result<Vec<PackageEntry>, AptListsError> {
    let path = path.as_ref();
    let content = std::fs::read_to_string(path).map_err(AptListsError::Io)?;

    let deb822: Deb822 = content
        .parse()
        .map_err(|e: deb822_fast::Error| AptListsError::Parse {
            path: path.to_string_lossy().to_string(),
            detail: e.to_string(),
        })?;

    let entries: Vec<PackageEntry> = deb822
        .iter()
        .filter_map(|p| PackageEntry::from_paragraph(p).ok())
        .collect();

    Ok(entries)
}

/// Build a description cache map (package name → summary) from parsed entries.
pub fn build_description_map(entries: &[PackageEntry]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for entry in entries {
        if let Some(ref desc) = entry.description {
            // Description field format: first line is the summary
            let summary = desc.lines().next().unwrap_or(desc);
            map.entry(entry.package.clone())
                .or_insert_with(|| summary.to_string());
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_paragraph() {
        let input = "\
Package: zoxide
Version: 0.9.6-1
Architecture: amd64
Maintainer: AOSC Maintainers <maintainers@aosc.io>
Description: A smarter cd command for your terminal
Depends: libc6

";
        let deb822: Deb822 = input.parse().unwrap();
        let entry = PackageEntry::from_paragraph(deb822.iter().next().unwrap()).unwrap();

        assert_eq!(entry.package, "zoxide");
        assert_eq!(entry.version.as_deref(), Some("0.9.6-1"));
        assert_eq!(entry.architecture.as_deref(), Some("amd64"));
        assert_eq!(
            entry.description.as_deref(),
            Some("A smarter cd command for your terminal")
        );
        assert_eq!(entry.depends.as_deref(), Some("libc6"));
    }

    #[test]
    fn test_parse_multiple_paragraphs() {
        let input = "\
Package: foo
Version: 1.0
Description: Foo package
Status: install ok installed

Package: bar
Version: 2.0
Description: Bar package
Status: deinstall ok config-files

";
        let deb822: Deb822 = input.parse().unwrap();
        let entries: Vec<PackageEntry> = deb822
            .iter()
            .filter_map(|p| PackageEntry::from_paragraph(p).ok())
            .collect();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].package, "foo");
        assert_eq!(entries[1].package, "bar");
    }

    #[test]
    fn test_build_description_map() {
        let entries = vec![
            PackageEntry {
                package: "foo".into(),
                version: Some("1.0".into()),
                architecture: None,
                description: Some("First line\nSecond line".into()),
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
            },
            PackageEntry {
                package: "bar".into(),
                version: Some("2.0".into()),
                architecture: None,
                description: Some("Bar description".into()),
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
            },
        ];

        let map = build_description_map(&entries);
        assert_eq!(map.get("foo").map(|s| s.as_str()), Some("First line"));
        assert_eq!(map.get("bar").map(|s| s.as_str()), Some("Bar description"));
    }

    #[test]
    fn test_parse_all_fields() {
        let input = "\
Package: bash
Version: 5.2.37-1
Architecture: amd64
Section: shells
Priority: optional
Maintainer: AOSC Maintainers <maintainers@aosc.io>
Installed-Size: 2048
Depends: libc6 (>= 2.38), ncurses (>= 6.5)
Pre-Depends: libc6
Recommends: bash-completion
Suggests: bash-doc
Breaks: old-bash (<< 5.0)
Conflicts: bash-pre-v2
Replaces: bash-compat
Provides: sh
Homepage: https://www.gnu.org/software/bash/
Multi-Arch: foreign
Description: GNU Bourne Again SHell
 The standard shell for GNU/Linux systems.
Description-md5: abc123
Filename: pool/main/b/bash/bash_5.2.37-1_amd64.deb
Size: 1234567
SHA256: deadbeef1234567890abcdef01234567890abcdef01234567890abcdef012345678

";
        let deb822: Deb822 = input.parse().unwrap();
        let entry = PackageEntry::from_paragraph(deb822.iter().next().unwrap()).unwrap();

        assert_eq!(entry.package, "bash");
        assert_eq!(entry.version.as_deref(), Some("5.2.37-1"));
        assert_eq!(entry.architecture.as_deref(), Some("amd64"));
        assert_eq!(entry.section.as_deref(), Some("shells"));
        assert_eq!(entry.priority.as_deref(), Some("optional"));
        assert_eq!(entry.maintainer.as_deref(), Some("AOSC Maintainers <maintainers@aosc.io>"));
        assert_eq!(entry.installed_size, Some(2048));
        assert_eq!(entry.depends.as_deref(), Some("libc6 (>= 2.38), ncurses (>= 6.5)"));
        assert_eq!(entry.pre_depends.as_deref(), Some("libc6"));
        assert_eq!(entry.recommends.as_deref(), Some("bash-completion"));
        assert_eq!(entry.suggests.as_deref(), Some("bash-doc"));
        assert_eq!(entry.breaks.as_deref(), Some("old-bash (<< 5.0)"));
        assert_eq!(entry.conflicts.as_deref(), Some("bash-pre-v2"));
        assert_eq!(entry.replaces.as_deref(), Some("bash-compat"));
        assert_eq!(entry.provides.as_deref(), Some("sh"));
        assert_eq!(entry.homepage.as_deref(), Some("https://www.gnu.org/software/bash/"));
        assert_eq!(entry.multi_arch.as_deref(), Some("foreign"));
        assert_eq!(entry.description.as_deref(), Some("GNU Bourne Again SHell\nThe standard shell for GNU/Linux systems."));
        assert_eq!(entry.filename.as_deref(), Some("pool/main/b/bash/bash_5.2.37-1_amd64.deb"));
        assert_eq!(entry.size, Some(1234567));
        assert_eq!(entry.sha256.as_deref(), Some("deadbeef1234567890abcdef01234567890abcdef01234567890abcdef012345678"));
    }

    #[test]
    fn test_parse_optional_fields_absent() {
        // Minimal paragraph — only required field
        let input = "Package: minimal\n\n";
        let deb822: Deb822 = input.parse().unwrap();
        let entry = PackageEntry::from_paragraph(deb822.iter().next().unwrap()).unwrap();

        assert_eq!(entry.package, "minimal");
        assert!(entry.version.is_none());
        assert!(entry.architecture.is_none());
        assert!(entry.description.is_none());
        assert!(entry.maintainer.is_none());
        assert!(entry.installed_size.is_none());
        assert!(entry.depends.is_none());
        assert!(entry.pre_depends.is_none());
        assert!(entry.recommends.is_none());
        assert!(entry.suggests.is_none());
        assert!(entry.breaks.is_none());
        assert!(entry.conflicts.is_none());
        assert!(entry.replaces.is_none());
        assert!(entry.provides.is_none());
        assert!(entry.section.is_none());
        assert!(entry.priority.is_none());
        assert!(entry.homepage.is_none());
        assert!(entry.multi_arch.is_none());
        assert!(entry.filename.is_none());
        assert!(entry.size.is_none());
        assert!(entry.sha256.is_none());
    }
}
