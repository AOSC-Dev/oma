use std::{borrow::Cow, path::Path};

use aho_corasick::AhoCorasick;
use oma_apt_sources_lists::{SourceEntry, SourceLine, SourceListType, SourcesLists};
use oma_utils::dpkg::dpkg_arch;
use url::Url;

use crate::db::RefreshError;

pub(crate) struct DatabaseFilenameReplacer {
    ac: AhoCorasick,
}

impl DatabaseFilenameReplacer {
    const PATTERNS: &'static [&'static str] = &["_", "/", "+", "%3a", "%3A"];
    const REPLACE_WITH: &'static [&'static str] = &["%5f", "_", "%252b", ":", ":"];

    pub fn new() -> Result<Self, RefreshError> {
        Ok(Self {
            ac: AhoCorasick::new(Self::PATTERNS)?,
        })
    }

    pub fn replace(&self, url: &str) -> Result<String, RefreshError> {
        let url_parsed = Url::parse(url).map_err(|_| RefreshError::InvalidUrl(url.to_string()))?;

        let host = url_parsed.host_str();

        // 不能使用 url_parsed.path()
        // 原因是 "/./" 会被解析器解析为 "/"，而 apt 则不会这样
        let path = if let Some(host) = host {
            url.split_once(host)
                .ok_or_else(|| RefreshError::InvalidUrl(url.to_string()))?
                .1
        } else {
            // file:/// or file:/
            url.strip_prefix("file://")
                .or_else(|| url.strip_prefix("file:"))
                .ok_or_else(|| RefreshError::InvalidUrl(url.to_string()))?
        };

        let url = if let Some(host) = host {
            Cow::Owned(format!("{}{}", host, path))
        } else {
            Cow::Borrowed(path)
        };

        let mut wtr = vec![];

        self.ac
            .try_stream_replace_all(url.as_bytes(), &mut wtr, Self::REPLACE_WITH)
            .map_err(RefreshError::ReplaceAll)?;

        Ok(String::from_utf8_lossy(&wtr).to_string())
    }
}

pub(crate) fn human_download_url(
    ose: &OmaSourceEntry,
    file_name: Option<&str>,
) -> Result<String, RefreshError> {
    let url = Url::parse(&ose.url).map_err(|_| RefreshError::InvalidUrl(ose.url.to_string()))?;

    let host = url.host_str();

    let url = if let Some(host) = host {
        host
    } else {
        url.path()
    };

    let mut s = format!("{}:{}", url, ose.suite);

    if let Some(file_name) = file_name {
        s.push(' ');
        s.push_str(file_name);
    }

    Ok(s)
}

/// Get /etc/apt/sources.list and /etc/apt/sources.list.d
pub(crate) fn get_sources<P: AsRef<Path>>(sysroot: P) -> Result<Vec<OmaSourceEntry>, RefreshError> {
    let mut res = Vec::new();
    let list = SourcesLists::scan_from_root(&sysroot).map_err(RefreshError::ScanSourceError)?;

    for file in list.iter() {
        match file.entries {
            SourceListType::SourceLine(ref lines) => {
                for i in lines {
                    if let SourceLine::Entry(entry) = i {
                        res.push(OmaSourceEntry::new(entry, &sysroot)?);
                    }
                }
            }
            SourceListType::Deb822(ref e) => {
                for i in &e.entries {
                    res.push(OmaSourceEntry::new(i, &sysroot)?);
                }
            }
        }
    }

    Ok(res)
}

