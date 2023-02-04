use core::panic;
use std::{
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use apt_sources_lists::*;
use flate2::bufread::GzDecoder;
use indexmap::IndexMap;
use log::info;
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use std::io::Write;
use xz2::read::XzDecoder;

use crate::{
    download::{download, download_package},
    pkgversion::PkgVersion,
    utils::get_arch_name,
    verify,
};

pub const APT_LIST_DISTS: &str = "/var/lib/apt/lists";
pub const DOWNLOAD_DIR: &str = "/var/cache/apt/archives";

struct FileBuf(Vec<u8>);

#[derive(Debug)]
struct FileName(String);

impl FileName {
    fn new(s: &str) -> Result<Self> {
        let url = reqwest::Url::parse(s)?;
        let scheme = url.scheme();
        let url = s
            .strip_prefix(&format!("{scheme}://"))
            .ok_or_else(|| anyhow!("Can not get url without url scheme"))?
            .replace('/', "_");

        Ok(FileName(url))
    }
}

fn download_db(url: &str, client: &Client, filename: &str) -> Result<(FileName, FileBuf)> {
    info!("Downloading {}", url);

    let v = download(url, client, filename, Path::new(APT_LIST_DISTS))?;

    Ok((FileName::new(url)?, FileBuf(v)))
}

#[derive(Debug)]
struct InReleaseParser {
    _source: Vec<IndexMap<String, String>>,
    checksums: Vec<ChecksumItem>,
}

#[derive(Debug)]
struct ChecksumItem {
    name: String,
    _size: u64,
    checksum: String,
    file_type: DistFileType,
}

#[derive(Debug, PartialEq)]
enum DistFileType {
    BinaryContents,
    Contents,
    CompressContents,
    PackageList,
    CompressPackageList,
}

impl InReleaseParser {
    fn new(p: &Path) -> Result<Self> {
        let mut f = std::fs::File::open(p)?;
        let mut s = String::new();

        f.read_to_string(&mut s)?;

        let s = if s.starts_with("-----BEGIN PGP SIGNED MESSAGE-----") {
            verify::verify(&s)?
        } else {
            s
        };

        let source = debcontrol_from_str(&s)?;
        let sha256 = source
            .first()
            .and_then(|x| x.get("SHA256"))
            .take()
            .context("source is empty")?;

        let mut checksums = sha256.split('\n');

        // remove first item, It's empty
        checksums.next();

        let mut checksums_res = vec![];

        for i in checksums {
            let checksum = i.split_whitespace().collect::<Vec<_>>();
            let checksum = (checksum[2], checksum[1], checksum[0]);
            checksums_res.push(checksum);
        }

        let arch = get_arch_name().ok_or_else(|| anyhow!("Can not get arch!"))?;

        let mut res = vec![];

        let c = checksums_res
            .into_iter()
            .filter(|(name, _, _)| name.contains("all") || name.contains(arch));

        for i in c {
            let t = if i.0.contains("BinContents") {
                DistFileType::BinaryContents
            } else if i.0.contains("/Contents-") && i.0.contains('.') {
                DistFileType::CompressContents
            } else if i.0.contains("/Contents-") && !i.0.contains('.') {
                DistFileType::Contents
            } else if i.0.contains("Packages") && !i.0.contains('.') {
                DistFileType::PackageList
            } else if i.0.contains("Packages") && i.0.contains('.') {
                DistFileType::CompressPackageList
            } else {
                panic!("I Dont known why ...")
            };

            res.push(ChecksumItem {
                name: i.0.to_owned(),
                _size: i.1.parse::<u64>()?,
                checksum: i.2.to_owned(),
                file_type: t,
            })
        }

        Ok(Self {
            _source: source,
            checksums: res,
        })
    }
}

pub fn debcontrol_from_file(p: &Path) -> Result<Vec<IndexMap<String, String>>> {
    let mut f = std::fs::File::open(p)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;

    debcontrol_from_str(&s)
}

fn debcontrol_from_str(s: &str) -> Result<Vec<IndexMap<String, String>>> {
    let mut res = vec![];

    let debcontrol = debcontrol::parse_str(s).map_err(|e| anyhow!("{}", e))?;

    for i in debcontrol {
        let mut item = IndexMap::new();
        let field = i.fields;

        for j in field {
            item.insert(j.name.to_string(), j.value.to_string());
        }

        res.push(item);
    }

    Ok(res)
}

pub fn get_sources_dists_filename(sources: &[SourceEntry], client: &Client) -> Result<Vec<String>> {
    let dist_urls = sources.iter().map(|x| x.dist_path()).collect::<Vec<_>>();
    let dists_in_releases = dist_urls.iter().map(|x| {
        (
            x.to_owned(),
            FileName::new(&format!("{}/{}", x, "InRelease")),
        )
    });

    let mut res = vec![];

    let components = sources
        .iter()
        .map(|x| x.components.to_owned())
        .collect::<Vec<_>>();

    for (i, c) in dists_in_releases.enumerate() {
        let filename = c.1?.0;
        let in_release = InReleaseParser::new(&Path::new(APT_LIST_DISTS).join(&filename));

        let in_release = if in_release.is_err() {
            update_db(sources, client)?;
            let in_release = InReleaseParser::new(&Path::new(APT_LIST_DISTS).join(filename))?;

            in_release
        } else {
            in_release?
        };

        let checksums = in_release
            .checksums
            .iter()
            .filter(|x| components[i].contains(&x.name.split('/').next().unwrap().to_string()))
            .collect::<Vec<_>>();

        for j in checksums {
            if j.name.ends_with("Packages") {
                res.push(FileName::new(&format!("{}/{}", c.0, j.name))?);
            }
        }
    }

    let res = res.into_iter().map(|x| x.0).collect();

    Ok(res)
}

/// Get /etc/apt/sources.list and /etc/apt/sources.list.d
pub fn get_sources() -> Result<Vec<SourceEntry>> {
    let mut res = Vec::new();
    let list = SourcesLists::scan()?;

    for file in list.iter() {
        for i in &file.lines {
            if let SourceLine::Entry(entry) = i {
                res.push(entry.to_owned());
            }
        }
    }

    Ok(res)
}

/// Download packages
pub fn packages_download(
    list: &[String],
    apt: &[IndexMap<String, String>],
    sources: &[SourceEntry],
    client: &Client,
) -> Result<()> {
    for i in list {
        let v = apt.iter().find(|x| x.get("Package") == Some(i));

        let Some(v) = v else { bail!("Can not get package {} from list", i) };
        let file_name = v["Filename"].clone();
        let checksum = v["SHA256"].clone();
        let version = v["Version"].clone();
        let mut file_name_split = file_name.split('/');

        let branch = file_name_split
            .nth(1)
            .take()
            .context(format!("Can not parse package {i} Filename field!"))?;

        let component = file_name_split
            .next()
            .take()
            .context(format!("Can not parse package {i} Filename field!"))?;

        let mirror = sources
            .iter()
            .filter(|x| x.components.contains(&component.to_string()))
            .filter(|x| x.suite == branch)
            .map(|x| x.url());

        let available_download = mirror.map(|x| format!("{x}/{file_name}"));

        let mut deb_filename = vec![];

        for i in available_download {
            if let Ok(filename) = download_package(&i, None, client, &checksum, &version) {
                deb_filename.push(filename);
                break;
            }
        }
    }

    Ok(())
}

pub fn package_list(db_file_paths: Vec<PathBuf>) -> Result<Vec<IndexMap<String, String>>> {
    let mut apt = vec![];
    for i in db_file_paths {
        let parse = debcontrol_from_file(&i)?;
        apt.extend(parse);
    }

    apt.sort_by(|x, y| x["Package"].cmp(&y["Package"]));

    Ok(apt)
}

pub fn update_db(sources: &[SourceEntry], client: &Client) -> Result<()> {
    let dist_urls = sources.iter().map(|x| x.dist_path()).collect::<Vec<_>>();
    let dist_files = update_in_release(&dist_urls, client);

    let components = sources
        .iter()
        .map(|x| x.components.to_owned())
        .collect::<Vec<_>>();

    for (index, (name, file)) in dist_files.into_iter().enumerate() {
        let p = Path::new(APT_LIST_DISTS).join(name.0);

        if !p.exists() || !p.is_file() {
            std::fs::create_dir_all(APT_LIST_DISTS)?;
            std::fs::write(&p, &file.0)?;
        } else {
            let mut f = std::fs::File::open(&p)?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)?;

            if buf != file.0 {
                std::fs::write(&p, &file.0)?;
            }
        }

        let in_release = InReleaseParser::new(&p)?;

        let checksums = in_release
            .checksums
            .iter()
            .filter(|x| components[index].contains(&x.name.split('/').next().unwrap().to_string()))
            .collect::<Vec<_>>();

        for i in &checksums {
            if i.file_type == DistFileType::CompressContents
                || i.file_type == DistFileType::CompressPackageList
            {
                let not_compress_file = i.name.replace(".xz", "").replace(".gz", "");
                let file_name =
                    FileName::new(&format!("{}/{}", dist_urls[index], not_compress_file))?;

                let p = Path::new(APT_LIST_DISTS).join(&file_name.0);

                if p.exists() {
                    let mut f = std::fs::File::open(&p)?;
                    let mut buf = Vec::new();
                    f.read_to_end(&mut buf)?;

                    let checksums_index = checksums
                        .iter()
                        .position(|x| x.name == not_compress_file)
                        .unwrap();

                    let hash = checksums[checksums_index].checksum.to_owned();

                    if checksum(&buf, &hash).is_err() {
                        download_and_extract(&dist_urls[index], i, client, &file_name.0)?;
                    } else {
                        continue;
                    }
                } else {
                    download_and_extract(&dist_urls[index], i, client, &file_name.0)?;
                }
            }
        }
    }

    Ok(())
}

