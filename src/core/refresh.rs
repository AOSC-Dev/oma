use std::thread;

use bon::Builder;
use flume::unbounded;
use oma_logger::{debug, info};
use oma_refresh::db::OmaRefresh;
use oma_utils::dpkg::dpkg_arch;

use crate::{
    config::OmaConfig, error::OutputError, fl, pb::ProgressRenderer, utils::get_lists_dir,
};

#[derive(Debug, Builder)]
pub struct Refresh<'a> {
    config: &'a OmaConfig,
}

impl Refresh<'_> {
    pub fn run(self) -> Result<(), OutputError> {
        let Refresh { config } = self;

        if config.dry_run {
            return Ok(());
        }

        info!("{}", fl!("refreshing-repo-metadata"));

        let sysroot = &config.sysroot;
        let arch = dpkg_arch(sysroot)?;

        let refresh = OmaRefresh::builder()
            .download_dir(get_lists_dir())
            .source(sysroot.clone())
            .threads(config.download_threads)
            .arch(arch)
            .client(config.http_client()?.clone())
            // 传入 oma 已初始化的 APT 配置（含 sysroot 的 Dir、apt_options、
            // 二进制缓存路径等），OmaRefresh 不再自己从系统默认值新建一份。
            // `Arc::clone` 只是引用计数 +1，不深拷贝配置树。
            .apt_config(config.apt_config_arc().clone());

        #[cfg(feature = "aosc")]
        let msg = fl!("do-not-edit-topic-sources-list");

        #[cfg(feature = "aosc")]
        let refresh = refresh
            .refresh_topics(!config.no_refresh_topics)
            .topic_msg(msg.into())
            .build();

        #[cfg(not(feature = "aosc"))]
        let refresh = refresh.build();

        let (tx, rx) = unbounded();

        let no_progress = config.no_progress();

        let handle = thread::spawn(move || {
            let mut pb = ProgressRenderer::new(no_progress);
            pb.render_refresh_progress(&rx);
        });

        let res = refresh.start(move |event| {
            if let Err(e) = tx.send(event) {
                debug!("{}", e);
            }
        });

        // Wait for the renderer thread to process the remaining events so no
        // progress bar is left on screen when we continue.
        handle.join().ok();

        res?;

        Ok(())
    }
}
