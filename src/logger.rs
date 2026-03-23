use std::{
    borrow::Cow,
    env,
    path::{Path, PathBuf},
    sync::Arc,
    thread::{self, JoinHandle},
};

use oma_console::OmaFormatter;
use spdlog::{
    Level, LevelFilter, Logger, debug,
    error::SendToChannelError,
    init_log_crate_proxy, log_crate_proxy, set_default_logger,
    sink::{AsyncPoolSink, FileSink, StdStreamSink},
    warn,
};

use crate::{args::OhManagerAilurus, config::OmaConfig, root::is_root};

pub fn init_logger(oma: &OhManagerAilurus) -> anyhow::Result<PathBuf> {
    let debug = oma.global.debug;
    let dry_run = oma.global.dry_run;

    let log_file = (if is_root() {
        PathBuf::from(&oma.global.sysroot).join("var/log/oma")
    } else {
        dirs::state_dir()
            .expect("Failed to get state dir")
            .join("oma")
    })
    .join(format!("oma.log.{}", chrono::Local::now().timestamp()));

    init_log_crate_proxy().unwrap();

    let debug_formatter = oma.debug_formatter();

    let (level_filter, formatter, filter) = if !debug && !dry_run && env::var("OMA_LOG").is_err() {
        let no_i18n_embd = env_filter::Builder::new()
            .try_parse("i18n_embed=off,info")
            .unwrap()
            .build();

        let level_filter = LevelFilter::MoreSevereEqual(Level::Info);

        let formatter = OmaFormatter::new().with_ansi(oma.enable_ansi());

        (level_filter, formatter, no_i18n_embd)
    } else {
        let filter = env_filter::Builder::new()
            .try_parse(
                &env::var("OMA_LOG")
                    .or_else(|_| env::var("RUST_LOG"))
                    .map(Cow::Owned)
                    .unwrap_or(Cow::Borrowed("hyper=off,rustls=off,debug")),
            )
            .unwrap()
            .build();

        let level_filter = LevelFilter::MoreSevereEqual(Level::Debug);

        (level_filter, debug_formatter.clone(), filter)
    };

    log_crate_proxy().set_filter(Some(filter));

    let file_sink = FileSink::builder()
        .path(&log_file)
        .formatter(debug_formatter)
        .level_filter(LevelFilter::MoreSevereEqual(Level::Debug))
        .build();

    let mut file_sink_error = None;

    let file_sink = match file_sink {
        Ok(file_sink) => Some(
            AsyncPoolSink::builder()
                .sink(Arc::new(file_sink))
                .overflow_policy(spdlog::sink::OverflowPolicy::DropIncoming)
                .build()
                .unwrap(),
        ),
        Err(e) => {
            file_sink_error = Some(e);
            None
        }
    };

    let stream_sink = StdStreamSink::builder()
        .formatter(formatter)
        .level_filter(level_filter)
        .stderr()
        .build()
        .unwrap();

    let mut logger_builder = Logger::builder();

    logger_builder
        .level_filter(LevelFilter::All)
        .flush_level_filter(LevelFilter::All)
        .error_handler(|err| {
            match err {
                spdlog::Error::SendToChannel(SendToChannelError::Full, _) => {
                    // Ignore, the async pool sink is dropping logs
                }
                err => spdlog::ErrorHandler::default().call(err),
            }
        })
        .sink(Arc::new(stream_sink));

    if let Some(file_sink) = file_sink {
        logger_builder.sink(Arc::new(file_sink));
    }

    let logger = logger_builder.build().unwrap();

    set_default_logger(Arc::new(logger));

    if let Some(e) = file_sink_error {
        Err(e.into())
    } else {
        Ok(log_file)
    }
}

pub fn remove_old_log_file_impl(
    file: anyhow::Result<PathBuf>,
    config_ctx: &OmaConfig,
) -> Option<JoinHandle<()>> {
    match file {
        Ok(file) => {
            debug!("Log file: {}", file.display());
            let save_log_count = config_ctx.save_log_count;

            Some(thread::spawn(move || {
                remove_old_log_file(save_log_count, &file)
            }))
        }
        Err(e) => {
            warn!("Failed to write log to file: {e}");
            None
        }
    }
}

fn remove_old_log_file(save_log_count: usize, log_file: &Path) {
    let mut v = vec![];
    let log_dir = log_file.parent().unwrap();
    let dirs = std::fs::read_dir(log_dir)
        .expect("Failed to read log dir")
        .collect::<Vec<_>>();

    for p in &dirs {
        let Ok(p) = p else {
            continue;
        };

        let file_name = p.file_name();
        let file_name = file_name.to_string_lossy();
        let Some((prefix, timestamp)) = file_name.rsplit_once('.') else {
            continue;
        };

        if prefix != "oma.log" {
            continue;
        }

        let Ok(timestamp) = timestamp.parse::<usize>() else {
            continue;
        };

        v.push(timestamp);
    }

    if v.len() > save_log_count {
        v.sort_unstable_by(|a, b| b.cmp(a));

        for _ in 1..=(v.len() - save_log_count) {
            let Some(pop) = v.pop() else {
                break;
            };

            let log_path = log_dir.join(format!("oma.log.{pop}"));
            if let Err(e) = std::fs::remove_file(&log_path) {
                debug!("Failed to remove file {}: {}", log_path.display(), e);
            }
        }
    }
}
