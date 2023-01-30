use core::panic;
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Ok, Result};
use apt_sources_lists::*;
use eight_deep_parser::Item;
use flate2::bufread::GzDecoder;
use log::info;
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use std::io::Write;
use xz2::read::XzDecoder;

use crate::{pkgversion::PkgVersion, utils::get_arch_name, verify};

const APT_LIST_DISTS: &str = "/var/lib/apt/lists";
const DPKG_STATUS: &str = "/var/lib/dpkg/status";

struct FileBuf(Vec<u8>);
struct FileName(String);

impl FileName {
    fn new(s: &str) -> Result<Self> {
        let url = reqwest::Url::parse(&s)?;
        let scheme = url.scheme();
        let url = s
            .strip_prefix(&format!("{}://", scheme))
            .ok_or_else(|| anyhow!("Can not get url without url scheme"))?
            .replace("/", "_");

        Ok(FileName(url))
    }
}

fn download_db(url: &str, client: &Client) -> Result<(FileName, FileBuf)> {
    info!("Downloading {}", url);

    let v = client
        .get(url)
        .send()?
        .error_for_status()?
        .bytes()?
        .to_vec();

    Ok((FileName::new(url)?, FileBuf(v)))
}

#[derive(Debug)]
struct InReleaseParser {
    source: HashMap<String, Item>,
    checksums: Vec<ChecksumItem>,
}

#[derive(Debug)]
struct ChecksumItem {
    name: String,
    size: u64,
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

        let source = eight_deep_parser::parse_one(&s)?;
        let dists = source
            .get("SHA256")
            .ok_or_else(|| anyhow!("Can not get sha256 item from InRelease: {}", p.display()))?;

        let checksums = if let Item::MultiLine(v) = dists {
            v.to_owned()
        } else {
            return Err(anyhow!("Can not get dists checksums!"));
        };

        let mut checksums_res = vec![];

        for i in &checksums {
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
            } else if i.0.contains("/Contents-") && i.0.contains(".") {
                DistFileType::CompressContents
            } else if i.0.contains("/Contents-") && !i.0.contains(".") {
                DistFileType::Contents
            } else if i.0.contains("Packages") && !i.0.contains(".") {
                DistFileType::PackageList
            } else if i.0.contains("Packages") && i.0.contains(".") {
                DistFileType::CompressPackageList
            } else {
                panic!("I Dont known why ...")
            };

            res.push(ChecksumItem {
                name: i.0.to_owned(),
                size: i.1.parse::<u64>()?,
                checksum: i.2.to_owned(),
                file_type: t,
            })
        }

        Ok(Self {
            source,
            checksums: res,
        })
    }
}

