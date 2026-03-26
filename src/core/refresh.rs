use std::thread;

use apt_auth_config::AuthConfig;
use bon::Builder;
use flume::unbounded;
use oma_pm::apt::AptConfig;
use oma_refresh::db::OmaRefresh;
use oma_utils::dpkg::dpkg_arch;
use spdlog::{debug, info};

use crate::{
    RT,
    config::OmaConfig,
    error::OutputError,
    fl,
    pb::{NoProgressBar, OmaMultiProgressBar, RenderRefreshProgress},
    utils::get_lists_dir,
};

#[derive(Debug, Builder)]
pub struct Refresh<'a> {
    config: &'a OmaConfig,
    apt_config: &'a AptConfig,
    auth_config: Option<&'a AuthConfig>,
}

impl Refresh<'_> {
    pub fn run(self) -> Result<(), OutputError> {
        let Refresh {
            config,
            apt_config,
            auth_config,
        } = self;

        if config.dry_run {
            return Ok(());
        }

        info!("{}", fl!("refreshing-repo-metadata"));

        let sysroot = &config.sysroot;
        let arch = dpkg_arch(sysroot)?;

        let refresh = OmaRefresh::builder()
            .download_dir(get_lists_dir(apt_config))
            .source(sysroot.clone())
            .threads(config.download_threads)
            .arch(arch)
            .apt_config(apt_config)
            .client(config.http_client()?)
            .another_apt_options(&config.apt_options)
            .maybe_auth_config(auth_config);

        #[cfg(feature = "aosc")]
        let msg = fl!("do-not-edit-topic-sources-list");

        #[cfg(feature = "aosc")]
        let refresh = refresh
            .refresh_topics(!config.no_refresh_topics)
            .topic_msg(&msg)
            .build();

        #[cfg(not(feature = "aosc"))]
        let refresh = refresh.build();

        let (tx, rx) = unbounded();

        let no_progress = config.no_progress();

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
