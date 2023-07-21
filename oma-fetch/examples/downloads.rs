use std::path::PathBuf;

use oma_fetch::{DownloadEntry, DownloadResult, DownloadSource, DownloadSourceType, OmaFetcher};
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> DownloadResult<()> {
    let source_1 = DownloadSource::new(
        "https://mirrors.bfsu.edu.cn/anthon/debs/pool/stable/main/g/go_1.19.4%2Btools0.4.0%2Bnet0.4.0-0_amd64.deb".to_string(),
        DownloadSourceType::Http
    );

    let file_1 = DownloadEntry::new(
        vec![source_1],
        "go.deb".to_string(),
        PathBuf::from("./oma-fetcher-test"),
        Some("0625cbba48a14438eea144682567a026a17e173420c5bdcbc06dcb11aba50628".to_string()),
        true,
        None,
    );

    let source_2 = DownloadSource::new(
        "https://mirrors.bfsu.edu.cn/anthon/debs/pool/stable/main/v/vscodium_1.77.3.23102-0_arm64.deb".to_string(),
        DownloadSourceType::Http
    );

    let file_2 = DownloadEntry::new(
        vec![source_2],
        "vscode.deb".to_string(),
        PathBuf::from("./oma-fetcher-test"),
        None,
        true,
        None,
    );

    let mut test_local_file = tokio::fs::File::create("test").await?;
    test_local_file.write_all(b"test").await?;

    let source_3 = DownloadSource::new("./test".to_string(), DownloadSourceType::Local);

    let file_3 = DownloadEntry::new(
        vec![source_3],
        "test_downloaded".to_string(),
        PathBuf::from("./oma-fetcher-test"),
        None,
        false,
        None,
    );

    let fetcher = OmaFetcher::new(None, true, None, vec![file_1, file_2, file_3], None)?;

    tokio::fs::create_dir_all("./oma-fetcher-test").await?;
    fetcher
        .start_download()
        .await
        .into_iter()
        .collect::<DownloadResult<Vec<_>>>()?;

    Ok(())
}
