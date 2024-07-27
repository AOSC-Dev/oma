use std::{borrow::Cow, path::Path};

use oma_apt_sources_lists::{SourceLine, SourceListType, SourcesLists};
use url::Url;

use crate::db::{OmaSourceEntry, RefreshError};

pub(crate) fn database_filename(url: &str) -> Result<String, RefreshError> {
    let url_parsed = Url::parse(url).map_err(|_| RefreshError::InvaildUrl(url.to_string()))?;

    let host = url_parsed.host_str();

    let path = if let Some(host) = host {
        // 不能使用 url_parsed.path()
        // 原因是 "/./" 会被解析器解析为 "/"，而 apt 则不会这样
        url.split_once(host)
            .ok_or_else(|| RefreshError::InvaildUrl(url.to_string()))?
            .1
    } else {
        // file 协议下不会有 host
        url_parsed.path()
    };

    let url = if let Some(host) = host {
        Cow::Owned(format!("{}{}", host, path))
    } else {
        Cow::Borrowed(path)
    };

    let url = url
        .replace('/', "_")
        .replace('+', "%252b")
        .replace("%3a", ":")
        .replace("%3A", ":");

    Ok(url)
}

pub(crate) fn human_download_url(
    ose: &OmaSourceEntry,
    file_name: Option<&str>,
) -> Result<String, RefreshError> {
    let url = Url::parse(&ose.url).map_err(|_| RefreshError::InvaildUrl(ose.url.to_string()))?;

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

#[test]
fn test_database_filename() {
    // Mirror name contains '+' must be url encode twice
    let s = "https://repo.aosc.io/debs/dists/x264-0+git20240305/InRelease";
    let res = database_filename(s).unwrap();

    assert_eq!(
        res,
        "repo.aosc.io_debs_dists_x264-0%252bgit20240305_InRelease"
    );

    let s = "https://ci.deepin.com/repo/obs/deepin%3A/CI%3A/TestingIntegration%3A/test-integration-pr-1537/testing/./Packages";
    let res = database_filename(s).unwrap();

    assert_eq!(res, "ci.deepin.com_repo_obs_deepin:_CI:_TestingIntegration:_test-integration-pr-1537_testing_._Packages");

    let s1 = "file:/debs";
    let s2 = "file:///debs";
    let res1 = database_filename(s1).unwrap();
    let res2 = database_filename(s2).unwrap();
    assert_eq!(res1, "_debs");
    assert_eq!(res1, res2);
}
