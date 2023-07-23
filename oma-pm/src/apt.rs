use std::{
    collections::HashMap,
    io::{IsTerminal, Write},
    path::{Path, PathBuf},
};

use oma_fetch::{DownloadEntry, DownloadError, DownloadSource, DownloadSourceType, OmaFetcher};
use rust_apt::{
    cache::{Cache, Upgrade},
    config::Config as AptConfig,
    new_cache,
    package::{Package, Version},
    raw::progress::InstallProgress,
    records::RecordField,
    util::{get_apt_progress_string, terminal_height, terminal_width},
};

use crate::{
    operation::{InstallEntry, OmaOperation, OperationEntry, RemoveEntry, RemoveTag},
    pkginfo::PkgInfo,
    query::{OmaDatabase, OmaDatabaseError},
};

pub struct OmaApt {
    cache: Cache,
    config: AptConfig,
}

#[derive(Debug, thiserror::Error)]
pub enum OmaAptError {
    #[error(transparent)]
    RustApt(#[from] rust_apt::util::Exception),
    #[error(transparent)]
    OmaDatabaseError(#[from] OmaDatabaseError),
    #[error("Failed to mark reinstall pkg: {0}")]
    MarkReinstallError(String),
    #[error("Dep issue")]
    DependencyIssue,
    #[error("Package: {0} is essential.")]
    PkgIsEssential(String),
    #[error("Package: {0} is no candidate.")]
    PkgNoCandidate(String),
    #[error("Package: {0} has no SHA256 checksum.")]
    PkgNoChecksum(String),
    #[error("Ivaild file name: {0}")]
    InvalidFileName(String),
    #[error(transparent)]
    DownlaodError(#[from] DownloadError),
}

type OmaAptResult<T> = Result<T, OmaAptError>;

impl OmaApt {
    pub fn new() -> OmaAptResult<Self> {
        Ok(Self {
            cache: new_cache!()?,
            config: AptConfig::new(),
        })
    }

    pub fn upgrade(&self) -> OmaAptResult<()> {
        self.cache.upgrade(&Upgrade::FullUpgrade)?;

        Ok(())
    }

    pub fn install(&self, pkgs: Vec<PkgInfo>, reinstall: bool) -> OmaAptResult<()> {
        for pkg in pkgs {
            mark_install(&self.cache, pkg, reinstall)?;
        }

        Ok(())
    }

    pub fn remove(&self, pkgs: Vec<PkgInfo>, purge: bool, protect: bool) -> OmaAptResult<()> {
        for pkg in pkgs {
            let pkg = Package::new(&self.cache, pkg.raw_pkg.unique());
            if pkg.is_essential() {
                if protect {
                    return Err(OmaAptError::PkgIsEssential(pkg.name().to_string()));
                } else {
                    if std::io::stdout().is_terminal() {
                        todo!()
                    } else {
                        return Err(OmaAptError::PkgIsEssential(pkg.name().to_string()));
                    }
                }
            }
            pkg.mark_delete(purge);
        }

        Ok(())
    }

    pub fn commit(self, network_thread: Option<usize>) -> OmaAptResult<()> {
        let map = self.operation_map()?;

        let download_pkg_list = map
            .into_iter()
            .filter(|(k, _)| k != &OmaOperation::Remove)
            .map(|(_, v)| v)
            .flatten()
            .collect::<Vec<_>>();

        let mut download_list = vec![];

        let mut total_size = 0;

        for entry in download_pkg_list {
            let OperationEntry::Install(v) = entry else { unreachable!() };
            let uris = v.pkg_urls();
            let sources = uris
                .into_iter()
                .map(|x| {
                    let source_type = if x.starts_with("file:") {
                        DownloadSourceType::Local
                    } else {
                        DownloadSourceType::Http
                    };

                    DownloadSource::new(x.to_string(), source_type)
                })
                .collect::<Vec<_>>();

            let filename = uris
                .first()
                .and_then(|x| x.split('/').last())
                .take()
                .ok_or_else(|| OmaAptError::InvalidFileName(v.name().to_string()))?;

            let download_entry = DownloadEntry::new(
                sources,
                apt_style_filename(filename, v.new_version().to_string())?,
                self.get_archive_dir(),
                Some(v.checksum().to_owned()),
                true,
                Some(format!("{} {} ({})", v.name(), v.new_version(), v.arch())),
            );

            total_size += v.download_size();

            download_list.push(download_entry);
        }

        let downloader =
            OmaFetcher::new(None, true, Some(total_size), download_list, network_thread)?;

        todo!()
    }

    fn get_archive_dir(&self) -> PathBuf {
        let archives_dir = self.config.get("Dir::Cache::Archives");

        let path = if let Some(archives_dir) = archives_dir {
            if !Path::new(&archives_dir).is_absolute() {
                PathBuf::from(format!("/var/cache/apt/{archives_dir}"))
            } else {
                PathBuf::from(archives_dir)
            }
        } else {
            PathBuf::from("/var/cache/apt/archives/")
        };

        path
    }

    pub fn operation_map(&self) -> OmaAptResult<HashMap<OmaOperation, Vec<OperationEntry>>> {
        let mut res = HashMap::new();
        let changes = self.cache.get_changes(false)?;

        res.insert(OmaOperation::Install, vec![]);
        res.insert(OmaOperation::Upgrade, vec![]);
        res.insert(OmaOperation::ReInstall, vec![]);
        res.insert(OmaOperation::Remove, vec![]);
        res.insert(OmaOperation::Downgrade, vec![]);

        for pkg in changes {
            if pkg.marked_install() {
                let cand = pkg
                    .candidate()
                    .take()
                    .ok_or_else(|| OmaAptError::PkgNoCandidate(pkg.name().to_string()))?;

                let uri = cand.uris().collect::<Vec<_>>();
                let version = cand.version();
                let checksum = cand
                    .get_record(RecordField::SHA256)
                    .ok_or_else(|| OmaAptError::PkgNoChecksum(pkg.name().to_string()))?;

                let size = cand.installed_size();

                let install_entry = InstallEntry::new(
                    pkg.name().to_string(),
                    None,
                    version.to_string(),
                    None,
                    size,
                    uri,
                    checksum,
                    pkg.arch().to_string(),
                    cand.size(),
                );

                res.get_mut(&OmaOperation::Install)
                    .unwrap()
                    .push(OperationEntry::Install(install_entry));

                // If the package is marked install then it will also
                // show up as marked upgrade, downgrade etc.
                // Check this first and continue.
                continue;
            }

            if pkg.marked_upgrade() {
                let install_entry = pkg_delta(&pkg)?;

                res.get_mut(&OmaOperation::Upgrade)
                    .unwrap()
                    .push(OperationEntry::Install(install_entry));
            }

            if pkg.marked_delete() {
                let name = pkg.name();
                let is_purge = pkg.marked_purge();

                let mut tags = vec![];
                if is_purge {
                    tags.push(RemoveTag::Purge);
                }
                // TODO: autoremove

                let installed = pkg.installed().unwrap();
                let version = installed.version();
                let size = installed.size();

                let remove_entry =
                    RemoveEntry::new(name.to_string(), version.to_owned(), size, tags);

                res.get_mut(&OmaOperation::Remove)
                    .unwrap()
                    .push(OperationEntry::Remove(remove_entry));
            }

            if pkg.marked_reinstall() {
                let version = pkg.installed().unwrap();

                let checksum = version
                    .get_record(RecordField::SHA256)
                    .ok_or_else(|| OmaAptError::PkgNoChecksum(pkg.name().to_string()))?;

                let install_entry = InstallEntry::new(
                    pkg.name().to_string(),
                    None,
                    version.version().to_string(),
                    Some(version.installed_size()),
                    version.installed_size(),
                    version.uris().collect(),
                    checksum,
                    pkg.arch().to_string(),
                    0,
                );

                res.get_mut(&OmaOperation::ReInstall)
                    .unwrap()
                    .push(OperationEntry::Install(install_entry))
            }

            if pkg.marked_downgrade() {
                let install_entry = pkg_delta(&pkg)?;

                res.get_mut(&OmaOperation::Downgrade)
                    .unwrap()
                    .push(OperationEntry::Install(install_entry));
            }
        }

        Ok(res)
    }
}

fn pkg_delta(new_pkg: &Package) -> OmaAptResult<InstallEntry> {
    let cand = new_pkg
        .candidate()
        .take()
        .ok_or_else(|| OmaAptError::PkgNoCandidate(new_pkg.name().to_string()))?;

    let new_version = cand.version();
    let installed = new_pkg.installed().unwrap();
    let old_version = installed.version();

    let checksum = cand
        .get_record(RecordField::SHA256)
        .ok_or_else(|| OmaAptError::PkgNoChecksum(new_pkg.name().to_string()))?;

    let install_entry = InstallEntry::new(
        new_pkg.name().to_string(),
        Some(old_version.to_string()),
        new_version.to_owned(),
        Some(installed.installed_size()),
        cand.installed_size(),
        cand.uris().collect::<Vec<_>>(),
        checksum,
        new_pkg.arch().to_string(),
        cand.size(),
    );

    Ok(install_entry)
}

pub fn select_pkg(keywords: Vec<&str>, cache: &Cache) -> OmaAptResult<Vec<PkgInfo>> {
    let db = OmaDatabase::new(cache)?;
    let mut pkgs = vec![];
    for keyword in keywords {
        pkgs.extend(match keyword {
            x if x.ends_with(".deb") => db.query_local_glob(x)?,
            x if x.split_once('/').is_some() => db.query_from_branch(x, true)?,
            x if x.split_once('=').is_some() => vec![db.query_from_version(x)?],
            x => db.query_from_glob(x, true)?,
        });
    }

    Ok(pkgs)
}

fn mark_install(cache: &Cache, pkginfo: PkgInfo, reinstall: bool) -> OmaAptResult<()> {
    let pkg = pkginfo.raw_pkg;
    let version = pkginfo.version_raw;
    let ver = Version::new(version, cache);
    let pkg = Package::new(cache, pkg);
    ver.set_candidate();

    if pkg.installed().as_ref() == Some(&ver) && !reinstall {
        tracing::info!("already-installed");

        return Ok(());
    } else if pkg.installed().as_ref() == Some(&ver) && reinstall {
        if ver.uris().next().is_none() {
            return Err(OmaAptError::MarkReinstallError(pkg.name().to_string()));
        }
        pkg.mark_reinstall(true);
    } else {
        pkg.mark_install(true, true);
        if !pkg.marked_install() && !pkg.marked_downgrade() && !pkg.marked_upgrade() {
            // apt 会先就地检查这个包的表面依赖是否满足要求，如果不满足则直接返回错误，而不是先交给 resolver
            // TODO: 依赖信息显示
            tracing::error!("Dep issue: {}", pkg.name());
            return Err(OmaAptError::DependencyIssue);
        }
    }

    pkg.protect();

    Ok(())
}

pub struct OmaAptInstallProgress {
    config: AptConfig,
}

impl OmaAptInstallProgress {
    #[allow(dead_code)]
    pub fn new(
        config: AptConfig,
        yes: bool,
        force_yes: bool,
        dpkg_force_confnew: bool,
        dpkg_force_all: bool,
    ) -> Self {
        if yes {
            rust_apt::raw::config::raw::config_set(
                "APT::Get::Assume-Yes".to_owned(),
                "true".to_owned(),
            );
            tracing::debug!("APT::Get::Assume-Yes is set to true");
        }

        if dpkg_force_confnew {
            config.set("Dpkg::Options::", "--force-confnew");
            tracing::debug!("Dpkg::Options:: is set to --force-confnew");
        } else if yes {
            config.set("Dpkg::Options::", "--force-confold");
            tracing::debug!("Dpkg::Options:: is set to --force-confold");
        }

        if force_yes {
            // warn!("{}", fl!("force-auto-mode"));
            config.set("APT::Get::force-yes", "true");
            tracing::debug!("APT::Get::force-Yes is set to true");
        }

        if dpkg_force_all {
            // warn!("{}", fl!("dpkg-force-all-mode"));
            config.set("Dpkg::Options::", "--force-all");
            tracing::debug!("Dpkg::Options:: is set to --force-all");
        }

        Self { config }
    }

    /// Return the AptInstallProgress in a box
    /// To easily pass through to do_install
    pub fn new_box(
        config: AptConfig,
        yes: bool,
        force_yes: bool,
        dpkg_force_confnew: bool,
        dpkg_force_all: bool,
    ) -> Box<dyn InstallProgress> {
        Box::new(Self::new(
            config,
            yes,
            force_yes,
            dpkg_force_confnew,
            dpkg_force_all,
        ))
    }
}

impl InstallProgress for OmaAptInstallProgress {
    fn status_changed(
        &mut self,
        _pkgname: String,
        steps_done: u64,
        total_steps: u64,
        _action: String,
    ) {
        // Get the terminal's width and height.
        let term_height = terminal_height();
        let term_width = terminal_width();

        // Save the current cursor position.
        print!("\x1b7");

        // Go to the progress reporting line.
        print!("\x1b[{term_height};0f");
        std::io::stdout().flush().unwrap();

        // Convert the float to a percentage string.
        let percent = steps_done as f32 / total_steps as f32;
        let mut percent_str = (percent * 100.0).round().to_string();

        let percent_padding = match percent_str.len() {
            1 => "  ",
            2 => " ",
            3 => "",
            _ => unreachable!(),
        };

        percent_str = percent_padding.to_owned() + &percent_str;

        // Get colors for progress reporting.
        // NOTE: The APT implementation confusingly has 'Progress-fg' for 'bg_color',
        // and the same the other way around.
        let bg_color = self
            .config
            .find("Dpkg::Progress-Fancy::Progress-fg", "\x1b[42m");
        let fg_color = self
            .config
            .find("Dpkg::Progress-Fancy::Progress-bg", "\x1b[30m");
        const BG_COLOR_RESET: &str = "\x1b[49m";
        const FG_COLOR_RESET: &str = "\x1b[39m";

        print!("{bg_color}{fg_color}Progress: [{percent_str}%]{BG_COLOR_RESET}{FG_COLOR_RESET} ");

        // The length of "Progress: [100%] ".
        const PROGRESS_STR_LEN: usize = 17;

        // Print the progress bar.
        // We should safely be able to convert the `usize`.try_into() into the `u32`
        // needed by `get_apt_progress_string`, as usize ints only take up 8 bytes on a
        // 64-bit processor.
        print!(
            "{}",
            get_apt_progress_string(percent, (term_width - PROGRESS_STR_LEN).try_into().unwrap())
        );
        std::io::stdout().flush().unwrap();

        // If this is the last change, remove the progress reporting bar.
        // if steps_done == total_steps {
        // print!("{}", " ".repeat(term_width));
        // print!("\x1b[0;{}r", term_height);
        // }
        // Finally, go back to the previous cursor position.
        print!("\x1b8");
        std::io::stdout().flush().unwrap();
    }

    // TODO: Need to figure out when to use this.
    fn error(&mut self, _pkgname: String, _steps_done: u64, _total_steps: u64, _error: String) {}
}

fn apt_style_filename(filename: &str, version: String) -> OmaAptResult<String> {
    let mut filename_split = filename.split('_');

    let package = filename_split
        .next()
        .take()
        .ok_or_else(|| OmaAptError::InvalidFileName(filename.to_owned()))?;

    let arch_deb = filename_split
        .nth(1)
        .take()
        .ok_or_else(|| OmaAptError::InvalidFileName(filename.to_owned()))?;

    let arch_deb = if arch_deb == "noarch.deb" {
        "all.deb"
    } else {
        arch_deb
    };

    let version = version.replace(':', "%3a");
    let filename = format!("{package}_{version}_{arch_deb}").replace("%2b", "+");

    Ok(filename)
}
