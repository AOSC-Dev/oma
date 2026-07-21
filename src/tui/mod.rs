use std::{
    thread,
    time::Duration,
};

use clap::Args;
use oma_console::pager::{exit_tui, prepare_create_tui};
use oma_pm::{
    apt::{OmaApt, OmaAptArgs, Upgrade},
    search::{IndiciumSearch, OmaSearch, SearchResult, SearchType},
};
use oma_refresh::db::{Event as RefreshEvent, OmaRefresh};
use render::{Task, Tui as TuiInner};
use spdlog::{debug, info};
use zbus::Connection;

use crate::{
    RT,
    args::CliExecuter,
    config::OmaConfig,
    error::OutputError,
    exit_handle::ExitHandle,
    fl,
    root::root,
    subcommand::{search::AmoProxy, utils::lock_oma},
    tui::render::PackageStatus,
    utils::get_lists_dir,
};
use crate::{core::commit_changes::CommitChanges, dbus::dbus_check};

mod key_binding;
mod refresh;
mod render;
mod state;
mod window;

#[derive(Debug, Args, Default)]
pub struct Tui {
    /// Fix apt broken status
    #[arg(short, long)]
    fix_broken: bool,
    /// Do not fix dpkg broken status
    #[arg(short, long)]
    no_fix_dpkg_status: bool,
    /// Install package(s) without fsync(2)
    #[arg(long)]
    force_unsafe_io: bool,
    /// Do not refresh repository metadata
    #[arg(long)]
    no_refresh: bool,
    /// Ignore repository and package dependency issues
    #[arg(long)]
    force_yes: bool,
    /// Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)
    #[arg(long)]
    force_confnew: bool,
    /// Remove package(s) also remove configuration file(s), like apt purge
    #[arg(long, visible_alias = "purge")]
    remove_config: bool,
    /// Do not clean local package cache
    #[arg(long, help = fl!("clap-noclean-help"), env = "OMA_NO_CLEAN", value_parser = clap::builder::FalseyValueParser::new())]
    no_clean: bool,
}

pub(crate) enum Searcher {
    Local(Box<IndiciumSearch>),
    Amo {
        _connection: Connection,
        proxy: AmoProxy<'static>,
    },
}

impl Searcher {
    async fn connect_amo() -> anyhow::Result<Searcher> {
        let connection = Connection::system().await?;

        let peer_proxy = zbus::fdo::PeerProxy::builder(&connection)
            .destination("io.aosc.Amo")?
            .path("/io/aosc/Amo")?
            .build()
            .await?;

        peer_proxy
            .ping()
            .await
            .inspect_err(|e| debug!("Failed to connect amo: {e}"))?;

        let proxy = AmoProxy::new(&connection).await?;

        Ok(Searcher::Amo {
            _connection: connection,
            proxy,
        })
    }

    pub(crate) fn search(&self, query: &str) -> anyhow::Result<Vec<SearchResult>> {
        match self {
            Searcher::Local(indicium_search) => Ok(indicium_search.search(query)?),
            Searcher::Amo { proxy, .. } => {
                Ok(serde_json::from_str(&RT.block_on(proxy.search(query))?)?)
            }
        }
    }
}

