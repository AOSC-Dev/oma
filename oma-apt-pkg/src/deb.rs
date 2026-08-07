//! Extract and parse the `control` file from a local `.deb` package.
//!
//! A `.deb` package is an `ar` archive containing three members:
//! `debian-binary`, `control.tar.{gz,xz,zst,…}` and `data.tar.{gz,xz,zst,…}`.
//! This module extracts the `./control` file out of the compressed
//! `control.tar` member and parses it into a [`PackageEntry`] using the
//! `debian-control` crate.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use ar::Archive;
use thiserror::Error;

use crate::apt_lists::PackageEntry;

/// Errors that can occur while reading a `.deb` package.
#[derive(Debug, Error)]
pub enum DebError {
    #[error("Failed to read file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Missing control.tar member in .deb archive")]
    MissingControlTar,
    #[error("Missing ./control entry in control.tar")]
    MissingControl,
    #[error("Unsupported control.tar compression: {0}")]
    UnsupportedCompression(String),
    #[error("Invalid control file: {0}")]
    Parse(String),
}

/// Read and parse the `control` file from a `.deb` package into a
/// [`PackageEntry`].
pub fn parse_deb(path: impl AsRef<Path>) -> Result<PackageEntry, DebError> {
    parse_deb_reader(File::open(path)?)
}

/// Parse a `.deb` package from any reader (e.g. in-memory bytes) into a
/// [`PackageEntry`].
pub fn parse_deb_reader(reader: impl Read) -> Result<PackageEntry, DebError> {
    let control = read_control_from_reader(reader)?;
    parse_control_entry(&control)
}

/// Read the raw `control` file content out of a `.deb` package.
pub fn read_control_from_deb(path: impl AsRef<Path>) -> Result<String, DebError> {
    read_control_from_reader(File::open(path)?)
}

/// Read the raw `control` file content from any reader (e.g. in-memory
/// bytes).
pub fn read_control_from_reader(reader: impl Read) -> Result<String, DebError> {
    let mut archive = Archive::new(reader);
    while let Some(entry) = archive.next_entry() {
        let entry = entry?;
        let name = String::from_utf8_lossy(entry.header().identifier()).into_owned();
        if name.starts_with("control.tar") {
            let tar_bytes = decompress_control_tar(&name, entry)?;
            return read_control_from_tar(&tar_bytes);
        }
    }

    Err(DebError::MissingControlTar)
}

/// Parse the `control` file text into a [`PackageEntry`] using
/// `debian-control`'s lossy APT model.
pub fn parse_control_entry(control: &str) -> Result<PackageEntry, DebError> {
    use debian_control::lossy::apt::Package;

    let pkg: Package = control.parse().map_err(DebError::Parse)?;

    Ok(PackageEntry {
        package: pkg.name,
        version: Some(pkg.version.to_string()),
        architecture: Some(pkg.architecture),
        description: pkg.description,
        description_md5: pkg.description_md5,
        maintainer: pkg.maintainer,
        installed_size: pkg.installed_size.map(|s| s as u64),
        depends: pkg.depends.map(|r| r.to_string()),
        pre_depends: pkg.pre_depends.map(|r| r.to_string()),
        recommends: pkg.recommends.map(|r| r.to_string()),
        suggests: pkg.suggests.map(|r| r.to_string()),
        breaks: pkg.breaks.map(|r| r.to_string()),
        conflicts: pkg.conflicts.map(|r| r.to_string()),
        replaces: pkg.replaces.map(|r| r.to_string()),
        provides: pkg.provides.map(|r| r.to_string()),
        section: pkg.section,
        priority: pkg.priority.map(|p| p.to_string()),
        homepage: pkg.homepage,
        multi_arch: pkg.multi_arch.map(|m| m.to_string()),
        filename: pkg.filename,
        size: pkg.size.map(|s| s as u64),
        sha256: pkg.sha256,
    })
}

/// Decompress a `control.tar.*` member into raw tar bytes.
fn decompress_control_tar(name: &str, reader: impl Read) -> Result<Vec<u8>, DebError> {
    let mut out = Vec::new();
    if name.ends_with(".gz") {
        flate2::read::GzDecoder::new(reader).read_to_end(&mut out)?;
    } else if name.ends_with(".xz") {
        liblzma::read::XzDecoder::new(reader).read_to_end(&mut out)?;
    } else if name.ends_with(".zst") {
        zstd::stream::read::Decoder::new(reader)?.read_to_end(&mut out)?;
    } else if name.ends_with(".tar") {
        reader.take(u64::MAX).read_to_end(&mut out)?;
    } else {
        return Err(DebError::UnsupportedCompression(name.to_string()));
    }

    Ok(out)
}

