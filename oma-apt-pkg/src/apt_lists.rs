use std::collections::HashMap;
use std::path::Path;

use deb822_fast::{Deb822, Paragraph};
use serde::Serialize;

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
#[derive(Debug, Clone, Serialize)]
pub struct PackageEntry {
    pub package: String,
    pub version: Option<String>,
    pub architecture: Option<String>,
    pub description: Option<String>,
    pub description_md5: Option<String>,
    pub maintainer: Option<String>,
    pub depends: Option<String>,
    pub provides: Option<String>,
    pub section: Option<String>,
    pub filename: Option<String>,
    pub size: Option<u64>,
    pub sha256: Option<String>,
}

impl PackageEntry {
    fn from_paragraph(para: &Paragraph) -> Option<Self> {
        let package = para.get("Package")?.to_string();

        let size = para.get("Size").and_then(|s| s.parse::<u64>().ok());

        Some(Self {
            package,
            version: para.get("Version").map(String::from),
            architecture: para.get("Architecture").map(String::from),
            description: para.get("Description").map(String::from),
            description_md5: para.get("Description-md5").map(String::from),
            maintainer: para.get("Maintainer").map(String::from),
            depends: para.get("Depends").map(String::from),
            provides: para.get("Provides").map(String::from),
            section: para.get("Section").map(String::from),
            filename: para.get("Filename").map(String::from),
            size,
            sha256: para.get("SHA256").map(String::from),
        })
    }
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
        .filter_map(PackageEntry::from_paragraph)
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
            .filter_map(PackageEntry::from_paragraph)
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
                depends: None,
                provides: None,
                section: None,
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
                depends: None,
                provides: None,
                section: None,
                filename: None,
                size: None,
                sha256: None,
            },
        ];

        let map = build_description_map(&entries);
        assert_eq!(map.get("foo").map(|s| s.as_str()), Some("First line"));
        assert_eq!(map.get("bar").map(|s| s.as_str()), Some("Bar description"));
    }
}
