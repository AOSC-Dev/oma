use std::{collections::HashMap, path::Path};

use oma_apt_sources_lists::{SourceLine, SourceListType, SourcesLists};
use serde::Deserialize;
use url::Url;

use crate::db::{OmaSourceEntry, RefreshError};

pub(crate) fn database_filename(url: &str) -> String {
    url.split("://")
        .nth(1)
        .unwrap_or(url)
        .replace('/', "_")
        .replace('+', "%252b")
}

#[derive(Deserialize)]
pub struct MirrorMapItem {
    url: String,
}

pub(crate) fn human_download_url(
    url: &str,
    mirror_map: &Option<HashMap<String, MirrorMapItem>>,
) -> Result<String, RefreshError> {
    let url = Url::parse(url).map_err(|_| RefreshError::InvaildUrl(url.to_string()))?;

    let host = if url.scheme() == "file" {
        "Local Mirror"
    } else {
        url.host_str()
            .ok_or_else(|| RefreshError::InvaildUrl(url.to_string()))?
    };

    let schema = url.scheme();
    let branch = url
        .path()
        .split('/')
        .nth_back(1)
        .ok_or_else(|| RefreshError::InvaildUrl(url.to_string()))?;

    let url = format!("{schema}://{host}/");

    // MIRROR 文件为 AOSC 独有，为兼容其他 .deb 系统，这里不直接返回错误
    if let Some(mirror_map) = mirror_map {
        for (k, v) in mirror_map.iter() {
            let mirror_url =
                Url::parse(&v.url).map_err(|_| RefreshError::InvaildUrl(v.url.to_string()))?;
            let mirror_url_host = mirror_url
                .host_str()
                .ok_or_else(|| RefreshError::InvaildUrl(v.url.to_string()))?;

            let schema = mirror_url.scheme();
            let mirror_url = format!("{schema}://{mirror_url_host}/");

            if mirror_url == url {
                return Ok(format!("{k}:{branch}"));
            }
        }
    }

    Ok(format!("{host}:{branch}"))
}

/// Get /etc/apt/sources.list and /etc/apt/sources.list.d
pub(crate) fn get_sources<P: AsRef<Path>>(sysroot: P) -> Result<Vec<OmaSourceEntry>, RefreshError> {
    let mut res = Vec::new();
    let list = SourcesLists::scan_from_root(sysroot).map_err(RefreshError::ScanSourceError)?;

    for file in list.iter() {
        match file.entries {
            SourceListType::SourceLine(ref lines) => {
                for i in lines {
                    if let SourceLine::Entry(entry) = i {
                        res.push(OmaSourceEntry::try_from(entry)?);
                    }
                }
            }
            SourceListType::Deb822(ref e) => {
                for i in &e.entries {
                    res.push(OmaSourceEntry::try_from(i)?);
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
    let res = database_filename(s);

    assert_eq!(
        res,
        "repo.aosc.io_debs_dists_x264-0%252bgit20240305_InRelease"
    )
}