#[derive(Debug, Clone)]
pub(crate) struct OmaSourceEntry {
    pub from: OmaSourceEntryFrom,
    pub components: Vec<String>,
    pub url: String,
    pub suite: String,
    pub dist_path: String,
    pub is_flat: bool,
    pub signed_by: Option<String>,
    pub archs: Vec<String>,
    pub trusted: bool,
    pub native_arch: String,
    pub is_source: bool,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum OmaSourceEntryFrom {
    Http,
    Local,
}

impl OmaSourceEntry {
    pub(crate) fn new(v: &SourceEntry, sysroot: impl AsRef<Path>) -> Result<Self, RefreshError> {
        let from = if v.url().starts_with("http://") || v.url().starts_with("https://") {
            OmaSourceEntryFrom::Http
        } else if v.url().starts_with("file:") {
            OmaSourceEntryFrom::Local
        } else {
            return Err(RefreshError::UnsupportedProtocol(format!("{v:?}")));
        };

        let components = v.components.clone();
        let arch = &dpkg_arch(sysroot)?;
        let url = v.url.replace("$(ARCH)", arch);
        let suite = v.suite.replace("$(ARCH)", arch);
        let is_source = v.source;

        let (dist_path, is_flat) = if components.is_empty() {
            // flat repo suite 后面一定有斜线
            if suite == "/" {
                if !url.ends_with('/') {
                    (format!("{}{}", url, suite), true)
                } else {
                    (url.to_string(), true)
                }
            } else if url.ends_with('/') {
                (format!("{}{}", url, suite), true)
            } else {
                (format!("{}/{}", url, suite), true)
            }
        } else {
            (v.dist_path(), false)
        };

        let mut signed_by = None;
        let mut archs = vec![];

        let mut trusted = false;

        for i in &v.options {
            if let Some(v) = i.strip_prefix("arch=") {
                for i in v.split(',') {
                    archs.push(i.to_string());
                }
            }

            if let Some(v) = i.strip_prefix("signed-by=") {
                signed_by = Some(v.to_string());
            }

            if let Some(v) = i.strip_prefix("trusted=") {
                trusted = v == "yes";
            }
        }

        Ok(Self {
            from,
            components,
            url,
            suite,
            is_flat,
            dist_path,
            signed_by,
            archs,
            trusted,
            native_arch: arch.to_string(),
            is_source,
        })
    }
}

#[test]
fn test_ose() {
    // Flat repository tests.

    // deb file:///debs/ /
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:///debs/".to_string(),
        suite: "/".to_string(),
        components: vec![],
        is_deb822: false,
    };

    let ose = OmaSourceEntry::new(&entry, "/").unwrap();
    assert_eq!(ose.url, "file:///debs/");
    assert_eq!(ose.dist_path, "file:///debs/");

    // deb file:///debs/ ./
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:///debs/".to_string(),
        suite: "./".to_string(),
        components: vec![],
        is_deb822: false,
    };

    let ose = OmaSourceEntry::new(&entry, "/").unwrap();
    assert_eq!(ose.url, "file:///debs/");
    assert_eq!(ose.dist_path, "file:///debs/./");

    // deb file:/debs/ /
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:/debs/".to_string(),
        suite: "/".to_string(),
        components: vec![],
        is_deb822: false,
    };

    let ose = OmaSourceEntry::new(&entry, "/").unwrap();
    assert_eq!(ose.url, "file:/debs/");
    assert_eq!(ose.dist_path, "file:/debs/");

    // deb file:/debs /
    //
    // APT will append implicitly a / at the end of the URL.
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:/debs".to_string(),
        suite: "/".to_string(),
        components: vec![],
        is_deb822: false,
    };

    let ose = OmaSourceEntry::new(&entry, "/").unwrap();
    assert_eq!(ose.url, "file:/debs");
    assert_eq!(ose.dist_path, "file:/debs/");

    // deb file:/debs/ ./././
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:/debs/".to_string(),
        suite: "./././".to_string(),
        components: vec![],
        is_deb822: false,
    };

    let ose = OmaSourceEntry::new(&entry, "/").unwrap();
    assert_eq!(ose.url, "file:/debs/");
    assert_eq!(ose.dist_path, "file:/debs/./././");

    // deb file:/debs/ .//
    //
    // APT will throw a warning but carry on with the suite name:
    //
    // W: Conflicting distribution: file:/debs .// Release (expected .// but got )
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:/debs/".to_string(),
        suite: ".//".to_string(),
        components: vec![],
        is_deb822: false,
    };

    let ose = OmaSourceEntry::new(&entry, "/").unwrap();
    assert_eq!(ose.url, "file:/debs/");
    assert_eq!(ose.dist_path, "file:/debs/.//");

    // deb file:/debs/ //
    //
    // APT will throw a warning but carry on with the suite name:
    //
    // W: Conflicting distribution: file:/debs // Release (expected // but got )
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:/debs/".to_string(),
        suite: "//".to_string(),
        components: vec![],
        is_deb822: false,
    };

    let ose = OmaSourceEntry::new(&entry, "/").unwrap();
    assert_eq!(ose.url, "file:/debs/");
    assert_eq!(ose.dist_path, "file:/debs///");

    // deb file:/./debs/ ./
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:/./debs/".to_string(),
        suite: "./".to_string(),
        components: vec![],
        is_deb822: false,
    };

    let ose = OmaSourceEntry::new(&entry, "/").unwrap();
    assert_eq!(ose.url, "file:/./debs/");
    assert_eq!(ose.dist_path, "file:/./debs/./");

    // deb file:/usr/../debs/ ./
    let entry = SourceEntry {
        enabled: true,
        source: false,
        options: vec![],
        url: "file:/usr/../debs/".to_string(),
        suite: "./".to_string(),
        components: vec![],
        is_deb822: false,
    };

    let ose = OmaSourceEntry::new(&entry, "/").unwrap();
    assert_eq!(ose.url, "file:/usr/../debs/");
    assert_eq!(ose.dist_path, "file:/usr/../debs/./");
}

