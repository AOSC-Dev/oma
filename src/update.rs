use std::{
    collections::{HashMap, HashSet},
    io::Read,
    path::Path,
};

use anyhow::{anyhow, Ok, Result};
use apt_sources_lists::*;
use eight_deep_parser::Item;
use log::info;
use reqwest::blocking::Client;

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
            } else {
                continue;
            }
        }

        let in_release = InReleaseParser::new(&p)?;

        let checksums = in_release
            .checksums
            .iter()
            .filter(|x| components[index].contains(&x.name.split('/').nth(0).unwrap().to_string()));

        for i in checksums {
            if i.file_type == DistFileType::CompressContents
                || i.file_type == DistFileType::CompressPackageList
            {
                let (name, buf) = download_db(&format!("{}/{}", dist_urls[index], i.name), &client)?;
                // checksum()
            }
        }
    }

    Ok(())
}

fn checksum(buf: &[u8], hash: &str) -> Result<()> {
    todo!()
}

// fn update_package_list(
//     p: &Path,
//     file: &[u8],
//     source: &SourceEntry,
//     client: &Client,
// ) -> Result<Vec<PathBuf>> {
//     std::fs::write(p, file)?;
//     let list = package_list_single(source)?;
//     let files = downloads_and_extract(client, &list, true)?;
//     let paths = package_list_file_names(&list, true)?;
//     let mut result = vec![];

//     for (i, c) in paths.iter().enumerate() {
//         std::fs::write(&c, &files[i])?;
//         result.push(c.to_path_buf());
//     }

//     Ok(result)
// }

// fn downloads_and_extract(client: &Client, urls: &[String], is_xz: bool) -> Result<Vec<Vec<u8>>> {
//     let mut files = vec![];

//     for i in urls {
//         info!("Downloading {} ...", i);
//         let file = client.get(i).send()?.error_for_status()?.bytes()?.to_vec();
//         files.push(file);
//     }

//     let mut result = vec![];

//     if is_xz {
//         for i in files {
//             let mut decompress = xz2::read::XzDecoder::new(&*i);

//             let mut buf = vec![];
//             decompress.read_to_end(&mut buf)?;

//             result.push(buf);
//         }

//         return Ok(result);
//     } else {
//         return Ok(files);
//     }
// }

// fn url_to_file_names(urls: &[String]) -> Result<Vec<String>> {
//     let mut file_names = vec![];

//     for i in urls {
//         let url = reqwest::Url::parse(&i)?;
//         let scheme = url.scheme();
//         let url = i
//             .strip_prefix(&format!("{}://", scheme))
//             .ok_or_else(|| anyhow!("Can not get url without url scheme"))?
//             .replace("/", "_");

//         file_names.push(url);
//     }

//     Ok(file_names)
// }

// fn need_update_list(list: &[PathBuf]) -> Result<Vec<(String, String, String)>> {
//     let mut result = vec![];
//     let mirror_package_list = mirror_package_list(&list)?;
//     let dpkg_status = dpkg_package_list()?;

//     let mirror_package_list = remove_useless_in_apt_list(&mirror_package_list)?;
//     let dpkg_status = to_package_version_map(&dpkg_status);

//     for (package, local_version) in dpkg_status {
//         if let Some(mirror_version) = mirror_package_list.get(&package) {
//             if PkgVersion::try_from(local_version.as_str())?
//                 < PkgVersion::try_from(mirror_version.as_str())?
//             {
//                 result.push((package, local_version, mirror_version.to_owned()));
//             }
//         }
//     }

//     Ok(result)
// }

// fn mirror_package_list(file_list: &[PathBuf]) -> Result<Vec<HashMap<String, Item>>> {
//     let mut result = vec![];

//     for i in file_list {
//         let mut buf = vec![];

//         let mut f = std::fs::File::open(i)?;
//         f.read_to_end(&mut buf)?;
//         let map = eight_deep_parser::parse_multi(std::str::from_utf8(&buf)?)?;

//         map.into_iter().for_each(|x| result.push(x));
//     }

//     Ok(result)
// }

// fn dpkg_package_list() -> Result<Vec<HashMap<String, Item>>> {
//     let mut buf = vec![];
//     let mut f = std::fs::File::open(DPKG_STATUS)?;
//     f.read_to_end(&mut buf)?;

//     let map = eight_deep_parser::parse_multi(std::str::from_utf8(&buf)?)?;

//     Ok(map)
// }

// fn to_package_version_map(items: &[HashMap<String, Item>]) -> HashMap<String, String> {
//     let mut map = HashMap::new();
//     for i in items {
//         if let Some(Item::OneLine(p)) = i.get("Package") {
//             if let Some(Item::OneLine(v)) = i.get("Version") {
//                 map.insert(p.to_owned(), v.to_owned());
//             }
//         }
//     }

//     map
// }

// fn remove_useless_in_apt_list(items: &[HashMap<String, Item>]) -> Result<HashMap<String, String>> {
//     let mut result = HashMap::new();
//     let mut pushed_package = Vec::new();

//     for (i, c) in items.iter().enumerate() {
//         let name = if let Some(Item::OneLine(name)) = c.get("Package") {
//             name
//         } else {
//             panic!("");
//         };

//         if pushed_package.contains(name) {
//             continue;
//         }

//         let mut eq = Vec::new();
//         for (h, j) in items.iter().enumerate() {
//             if h < i {
//                 continue;
//             }

//             if let Some(Item::OneLine(x)) = j.get("Package") {
//                 if x == name {
//                     eq.push(j);
//                 }
//             }
//         }

//         let mut parse_vec = Vec::new();

//         for i in eq {
//             if let Some(Item::OneLine(n)) = i.get("Package") {
//                 if let Some(Item::OneLine(v)) = i.get("Version") {
//                     parse_vec.push((n, PkgVersion::try_from(v.as_str()).unwrap(), v))
//                 }
//             }
//         }

//         parse_vec.sort_by(|x, y| x.1.cmp(&y.1));

//         let (name, _, v) = parse_vec.last().unwrap();
//         result.insert(name.to_string(), v.to_string());
//         pushed_package.push(name.to_string());
//     }

//     Ok(result)
// }

// #[test]
// fn test() {
//     let sources = get_sources().unwrap();
//     let a = sources
//         .first()
//         .unwrap()
//         .dist_components()
//         .collect::<Vec<_>>();
//     dbg!(a);
//     dbg!(package_list(&sources));
// }
