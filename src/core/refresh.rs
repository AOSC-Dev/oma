use std::{path::PathBuf, thread};

use apt_auth_config::AuthConfig;
use bon::Builder;
use flume::unbounded;
use oma_pm::apt::AptConfig;
use oma_refresh::db::OmaRefresh;
use oma_utils::dpkg::dpkg_arch;
use reqwest::Client;
use spdlog::{debug, info};

use crate::{
    RT,
    error::OutputError,
    fl,
    pb::{NoProgressBar, OmaMultiProgressBar, RenderRefreshProgress},
    utils::get_lists_dir,
};

#[derive(Debug, Builder)]
pub struct Refresh<'a> {
    client: &'a Client,
    #[builder(default)]
    dry_run: bool,
    #[builder(default)]
    no_progress: bool,
    #[builder(default = 4)]
    network_thread: usize,
    #[builder(default = "/")]
    sysroot: &'a str,
    #[builder(default = true)]
    refresh_topics: bool,
    config: &'a AptConfig,
    auth_config: Option<&'a AuthConfig>,
    apt_options: &'a [String],
}

impl Refresh<'_> {
    pub fn run(self) -> Result<(), OutputError> {
        let Refresh {
            client,
            dry_run,
            no_progress,
            network_thread,
            sysroot,
            refresh_topics,
            config,
            auth_config,
            apt_options,
        } = self;

        #[cfg(not(feature = "aosc"))]
        let _ = refresh_topics;

        if dry_run {
            return Ok(());
        }

        info!("{}", fl!("refreshing-repo-metadata"));

        let sysroot = PathBuf::from(sysroot);

        let arch = dpkg_arch(&sysroot)?;

        let refresh = OmaRefresh::builder()
            .download_dir(get_lists_dir(config))
            .source(sysroot)
            .threads(network_thread)
            .arch(arch)
            .apt_config(config)
            .client(client)
            .another_apt_options(apt_options)
            .maybe_auth_config(auth_config);

        #[cfg(feature = "aosc")]
        let msg = fl!("do-not-edit-topic-sources-list");

        #[cfg(feature = "aosc")]
        let refresh = refresh
            .refresh_topics(refresh_topics)
            .topic_msg(&msg)
            .build();

        #[cfg(not(feature = "aosc"))]
        let refresh = refresh.build();

        let (tx, rx) = unbounded();

        thread::spawn(move || {
            let mut pb: Box<dyn RenderRefreshProgress> = if no_progress {
                Box::new(NoProgressBar::default())
            } else {
                Box::new(OmaMultiProgressBar::default())
            };
            pb.render_refresh_progress(&rx);
        });

        RT.block_on(async move {
            refresh
                .start(async |event| {
                    if let Err(e) = tx.send_async(event).await {
                        debug!("{}", e);
                    }
                })
                .await
        })?;

        Ok(())
    }
}