#[test]
fn test_database_filename() {
    let replacer = DatabaseFilenameReplacer::new().unwrap();

    // Encode + as %252b.
    let s = "https://repo.aosc.io/debs/dists/x264-0+git20240305/InRelease";
    let res = replacer.replace(s).unwrap();
    assert_eq!(
        res,
        "repo.aosc.io_debs_dists_x264-0%252bgit20240305_InRelease"
    );

    // Encode : as %3A.
    let s = "https://ci.deepin.com/repo/obs/deepin%3A/CI%3A/TestingIntegration%3A/test-integration-pr-1537/testing/./Packages";
    let res = replacer.replace(s).unwrap();
    assert_eq!(res, "ci.deepin.com_repo_obs_deepin:_CI:_TestingIntegration:_test-integration-pr-1537_testing_._Packages");

    // Encode _ as %5f
    let s = "https://repo.aosc.io/debs/dists/xorg-server-21.1.13-hyperv_drm-fix";
    let res = replacer.replace(s).unwrap();
    assert_eq!(
        res,
        "repo.aosc.io_debs_dists_xorg-server-21.1.13-hyperv%5fdrm-fix"
    );

    // file:/// should be transliterated as file:/.
    let s1 = "file:/debs";
    let s2 = "file:///debs";
    let res1 = replacer.replace(s1).unwrap();
    let res2 = replacer.replace(s2).unwrap();
    assert_eq!(res1, "_debs");
    assert_eq!(res1, res2);

    // Dots (.) in flat repo URLs should be preserved in resolved database name.
    let s = "file:///././debs/./Packages";
    let res = replacer.replace(s).unwrap();
    assert_eq!(res, "_._._debs_._Packages");

    // Slash (/) in flat repo "suite" names should be transliterated as _.
    let s = "file:///debs/Packages";
    let res = replacer.replace(s).unwrap();
    assert_eq!(res, "_debs_Packages");

    // Dots (.) in flat repo "suite" names should be preserved in resolved database name.
    let s = "file:///debs/./Packages";
    let res = replacer.replace(s).unwrap();
    assert_eq!(res, "_debs_._Packages");

    // Slashes in URL and in flat repo "suite" names should be preserved in original number (1).
    let s = "file:///debs///./Packages";
    let res = replacer.replace(s).unwrap();
    assert_eq!(res, "_debs___._Packages");

    // Slashes in URL and in flat repo "suite" names should be preserved in original number (2).
    let s = "file:///debs///.///Packages";
    let res = replacer.replace(s).unwrap();
    assert_eq!(res, "_debs___.___Packages");
}
