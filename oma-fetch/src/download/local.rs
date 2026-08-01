//! Local (`file://`) source download implementation for
//! [`SingleDownloader`](super::SingleDownloader): symlinking, copying and
//! checksum verification of already-present files.

use std::path::Path;

use flume::Sender;
use spdlog::{debug, trace};
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt as _, AsyncWriteExt},
};
use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};

use crate::{
    DownloadSource, Event,
    checksum::{Checksum, ChecksumValidator},
};

use super::{
    READ_FILE_BUFSIZE, SingleDownloader, decompress_reader, error::SingleDownloadError,
    progress::ProgressReporter, verify,
};

impl SingleDownloader {
    /// Download local source file
    pub(super) async fn download_local(
        &self,
        source: &DownloadSource,
        as_symlink: bool,
        tx: &Sender<Event>,
    ) -> Result<bool, SingleDownloadError> {
        debug!("{:?}", self.entry);

        let url_path = Path::new(source.url.strip_prefix("file:").unwrap());

        // Probe the source up front: a missing file fails fast here, before
        // any symlink or copy is attempted.
        let total_size = fs::metadata(url_path)
            .await
            .map_err(SingleDownloadError::Open)?
            .len();

        // A stale symlink, or an existing file when linking, must not stay in
        // the way of the destination.
        let file = self.entry.dir.join(&*self.entry.filename);
        if file.is_symlink() || (as_symlink && file.is_file()) {
            fs::remove_file(&file)
                .await
                .map_err(SingleDownloadError::Remove)?;
        }

        if as_symlink {
            self.symlink_local(tx, url_path).await?;
        } else {
            self.copy_local(tx, url_path, total_size).await?;
        }

        Ok(true)
    }

    /// Symlink the destination to the local source, after verifying its
    /// checksum.
    async fn symlink_local(
        &self,
        tx: &Sender<Event>,
        url_path: &Path,
    ) -> Result<(), SingleDownloadError> {
        if let Some(hash) = &self.entry.hash {
            self.checksum_local(tx, url_path, hash).await?;
        }

        let file = self.entry.dir.join(&*self.entry.filename);
        fs::symlink(url_path, file)
            .await
            .map_err(SingleDownloadError::CreateSymlink)?;

        Ok(())
    }

