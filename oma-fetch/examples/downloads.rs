use std::{path::PathBuf, sync::Arc};

use dashmap::DashMap;
use indicatif::{MultiProgress, ProgressBar};
use oma_console::{
    pb::{oma_spinner, oma_style_pb},
    writer::Writer,
};
use oma_fetch::{
    DownloadEntryBuilder, DownloadEvent, DownloadResult, DownloadSource, DownloadSourceType,
    OmaFetcher,
};
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> DownloadResult<()> {
    let source_1 = DownloadSource::new(
        "https://mirrors.bfsu.edu.cn/anthon/debs/pool/stable/main/g/go_1.19.4%2Btools0.4.0%2Bnet0.4.0-0_amd64.deb".to_string(),
        DownloadSourceType::Http
    );

    let file_1 = DownloadEntryBuilder::default()
        .source(vec![source_1])
        .filename("go.deb".to_string())
        .dir(PathBuf::from("./oma-fetcher-test"))
        .hash("0625cbba48a14438eea144682567a026a17e173420c5bdcbc06dcb11aba50628".to_string())
        .allow_resume(true)
        .build()?;

    let source_2 = DownloadSource::new(
        "https://mirrors.bfsu.edu.cn/anthon/debs/pool/stable/main/v/vscodium_1.77.3.23102-0_arm64.deb".to_string(),
        DownloadSourceType::Http
    );

    let file_2 = DownloadEntryBuilder::default()
        .source(vec![source_2])
        .filename("vscode.deb".to_string())
        .dir(PathBuf::from("./oma-fetcher-test"))
        .allow_resume(false)
        .build()?;

    let mut test_local_file = tokio::fs::File::create("test").await?;
    test_local_file.write_all(b"test").await?;

    // let source_3 = DownloadSource::new("./test".to_string(), DownloadSourceType::Local);

    // let file_3 = DownloadEntryBuilder::default()
    //     .source(vec![source_3])
    //     .filename("test_downloaded".to_string())
    //     .dir(PathBuf::from("./oma-fetcher-test"))
    //     .build()?;

    let fetcher = OmaFetcher::new(None, vec![file_1, file_2], None)?;

    tokio::fs::create_dir_all("./oma-fetcher-test").await?;

    let mb = Arc::new(MultiProgress::new());
    let pb_map: DashMap<usize, ProgressBar> = DashMap::new();

    fetcher
        .start_download(move |count, event| match event {
            DownloadEvent::ChecksumMismatchRetry { filename, times } => {
                mb.println(format!(
                    "{filename} checksum failed, retrying {times} times"
                ))
                .unwrap();
            }
            DownloadEvent::GlobalProgressSet(size) => {
                if let Some(pb) = pb_map.get(&0) {
                    pb.set_position(size);
                }
            }
            DownloadEvent::GlobalProgressInc(size) => {
                if let Some(pb) = pb_map.get(&0) {
                    pb.inc(size);
                }
            }
            DownloadEvent::ProgressDone => {
                if let Some(pb) = pb_map.get(&(count + 1)) {
                    pb.finish_and_clear();
                }
            }
            DownloadEvent::NewProgressSpinner(msg) => {
                let (sty, inv) = oma_spinner(false).unwrap();
                let pb = mb.insert(count + 1, ProgressBar::new_spinner().with_style(sty));
                pb.set_message(msg);
                pb.enable_steady_tick(inv);
                pb_map.insert(count + 1, pb);
            }
            DownloadEvent::NewProgress(size, msg) => {
                let sty = oma_style_pb(Writer::default(), false).unwrap();
                let pb = mb.insert(count + 1, ProgressBar::new(size).with_style(sty));
                pb.set_message(msg);
                pb_map.insert(count + 1, pb);
            }
            DownloadEvent::ProgressInc(size) => {
                let pb = pb_map.get(&(count + 1)).unwrap();
                pb.inc(size);
            }
            DownloadEvent::CanNotGetSourceNextUrl(e) => {
                mb.println(format!("Error: {e}")).unwrap();
            }
            DownloadEvent::Done(_) => {
                return;
            }
        })
        .await
        .into_iter()
        .collect::<DownloadResult<Vec<_>>>()?;

    Ok(())
}
