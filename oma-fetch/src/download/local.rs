//! Local (`file://`) source download implementation for
//! [`SingleDownloader`](super::SingleDownloader): symlinking, copying and
//! checksum verification of already-present files.

use std::path::Path;

use async_compression::futures::bufread::{
    BzDecoder, GzipDecoder, Lz4Decoder, LzmaDecoder, XzDecoder, ZstdDecoder,
};
use flume::Sender;
use futures::{AsyncRead, io::BufReader};
use spdlog::{debug, trace};
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt as _, AsyncWriteExt},
};
use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};

use crate::{
    CompressType, DownloadSource, Event,
    checksum::{Checksum, ChecksumValidator},
};

use super::{
    READ_FILE_BUFSIZE, SingleDownloader, error::SingleDownloadError, progress::ProgressReporter,
    verify,
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
            .map_err(|source| SingleDownloadError::Open { source })?
            .len();

        // A stale symlink, or an existing file when linking, must not stay in
        // the way of the destination.
        let file = self.entry.dir.join(&*self.entry.filename);
        if file.is_symlink() || (as_symlink && file.is_file()) {
            fs::remove_file(&file)
                .await
                .map_err(|source| SingleDownloadError::Remove { source })?;
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
            .map_err(|source| SingleDownloadError::CreateSymlink { source })?;

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
        progress.bar(&self.download_message(), total_size);

        trace!("Path for file: {}", url_path.display());

        let from = File::open(url_path)
            .await
            .map_err(|source| SingleDownloadError::Open { source })?;
        let from = tokio::io::BufReader::new(from).compat();

        trace!("Successfully opened file: {}", url_path.display());

        let mut to = File::create(self.entry.dir.join(&*self.entry.filename))
            .await
            .map_err(|source| SingleDownloadError::Create { source })?;

        let reader: &mut (dyn AsyncRead + Unpin + Send) = match self.entry.file_type {
            CompressType::Xz => &mut XzDecoder::new(BufReader::new(from)),
            CompressType::Gzip => &mut GzipDecoder::new(BufReader::new(from)),
            CompressType::Bz2 => &mut BzDecoder::new(BufReader::new(from)),
            CompressType::Zstd => &mut ZstdDecoder::new(BufReader::new(from)),
            CompressType::Lzma => &mut LzmaDecoder::new(BufReader::new(from)),
            CompressType::Lz4 => &mut Lz4Decoder::new(BufReader::new(from)),
            CompressType::None => &mut BufReader::new(from),
        };

        let mut reader = reader.compat();

        trace!(
            "Successfully created file: {}",
            self.entry.dir.join(&*self.entry.filename).display()
        );

        // On failure, undo the bytes this copy reported so a fallback starts
        // from a clean bar; the per-file bar is cleared when `progress` drops.
        match self
            .download_local_copy(&mut to, &mut reader, &mut progress)
            .await
        {
            Ok(()) => Ok(()),
            Err(error) => {
                if progress.reported() != 0 {
                    progress.sub(progress.reported());
                }
                Err(error)
            }
        }
    }

    /// Copy loop for a local source. Progress is reported on success; on
    /// failure the plain error is returned and [`download_local`] cleans up
    /// the reported bytes.
    async fn download_local_copy<R>(
        &self,
        to: &mut File,
        reader: &mut R,
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
                Err(e) => return Err(SingleDownloadError::Read { source: e }),
            };

            if size == 0 {
                break;
            }

            to.write_all(&buf[..size])
                .await
                .map_err(|source| SingleDownloadError::Write { source })?;

            self_progress += size;

            progress.inc(size as u64);
            v.update(&buf[..size]);
            progress.add(size as u64);
            progress.set_reported(self_progress as u64);
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
            .map_err(|source| SingleDownloadError::Open { source })?;
        let (size, finish) = verify::checksum(tx, &mut f, &mut hash.get_validator()).await;

        if !finish {
            let _ = tx.send(Event::GlobalProgressSub(size));
            let _ = tx.send(Event::ProgressDone(self.download_list_index));
            return Err(SingleDownloadError::ChecksumMismatch);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;
    use crate::{
        DownloadEntry, DownloadSourceType,
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
}
