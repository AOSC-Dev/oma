use std::path::PathBuf;

use oma_fetcher::{DownloadEntry, DownloadResult, OmaFetcher};

#[tokio::main]
async fn main() -> DownloadResult<()> {
    let file_1 = DownloadEntry::new("https://mirrors.bfsu.edu.cn/anthon/debs/pool/stable/main/g/go_1.19.4%2Btools0.4.0%2Bnet0.4.0-0_amd64.deb".to_string(),
        "go.deb".to_string(),
        PathBuf::from("./oma-fetcher-test"),
        Some("0625cbba48a14438eea144682567a026a17e173420c5bdcbc06dcb11aba50628".to_string()),
        true
    );

    let file_2 = DownloadEntry::new("https://mirrors.bfsu.edu.cn/anthon/debs/pool/stable/main/v/vscodium_1.77.3.23102-0_arm64.deb".to_string(),
        "vscode.deb".to_string(),
        PathBuf::from("./oma-fetcher-test"),
        None,
        true
    );

    let fetcher = OmaFetcher::new(None, true, None, vec![file_1, file_2], None)?;

    tokio::fs::create_dir_all("./oma-fetcher-test").await?;
    fetcher.start_download().await?;

    Ok(())
}
