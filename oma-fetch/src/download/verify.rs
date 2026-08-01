//! Checksum verification helper shared by the HTTP and local download paths.

use spdlog::debug;
use tokio::{
    fs::File,
    io::{AsyncBufReadExt as _, BufReader},
};

use crate::checksum::ChecksumValidator;

use super::{READ_FILE_BUFSIZE, progress::ProgressReporter};

/// Outcome of [`checksum`]: how many bytes were read and whether they matched
/// the expected checksum.
pub(super) struct ChecksumResult {
    /// Bytes read while verifying.
    pub bytes: u64,
    /// Whether the content matched the expected checksum.
    pub matches: bool,
}

/// Stream a whole file through a [`ChecksumValidator`], advancing the
/// per-file progress bar for each chunk (consumers advance the global bar
/// from it).
pub(super) async fn checksum(
    progress: &ProgressReporter,
    f: &mut File,
    v: &mut ChecksumValidator,
) -> ChecksumResult {
    let mut reader = BufReader::with_capacity(READ_FILE_BUFSIZE, f);

    let mut read = 0;

    loop {
        let buffer = match reader.fill_buf().await {
            Ok([]) => break,
            Ok(buffer) => buffer,
            Err(e) => {
                debug!("Error while reading file: {e}");
                break;
            }
        };

        v.update(buffer);

        progress.advance(buffer.len() as u64);
        read += buffer.len() as u64;
        let len = buffer.len();

        reader.consume(len);
    }

    ChecksumResult {
        bytes: read,
        matches: v.finish(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Event, checksum::Checksum, test_support::TempDir};

    #[tokio::test]
    async fn verifies_matching_checksum() {
        let dir = TempDir::new("verify");
        let path = dir.path().join("f.bin");
        let data = b"checksum verification";
        tokio::fs::write(&path, data).await.unwrap();

        let expected = Checksum::from_file_sha256(&path).unwrap();
        let mut validator = expected.get_validator();
        let mut file = File::open(&path).await.unwrap();
        let (tx, _rx) = flume::unbounded::<Event>();
        let progress = ProgressReporter::new(&tx, 0, 1);

        let result = checksum(&progress, &mut file, &mut validator).await;
        assert_eq!(result.bytes, data.len() as u64);
        assert!(result.matches);
    }

    #[tokio::test]
    async fn detects_mismatched_checksum() {
        let dir = TempDir::new("verify-mismatch");
        let path = dir.path().join("f.bin");
        tokio::fs::write(&path, b"data").await.unwrap();

        let mut validator = Checksum::Sha256(vec![0; 32]).get_validator();
        let mut file = File::open(&path).await.unwrap();
        let (tx, _rx) = flume::unbounded::<Event>();
        let progress = ProgressReporter::new(&tx, 0, 1);

        let result = checksum(&progress, &mut file, &mut validator).await;
        assert!(!result.matches);
    }
}