fn update_in_release(dist_urls: &Vec<String>, client: &Client) -> Vec<(FileName, FileBuf)> {
    let dists_in_releases = dist_urls.iter().map(|x| {
        (
            format!("{}/{}", x, "InRelease"),
            FileName::new(&format!("{}/{}", x, "InRelease")),
        )
    });

    let dist_files = dists_in_releases
        .flat_map(|x| download_db(&x.0, client, &x.1.unwrap().0))
        .collect::<Vec<_>>();

    dist_files
}

/// Download and extract package list database
fn download_and_extract(
    dist_url: &str,
    i: &ChecksumItem,
    client: &Client,
    not_compress_file: &str,
) -> Result<()> {
    let (name, buf) = download_db(&format!("{}/{}", dist_url, i.name), client, &i.name)?;
    checksum(&buf.0, &i.checksum)?;

    let buf = decompress(&buf.0, &name.0)?;
    let p = Path::new(APT_LIST_DISTS).join(not_compress_file);
    std::fs::write(p, buf)?;

    Ok(())
}

/// Check download is success
fn checksum(buf: &[u8], hash: &str) -> Result<()> {
    let mut hasher = Sha256::new();
    hasher.write_all(buf)?;
    let buf_hash = hasher.finalize();
    let buf_hash = format!("{buf_hash:2x}");

    if hash != buf_hash {
        return Err(anyhow!(
            "Checksum mismatch. Expected {}, got {}",
            hash,
            buf_hash
        ));
    }

    Ok(())
}