fn get_sources() -> Result<Vec<SourceEntry>> {
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

pub fn update(client: &Client) -> Result<()> {
    let sources = get_sources()?;

    let dist_urls = sources.iter().map(|x| x.dist_path()).collect::<Vec<_>>();
    let dists_in_releases = dist_urls.iter().map(|x| format!("{}/{}", x, "InRelease"));

    let components = sources
        .iter()
        .map(|x| x.components.to_owned())
        .collect::<Vec<_>>();

    let dist_files = dists_in_releases.flat_map(|x| download_db(&x, &client));

    let mut db_file_paths = vec![];

    for (index, (name, file)) in dist_files.enumerate() {
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
            .filter(|x| components[index].contains(&x.name.split('/').nth(0).unwrap().to_string()))
            .collect::<Vec<_>>();

        for i in &checksums {
            if i.file_type == DistFileType::CompressContents
                || i.file_type == DistFileType::CompressPackageList
            {
                let not_compress_file = i.name.replace(".xz", "").replace(".gz", "");
                let file_name =
                    FileName::new(&format!("{}/{}", dist_urls[index], not_compress_file))?;

                let p = Path::new(APT_LIST_DISTS).join(&file_name.0);

                if i.file_type == DistFileType::CompressPackageList {
                    db_file_paths.push(p.to_path_buf());
                }

                if p.exists() {
                    let mut f = std::fs::File::open(&p)?;
                    let mut buf = Vec::new();
                    f.read_to_end(&mut buf)?;

                    let index = checksums
                        .iter()
                        .position(|x| x.name == not_compress_file)
                        .unwrap();

                    let hash = checksums[index].checksum.to_owned();

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

    let u = find_upgrade(&db_file_paths)?;
    dbg!(u);

    Ok(())
}

fn download_and_extract(dist_url: &str, i: &ChecksumItem, client: &Client, not_compress_file: &str) -> Result<()> {
    let (name, buf) =
        download_db(&format!("{}/{}", dist_url, i.name), &client)?;
    checksum(&buf.0, &i.checksum)?;

    let buf = decompress(&buf.0, &name.0)?;
    let p = Path::new(APT_LIST_DISTS).join(not_compress_file);
    std::fs::write(&p, buf)?;

    Ok(())
}

fn checksum(buf: &[u8], hash: &str) -> Result<()> {
    let mut hasher = Sha256::new();
    hasher.write_all(buf)?;
    let buf_hash = hasher.finalize();
    let buf_hash = format!("{:2x}", buf_hash);

    if hash != buf_hash {
        return Err(anyhow!(
            "Checksum mismatch. Expected {}, got {}",
            hash,
            buf_hash
        ));
    }

    Ok(())
}

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
struct UpdatePackage {
    package: String,
    old_version: String,
    new_version: String,
}

fn find_upgrade(db_paths: &[PathBuf]) -> Result<Vec<UpdatePackage>> {
    let mut res = Vec::new();
    let mut apt = vec![];
    for i in db_paths {
        let p = package_list(&i)?;
        apt.extend(p);
    }

    let mut dpkg = String::new();
    let mut f = std::fs::File::open(DPKG_STATUS)?;
    f.read_to_string(&mut dpkg)?;
    let dpkg = eight_deep_parser::parse_multi(&dpkg)?;

    for i in dpkg {
        let Item::OneLine(ref package) = i["Package"] else { panic!("8d") };
        let Item::OneLine(ref dpkg_version) = i["Version"] else { panic!("8d") };
        let index = apt.iter().position(|p| p["Package"] == Item::OneLine(package.to_string()));

        if let Some(index) = index {
            let Item::OneLine(ref apt_version) = apt[index]["Version"] else { panic!("8d") };
            if PkgVersion::try_from(apt_version.as_str())? > PkgVersion::try_from(dpkg_version.as_str())? {
                res.push(UpdatePackage {
                    package: package.to_string(),
                    new_version: apt_version.to_string(),
                    old_version: dpkg_version.to_string(),
                });
            }
        }
    }

    Ok(res)
}

fn package_list(list_path: &Path) -> Result<Vec<HashMap<String, Item>>> {
    info!("Handling package list at {}", list_path.display());
    let mut buf = String::new();
    let mut f = std::fs::File::open(list_path)?;
    f.read_to_string(&mut buf)?;

    let mut map = eight_deep_parser::parse_multi(&buf)?;

    map.sort_by(|x, y| {
        let Item::OneLine(ref a) = x["Package"] else { panic!("8d") };
        let Item::OneLine(ref b) = y["Package"] else { panic!("8d") };

        a.cmp(b)
    });

    let Item::OneLine(ref last_name) = map[0]["Package"] else { panic!("8d") };
    let mut last_name = last_name.to_owned();
    let mut list = vec![];
    let mut tmp = vec![];

    for i in map {
        let Item::OneLine(ref package) = i["Package"] else { panic!("8d") };
        let Item::OneLine(ref version) = i["Version"] else { panic!("8d") };

        if package.to_owned() == last_name {
            tmp.push((PkgVersion::try_from(version.as_str())?, i.to_owned()));
        } else {
            tmp.sort_by(|x, y| x.0.cmp(&y.0));
            list.push(tmp.last().unwrap().to_owned());
            tmp.clear();
            last_name = package.to_string();
            tmp.push((PkgVersion::try_from(version.as_str())?, i));
        }
    }

    let res = list.into_iter().map(|x| x.1).collect::<Vec<_>>();

    Ok(res)
}