/// Extract the `./control` file from a tar archive.
fn read_control_from_tar(tar_bytes: &[u8]) -> Result<String, DebError> {
    let mut archive = tar::Archive::new(tar_bytes);
    for entry in archive.entries()? {
        let mut entry = entry?;
        if entry
            .path()?
            .file_name()
            .is_some_and(|name| name == "control")
        {
            let mut content = String::new();
            entry.read_to_string(&mut content)?;
            return Ok(content);
        }
    }

    Err(DebError::MissingControl)
}

/// Test helper: build a minimal `.deb` archive in memory.
#[cfg(test)]
pub(crate) mod test_util {
    /// Build a `control.tar.gz` containing the given control text.
    pub fn control_tar_gz(control: &str) -> Vec<u8> {
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        {
            let mut tar = tar::Builder::new(&mut gz);
            let mut header = tar::Header::new_ustar();
            // `append_data` does not set the size field itself.
            header.set_size(control.len() as u64);
            header.set_mode(0o644);
            tar.append_data(&mut header, "./control", control.as_bytes())
                .unwrap();
            tar.finish().unwrap();
        }
        gz.finish().unwrap()
    }

    /// Build a complete `.deb` archive as bytes using the `ar` crate.
    pub fn build_deb(control: &str) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut builder = ar::Builder::new(&mut out);
            builder
                .append(
                    &ar::Header::new(b"debian-binary".to_vec(), 4),
                    &b"2.0\n"[..],
                )
                .unwrap();
            let ctrl = control_tar_gz(control);
            builder
                .append(
                    &ar::Header::new(b"control.tar.gz".to_vec(), ctrl.len() as u64),
                    &ctrl[..],
                )
                .unwrap();
            let data = control_tar_gz("# dummy\n");
            builder
                .append(
                    &ar::Header::new(b"data.tar.gz".to_vec(), data.len() as u64),
                    &data[..],
                )
                .unwrap();
        }
        out
    }

    /// Control file for the `hello` test package.
    pub const CONTROL: &str = concat!(
        "Package: hello\n",
        "Version: 2.10-2\n",
        "Architecture: amd64\n",
        "Maintainer: Example <ex@example.com>\n",
        "Installed-Size: 123\n",
        "Depends: libc6 (>= 2.2.5)\n",
        "Section: devel\n",
        "Priority: optional\n",
        "Multi-Arch: foreign\n",
        "Homepage: https://example.com\n",
        "Description: A test package\n",
        " Long description line.\n",
    );
}

#[cfg(test)]
mod tests {
    use super::test_util::*;
    use super::*;

    #[test]
    fn build_deb_members() {
        let deb = build_deb(CONTROL);
        let mut archive = ar::Archive::new(&deb[..]);
        let mut names = Vec::new();

        while let Some(entry) = archive.next_entry() {
            let entry = entry.unwrap();
            names.push(String::from_utf8_lossy(entry.header().identifier()).into_owned());
        }
        assert_eq!(
            names,
            vec!["debian-binary", "control.tar.gz", "data.tar.gz"]
        );
    }

    #[test]
    fn read_control_from_reader_extracts_control() {
        let control = read_control_from_reader(&build_deb(CONTROL)[..]).unwrap();
        assert!(control.contains("Package: hello"));
        assert!(control.contains("Depends: libc6 (>= 2.2.5)"));
    }

    #[test]
    fn parse_control_entry_maps_fields() {
        let entry = parse_control_entry(CONTROL).unwrap();
        assert_eq!(entry.package, "hello");
        assert_eq!(entry.version.as_deref(), Some("2.10-2"));
        assert_eq!(entry.architecture.as_deref(), Some("amd64"));
        assert_eq!(
            entry.maintainer.as_deref(),
            Some("Example <ex@example.com>")
        );
        assert_eq!(entry.installed_size, Some(123));
        assert_eq!(entry.depends.as_deref(), Some("libc6 (>= 2.2.5)"));
        assert_eq!(entry.section.as_deref(), Some("devel"));
        assert_eq!(entry.priority.as_deref(), Some("optional"));
        assert_eq!(entry.multi_arch.as_deref(), Some("foreign"));
        assert_eq!(entry.homepage.as_deref(), Some("https://example.com"));
        assert!(entry.description.unwrap().starts_with("A test package"));
    }

    #[test]
    fn parse_deb_reader_end_to_end() {
        let entry = parse_deb_reader(&build_deb(CONTROL)[..]).unwrap();
        assert_eq!(entry.package, "hello");
        assert_eq!(entry.version.as_deref(), Some("2.10-2"));
    }
}