/// Extract database
fn decompress(buf: &[u8], name: &str) -> Result<Vec<u8>> {
    let buf = if name.ends_with(".gz") {
        let mut decompressor = GzDecoder::new(buf);
        let mut buf = Vec::new();
        decompressor.read_to_end(&mut buf)?;

        buf
    } else if name.ends_with(".xz") {
        let mut decompressor = XzDecoder::new(buf);
        let mut buf = Vec::new();
        decompressor.read_to_end(&mut buf)?;

        buf
    } else {
        return Err(anyhow!("Unsupported compression format."));
    };

    Ok(buf)
}

#[derive(Debug)]
pub struct UpdatePackage {
    pub package: String,
    pub old_version: String,
    pub new_version: String,
    pub file_name: String,
    pub from: String,
    pub dpkg_installed_size: u64,
    pub apt_size: u64,
    pub apt_installed_size: u64,
}

/// Filter package newest version list
pub fn newest_package_list(
    input: &[IndexMap<String, String>],
) -> Result<Vec<IndexMap<String, String>>> {
    let mut input = input.to_vec();

    input.sort_by(|x, y| x["Package"].cmp(&y["Package"]));

    let apt = version_sort(&input)?;

    Ok(apt)
}

/// Sort package list with version
fn version_sort(map: &[IndexMap<String, String>]) -> Result<Vec<IndexMap<String, String>>> {
    let Some(last_name) = map.first().and_then(|x| x.get("Package")) else { bail!("package list is empty") };
    let mut last_name = last_name.to_owned();
    let mut list = vec![];
    let mut tmp = vec![];

    for i in map {
        let package = i["Package"].clone();
        let version = i["Version"].clone();

        if package == last_name {
            tmp.push((PkgVersion::try_from(version.as_str())?, i.to_owned()));
        } else {
            tmp.sort_by(|x, y| x.0.cmp(&y.0));
            list.push(tmp.last().unwrap().to_owned());
            tmp.clear();
            last_name = package.to_string();
            tmp.push((PkgVersion::try_from(version.as_str())?, i.to_owned()));
        }
    }

    Ok(list.into_iter().map(|x| x.1).collect())
}