impl CliExecuter for Tui {
    fn execute(self, config: OmaConfig) -> Result<ExitHandle, OutputError> {
        let Tui {
            fix_broken,
            force_unsafe_io,
            no_refresh,
            force_yes,
            force_confnew,
            remove_config,
            no_fix_dpkg_status,
            no_clean,
        } = self;

        if config.dry_run {
            info!("Running in dry-run mode, Exit.");
            return Ok(ExitHandle::default());
        }

        root()?;
        let _lock_fd = lock_oma(&config.sysroot)?;
        let _fds = dbus_check(false, &config)?;

        let mut terminal = prepare_create_tui().map_err(|e| OutputError {
            description: "BUG: Failed to create crossterm instance".to_string(),
            source: Some(Box::new(e)),
        })?;

        let sysroot = config.sysroot.clone();
        let arch = oma_utils::dpkg::dpkg_arch(&sysroot)?;
        let download_dir = get_lists_dir();
        let client = config.http_client()?.clone();

        let (tx, rx) = flume::unbounded::<RefreshEvent>();

        if !no_refresh {
            let event_tx = tx.clone();
            thread::spawn(move || {
                let refresh = OmaRefresh::builder()
                    .download_dir(download_dir)
                    .source(sysroot)
                    .threads(config.download_threads)
                    .arch(arch)
                    .client(client);

                #[cfg(feature = "aosc")]
                let refresh = refresh
                    .refresh_topics(!config.no_refresh_topics)
                    .topic_msg(fl!("do-not-edit-topic-sources-list").into());

                let refresh = refresh.build();
                let _ = refresh.start(move |event| {
                    let _ = event_tx.send(event);
                });
            });
        } else {
            // Signal done immediately so the loading loop exits
            let _ = tx.send(RefreshEvent::Done);
        }

        // Loading screen state
        let mut download_items: Vec<refresh::DownloadItem> = vec![];
        let mut status_text = fl!("refreshing-repo-metadata");

        let mut done = false;
        while !done {
            while let Ok(event) = rx.try_recv() {
                match event {
                    RefreshEvent::Done => done = true,
                    RefreshEvent::DownloadEvent(fetch_event) => {
                        refresh::update_download_items(&mut download_items, fetch_event);
                    }
                    RefreshEvent::ScanningTopic => {
                        status_text = "Scanning topics...".to_string();
                    }
                    RefreshEvent::ClosingTopic(t) => {
                        status_text = fl!("scan-topic-is-removed", name = t);
                    }
                    RefreshEvent::RunInvokeScript => {
                        status_text = fl!("oma-refresh-success-invoke");
                    }
                    RefreshEvent::SourceListFileNotSupport { path } => {
                        status_text = format!(
                            "{}",
                            fl!(
                                "unsupported-sources-list",
                                p = path.to_string_lossy(),
                                list = ".list",
                                sources = ".sources"
                            )
                        );
                    }
                    RefreshEvent::TopicNotInMirror { .. } => {}
                }
            }

            terminal
                .draw(|f| {
                    render::render_refresh_ui(f, &download_items, &status_text);
                })
                .map_err(|e| OutputError {
                    description: format!("TUI draw error: {e}"),
                    source: Some(Box::new(e)),
                })?;

            thread::sleep(Duration::from_millis(50));
        }

        let oma_apt_args = OmaAptArgs::builder()
            .sysroot(config.sysroot.to_string_lossy().to_string())
            .another_apt_options(&config.apt_options)
            .dpkg_force_confnew(force_confnew)
            .dpkg_force_unsafe_io(force_unsafe_io)
            .force_yes(force_yes)
            .build();

        let mut apt = OmaApt::new(vec![], oma_apt_args, false)?;

        let (upgradable, upgradable_but_held) = apt.count_pending_upgradable_pkgs();
        let autoremove_cnt = apt.count_pending_autoremovable_pkgs();
        let installed = apt.count_installed_packages();

        let searcher = if config.amo && !config.no_check_dbus {
            match RT.block_on(Searcher::connect_amo()) {
                Ok(s) => s,
                Err(_) => Searcher::Local(Box::new(IndiciumSearch::new(
                    &apt.cache,
                    SearchType::Live,
                    |_| {},
                )?)),
            }
        } else {
            Searcher::Local(Box::new(IndiciumSearch::new(
                &apt.cache,
                SearchType::Live,
                |_| {},
            )?))
        };

        let tui = TuiInner::new(
            &apt,
            PackageStatus {
                installed,
                upgradable,
                upgradable_but_held,
                autoremove: autoremove_cnt,
            },
            searcher,
        );

        let Task {
            execute_apt,
            install,
            remove,
            upgrade,
            autoremove,
        } = tui
            .run(&mut terminal, Duration::from_millis(250))
            .map_err(|e| OutputError {
                description: format!("TUI error: {e}"),
                source: Some(Box::new(e)),
            })?;

        exit_tui(&mut terminal).map_err(|e| OutputError {
            description: "BUG: Failed to exit tui".to_string(),
            source: Some(Box::new(e)),
        })?;

        let mut exit = ExitHandle::default();

        if execute_apt {
            if upgrade {
                apt.upgrade(Upgrade::FullUpgrade)?;
            }

            apt.install(&install, false)?;
            apt.remove(
                remove
                    .iter()
                    .flat_map(|x| x.into_oma_package_without_version()),
                false,
                !autoremove,
            )?;

            exit = CommitChanges::builder()
                .apt(apt)
                .no_fixbroken(!fix_broken)
                .fix_dpkg_status(!no_fix_dpkg_status)
                .yes(false)
                .remove_config(remove_config)
                .autoremove(autoremove)
                .check_tum(upgrade)
                .is_upgrade(upgrade)
                .config(&config)
                .no_clean(no_clean)
                .build()
                .run()?;
        }

        Ok(exit)
    }
}
