//! Checksum verification helper shared by the HTTP and local download paths.

use flume::Sender;
use spdlog::debug;
use tokio::{
    fs::File,
    io::{AsyncBufReadExt as _, BufReader},
};

use crate::{Event, checksum::ChecksumValidator};

use super::READ_FILE_BUFSIZE;

/// Stream a whole file through a [`ChecksumValidator`], reporting each chunk
/// to the global progress bar. Returns `(bytes_read, checksum_matches)`.
pub(super) async fn checksum(
    tx: &Sender<Event>,
    f: &mut File,
    v: &mut ChecksumValidator,
) -> (u64, bool) {
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

        let _ = tx.send(Event::GlobalProgressAdd(buffer.len() as u64));
        read += buffer.len() as u64;
        let len = buffer.len();

        reader.consume(len);
    }

    (read, v.finish())
}
