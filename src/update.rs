use std::{
    collections::{HashMap, HashSet},
    io::Read,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{anyhow, Ok, Result};
use apt_sources_lists::*;
use eight_deep_parser::Item;
use log::info;
use reqwest::blocking::Client;
use sequoia_openpgp::{
    parse::{stream::VerifierBuilder, Parse},
    policy::StandardPolicy,
    Cert,
};

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

fn download(url: &str, client: &Client) -> Result<(FileName, FileBuf)> {
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
    checksums: HashMap<String, (u64, String)>,
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

        let mut map = HashMap::new();
        let arch = get_arch_name().ok_or_else(|| anyhow!("Can not get arch!"))?;
        for (name, size, checksum) in checksums_res {
            if name.contains("all") || name.contains(arch) {
                map.insert(name.to_owned(), (size.parse::<u64>()?, checksum.to_owned()));
            }
        }

        Ok(Self {
            source,
            checksums: map,
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

pub fn update() -> Result<()> {
    let sources = get_sources()?;

    let dists_in_releases = sources
        .iter()
        .map(|x| format!("{}/{}", x.dist_path(), "InRelease"));

    let client = reqwest::blocking::ClientBuilder::new()
        .user_agent("aoscpt")
        .build()?;

    let dist_files = dists_in_releases.flat_map(|x| download(&x, &client));

    for (name, file) in dist_files {
        let p = Path::new(APT_LIST_DISTS).join(name.0);

        if !p.exists() || !p.is_dir() {
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
        dbg!(in_release);
    }

    // // like: mirrors.bfsu.edu.cn_anthon_debs_dists_stable_InRelease
    // let release_file_names = url_to_file_names(&dists)?;

    // let client = reqwest::blocking::ClientBuilder::new()
    //     .user_agent("aoscpt")
    //     .build()?;

    // let files = downloads_and_extract(&client, &dists, false)?;

    // let apt_dists = Path::new(APT_LIST_DISTS);

    // if !apt_dists.is_dir() {
    //     std::fs::create_dir_all(APT_LIST_DISTS)?;
    // }

    // let mut urls = vec![];

    // for i in &sources {
    //     let v = package_list_single(&i)?;
    //     for j in v {
    //         urls.push(j);
    //     }
    // }

    // // like: mirrors.bfsu.edu.cn_anthon_debs_dists_stable_main_binary-all_Packages
    // let package_list_paths = package_list_file_names(&urls, true)?;

    // for (i, c) in release_file_names.iter().enumerate() {
    //     let p = apt_dists.join(c);
    //     // If InRelease File not exist
    //     if !p.is_file() {
    //         info!("InRelease file doesn't not exist, Downloading Package list!");
    //         update_package_list(&p, &files[i], &sources[i], &client)?;
    //     }

    //     let mut exist_file = std::fs::File::open(&p)?;
    //     let mut buf = vec![];
    //     exist_file.read_to_end(&mut buf)?;

    //     // If InRelease File local and mirror mismatch
    //     if buf != files[i] {
    //         info!("InRelease file is olded, Downloading Package list!");
    //         update_package_list(&p, &files[i], &sources[i], &client)?;
    //     }
    // }

    // let list = need_update_list(&package_list_paths)?;

    // for i in &list {
    //     println!("{} {} {}", i.0, i.1, i.2);
    // }

    // dbg!(list.len());

    Ok(())
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