    /// Copy the local source into the destination, decompressing as needed.
    async fn copy_local(
        &self,
        tx: &Sender<Event>,
        url_path: &Path,
        total_size: u64,
    ) -> Result<(), SingleDownloadError> {
        let mut progress = ProgressReporter::new(tx, self.download_list_index, self.total);
        let msg = self.download_message();
        progress.start_determinate(&msg, total_size);

        trace!("Path for file: {}", url_path.display());

        let from = File::open(url_path)
            .await
            .map_err(SingleDownloadError::Open)?;
        let from = tokio::io::BufReader::new(from).compat();

        trace!("Successfully opened file: {}", url_path.display());

        let mut to = File::create(self.entry.dir.join(&*self.entry.filename))
            .await
            .map_err(SingleDownloadError::Create)?;

        let mut reader = decompress_reader(from, self.entry.file_type).compat();

        trace!(
            "Successfully created file: {}",
            self.entry.dir.join(&*self.entry.filename).display()
        );

        // On success, clear the per-file bar while keeping the global bytes;
        // on failure, dropping the reporter also undoes the global bytes.
        match self
            .download_local_copy(&mut to, &mut reader, &msg, total_size, &mut progress)
            .await
        {
            Ok(()) => {
                progress.finish();
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    /// Copy loop for a local source. Progress is reported on success; on
    /// failure the plain error is returned and [`download_local`] cleans up
    /// the reported bytes.
    async fn download_local_copy<R>(
        &self,
        to: &mut File,
        reader: &mut R,
        msg: &str,
        total_size: u64,
        progress: &mut ProgressReporter,
    ) -> Result<(), SingleDownloadError>
    where
        R: tokio::io::AsyncRead + Unpin + Send,
    {
        let mut v = self
            .entry
            .hash
            .as_ref()
            .map(|v| v.get_validator())
            .unwrap_or(ChecksumValidator::None);

        let mut buf = vec![0u8; READ_FILE_BUFSIZE];
        let mut self_progress = 0;

        loop {
            let size = match reader.read(&mut buf[..]).await {
                Ok(size) => size,
                Err(e) => return Err(SingleDownloadError::Read(e)),
            };

            if size == 0 {
                break;
            }

            to.write_all(&buf[..size])
                .await
                .map_err(SingleDownloadError::Write)?;

            self_progress += size;

            v.update(&buf[..size]);
            progress.update(msg, self_progress as u64, Some(total_size));
        }

        if !v.finish() {
            return Err(SingleDownloadError::ChecksumMismatch);
        }

        Ok(())
    }

    /// Verify a local source file against its checksum before symlinking.
    async fn checksum_local(
        &self,
        tx: &Sender<Event>,
        url_path: &Path,
        hash: &Checksum,
    ) -> Result<(), SingleDownloadError> {
        let mut f = fs::File::open(url_path)
            .await
            .map_err(SingleDownloadError::Open)?;

        // The checksum helper advances the global bar while verifying; on
        // failure the reporter's `Drop` undoes those bytes (and clears the
        // per-file bar), on success `finish()` keeps them.
        let mut progress = ProgressReporter::new(tx, self.download_list_index, self.total);
        let result = verify::checksum(&progress, &mut f, &mut hash.get_validator()).await;

        if !result.matches {
            progress.set_position(result.bytes);
            return Err(SingleDownloadError::ChecksumMismatch);
        }

        progress.finish();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;
    use crate::{
        CompressType, DownloadEntry, DownloadSourceType,
        download::{DownloadResult, test_support},
    };

    /// Build an entry pointing at a freshly-written `file://` source.
    fn local_entry(
        dir: &Path,
        data: &[u8],
        as_symlink: bool,
    ) -> (DownloadEntry, std::path::PathBuf) {
        let src = dir.join("source.bin");
        std::fs::write(&src, data).unwrap();

        let entry = DownloadEntry {
            source: vec![DownloadSource {
                url: format!("file://{}", src.display()),
                source_type: DownloadSourceType::Local(as_symlink),
            }],
            filename: "out.bin".to_string(),
            dir: dir.join("out"),
            ..Default::default()
        };
        (entry, src)
    }

    #[tokio::test]
    async fn copies_local_source_with_checksum() {
        let dir = TempDir::new("local-copy");
        let data = b"local copy payload";
        let (mut entry, src) = local_entry(dir.path(), data, false);
        entry.hash = Some(Checksum::from_file_sha256(&src).unwrap());

        let (tx, _rx) = flume::unbounded::<Event>();
        let result = test_support::downloader(entry).try_download(&tx).await;

        match result {
            DownloadResult::Success(summary) => {
                assert!(summary.wrote);
                assert_eq!(
                    tokio::fs::read(dir.path().join("out/out.bin"))
                        .await
                        .unwrap(),
                    data
                );
            }
            DownloadResult::Failed { file_name } => {
                panic!("expected success, got failed: {file_name}")
            }
        }
    }

    #[tokio::test]
    async fn symlinks_local_source() {
        let dir = TempDir::new("local-link");
        let data = b"symlink payload";
        let (entry, src) = local_entry(dir.path(), data, true);

        let (tx, _rx) = flume::unbounded::<Event>();
        let result = test_support::downloader(entry).try_download(&tx).await;
        assert!(matches!(result, DownloadResult::Success(_)));

        let link = dir.path().join("out/out.bin");
        let meta = std::fs::symlink_metadata(&link).unwrap();
        assert!(meta.file_type().is_symlink());
        assert_eq!(std::fs::read_link(&link).unwrap(), src);
    }

    #[tokio::test]
    async fn fails_on_checksum_mismatch() {
        let dir = TempDir::new("local-mismatch");
        let data = b"payload";
        let (mut entry, _src) = local_entry(dir.path(), data, false);
        entry.hash = Some(Checksum::Sha256(vec![0; 32])); // wrong on purpose

        let (tx, _rx) = flume::unbounded::<Event>();
        let result = test_support::downloader(entry).try_download(&tx).await;
        assert!(matches!(result, DownloadResult::Failed { .. }));
    }

    #[tokio::test]
    async fn moves_completed_file_to_final_dir() {
        let dir = TempDir::new("local-final");
        let data = b"final payload";
        let (mut entry, src) = local_entry(dir.path(), data, false);
        entry.hash = Some(Checksum::from_file_sha256(&src).unwrap());
        entry.final_dir = Some(dir.path().join("final"));

        let (tx, _rx) = flume::unbounded::<Event>();
        let result = test_support::downloader(entry).try_download(&tx).await;
        assert!(matches!(result, DownloadResult::Success(_)));

        let final_path = dir.path().join("final/out.bin");
        assert_eq!(tokio::fs::read(&final_path).await.unwrap(), data);
        assert!(!dir.path().join("out/out.bin").exists());
    }

    #[tokio::test]
    async fn copies_compressed_local_source() {
        let dir = TempDir::new("local-gzip");
        let data: &[u8] = b"compressed local payload";

        // a gzip-compressed version of `data` (fixed bytes so the test needs
        // no encoder dependency)
        let compressed: &[u8] = &[
            31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 75, 206, 207, 45, 40, 74, 45, 46, 78, 77, 81, 200,
            201, 79, 78, 204, 81, 40, 72, 172, 204, 201, 79, 76, 1, 0, 230, 247, 37, 109, 24, 0, 0,
            0,
        ];

        let src = dir.path().join("source.bin.gz");
        std::fs::write(&src, compressed).unwrap();

        let entry = DownloadEntry {
            source: vec![DownloadSource {
                url: format!("file://{}", src.display()),
                source_type: DownloadSourceType::Local(false),
            }],
            filename: "out.bin".to_string(),
            dir: dir.path().join("out"),
            file_type: CompressType::Gzip,
            ..Default::default()
        };

        let (tx, _rx) = flume::unbounded::<Event>();
        let result = test_support::downloader(entry).try_download(&tx).await;
        assert!(matches!(result, DownloadResult::Success(_)));
        assert_eq!(
            tokio::fs::read(dir.path().join("out/out.bin"))
                .await
                .unwrap(),
            data
        );
    }

    #[tokio::test]
    async fn fails_symlink_on_checksum_mismatch_and_undoes_global() {
        let dir = TempDir::new("local-link-mismatch");
        let data = b"payload";
        let (mut entry, _src) = local_entry(dir.path(), data, true); // symlink path
        entry.hash = Some(Checksum::Sha256(vec![0; 32])); // wrong on purpose

        let (tx, rx) = flume::unbounded::<Event>();
        let result = test_support::downloader(entry).try_download(&tx).await;
        assert!(matches!(result, DownloadResult::Failed { .. }));

        // the checksummed bytes were undone from the global progress bar
        let events: Vec<_> = rx.drain().collect();
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Cleared { sub, .. } if *sub == data.len() as u64
        )));
    }
}
