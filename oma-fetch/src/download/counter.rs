//! An [`AsyncRead`] wrapper that counts bytes read, used by the HTTP body
//! stream to track how many (possibly compressed) bytes the server sent.

use std::{
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll},
};

use futures::{AsyncBufRead, AsyncRead};

pub(crate) struct Counter<'a, D> {
    inner: D,
    bytes: &'a AtomicUsize,
}

impl<'a, D> Counter<'a, D> {
    #[inline]
    pub(crate) const fn new(inner: D, bytes: &'a AtomicUsize) -> Self {
        Self { inner, bytes }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for Counter<'_, R> {
    fn poll_read(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let counter = self.get_mut();
        let pin = Pin::new(&mut counter.inner);

        let poll = pin.poll_read(ctx, buf);
        if let Poll::Ready(Ok(bytes)) = poll {
            counter.bytes.fetch_add(bytes, Ordering::AcqRel);
        }

        poll
    }
}

impl<R: AsyncBufRead + Unpin> AsyncBufRead for Counter<'_, R> {
    fn poll_fill_buf(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        let counter = self.get_mut();
        let pin = Pin::new(&mut counter.inner);
        pin.poll_fill_buf(ctx)
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        let counter = self.get_mut();
        counter.bytes.fetch_add(amt, Ordering::AcqRel);
        let pin = Pin::new(&mut counter.inner);
        pin.consume(amt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{AsyncReadExt as _, io::Cursor};

    #[tokio::test]
    async fn counts_every_byte_read() {
        let data = b"hello world".to_vec();
        let counter = AtomicUsize::new(0);
        let mut reader = Counter::new(Cursor::new(data.clone()), &counter);

        let mut buf = vec![0u8; 4];
        let mut read = 0;
        loop {
            let n = reader.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            read += n;
        }
        assert_eq!(read, data.len());
        assert_eq!(counter.load(Ordering::SeqCst), data.len());
    }

    #[tokio::test]
    async fn stops_counting_after_eof() {
        let data = b"abc".to_vec();
        let counter = AtomicUsize::new(0);
        let mut reader = Counter::new(Cursor::new(data), &counter);

        let mut buf = vec![0u8; 1024];
        assert_eq!(reader.read(&mut buf).await.unwrap(), 3);
        assert_eq!(reader.read(&mut buf).await.unwrap(), 0);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
}
