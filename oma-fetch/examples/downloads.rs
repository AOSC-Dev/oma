use std::path::PathBuf;

use oma_fetch::{
    DownloadEntryBuilder, DownloadResult, DownloadSource, DownloadSourceType, OmaFetcher,
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

    let source_3 = DownloadSource::new("./test".to_string(), DownloadSourceType::Local);

    let file_3 = DownloadEntryBuilder::default()
        .source(vec![source_3])
        .filename("test_downloaded".to_string())
        .dir(PathBuf::from("./oma-fetcher-test"))
        .build()?;

    let fetcher = OmaFetcher::new(None, true, None, vec![file_1, file_2, file_3], None)?;

    tokio::fs::create_dir_all("./oma-fetcher-test").await?;
    fetcher
        .start_download()
        .await
        .into_iter()
        .collect::<DownloadResult<Vec<_>>>()?;

    Ok(())
}
