use std::path::PathBuf;

use oma_fetch::{DownloadEntry, DownloadResult, DownloadSourceType, OmaFetcher};
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> DownloadResult<()> {
    let file_1 = DownloadEntry::new("https://mirrors.bfsu.edu.cn/anthon/debs/pool/stable/main/g/go_1.19.4%2Btools0.4.0%2Bnet0.4.0-0_amd64.deb".to_string(),
        "go.deb".to_string(),
        PathBuf::from("./oma-fetcher-test"),
        Some("0625cbba48a14438eea144682567a026a17e173420c5bdcbc06dcb11aba50628".to_string()),
        true,
        DownloadSourceType::Http
    );

    let file_2 = DownloadEntry::new("https://mirrors.bfsu.edu.cn/anthon/debs/pool/stable/main/v/vscodium_1.77.3.23102-0_arm64.deb".to_string(),
        "vscode.deb".to_string(),
        PathBuf::from("./oma-fetcher-test"),
        None,
        true,
        DownloadSourceType::Http
    );

    let mut test_local_file = tokio::fs::File::create("test").await?;
    test_local_file.write_all(b"test").await?;

    let file_3 = DownloadEntry::new(
        "./test".to_string(),
        "test_downloaded".to_string(),
        PathBuf::from("./oma-fetcher-test"),
        None,
        false,
        DownloadSourceType::Local,
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
