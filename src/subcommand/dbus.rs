use std::thread;

use clap::Args;
use flume::unbounded;
use oma_pm::{
    CommitNetworkConfig,
    apt::{AptConfig, OmaApt, OmaAptArgs},
    matches::{GetArchMethod, PackagesMatcher},
    sort::SummarySort,
};
use oma_utils::zbus::{self, interface};
use tracing::debug;
use zbus::Connection;

use crate::{
    HTTP_CLIENT, args::CliExecuter, config::Config, error::OutputError,
    install_progress::NoInstallProgressManager,
};

#[derive(Debug, Args)]
struct DBus;

struct OmaDbus;

#[interface(name = "io.aosc.OmaService1")]
impl OmaDbus {
    fn install(&self, debs: Vec<String>) -> String {
        thread::spawn(move || {
            let mut apt = OmaApt::new(
                debs.iter()
                    .filter(|x| x.ends_with(".deb"))
                    .map(|x| x.to_string())
                    .collect(),
                OmaAptArgs::builder().build(),
                false,
                AptConfig::new(),
            )
            .unwrap();

            let matcher = PackagesMatcher::builder()
                .cache(&apt.cache)
                .filter_candidate(true)
                .filter_downloadable_candidate(false)
                .select_dbg(false)
                .native_arch(GetArchMethod::DirectRoot)
                .build();

            let (pkgs, _) = matcher
                .match_pkgs_and_versions(debs.iter().map(|x| x.as_str()))
                .unwrap();

            apt.install(&pkgs, false).unwrap();
            apt.resolve(true, false).unwrap();

            let op = apt
                .summary(SummarySort::default(), |_| false, |_| false)
                .unwrap();

            let (download_tx, download_rx) = unbounded();

            let res = apt.commit(
                Box::new(NoInstallProgressManager),
                &op,
                &HTTP_CLIENT,
                CommitNetworkConfig {
                    auth_config: None,
                    network_thread: None,
                },
                None,
                |event| async {
                    if let Err(e) = download_tx.send_async(event).await {
                        debug!("Send progress channel got error: {}; maybe check archive work still in progress", e);
                    }
                },
            ).unwrap();

              
        });

        "ok".to_string()
    }
}

impl CliExecuter for DBus {
    fn execute(self, config: &Config, no_progress: bool) -> Result<i32, OutputError> {
        todo!()
    }
}

async fn create_session() -> anyhow::Result<Connection> {
    let conn = zbus::connection::Builder::system()?
        .name("io.aosc.Oma")?
        .serve_at("/io/aosc/Oma", OmaDbus)?
        .build()
        .await?;

    debug!("oma dbus service session created");

    Ok(conn)
}
