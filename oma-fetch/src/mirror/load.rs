use std::path::Path;

use reqwest::{Method, Url};
use reqwest_middleware::ClientWithMiddleware;

use crate::send_request_with_url_and_method;

use super::error::MirrorError;
use super::uri::MirrorListLocation;

/// Maximum accepted size of a mirror list, matching apt's `MaximumSize`.
pub const MAX_MIRROR_LIST_SIZE: u64 = 1024 * 1024;

pub(super) fn compressed_extension(location: &MirrorListLocation) -> Option<String> {
    match location {
        MirrorListLocation::Http(url) => {
            let parsed = Url::parse(url).ok()?;
            compressed_suffix(Path::new(parsed.path()).extension()?.to_str()?)
        }
        MirrorListLocation::File(path) => compressed_suffix(path.extension()?.to_str()?),
    }
}

fn compressed_suffix(extension: &str) -> Option<String> {
    ["gz", "xz", "bz2", "zst", "zstd", "lz4", "lzma"]
        .iter()
        .find(|known| extension.eq_ignore_ascii_case(known))
        .map(|known| (*known).to_string())
}

pub(super) async fn fetch_text(
    uri: &str,
    url: &str,
    client: &ClientWithMiddleware,
) -> Result<String, MirrorError> {
    let mut resp = send_request_with_url_and_method(url, client, Method::GET)
        .await
        .map_err(|source| MirrorError::Fetch {
            url: url.to_string(),
            source,
        })?;

    if let Some(len) = resp.content_length()
        && len > MAX_MIRROR_LIST_SIZE
    {
        return Err(MirrorError::TooLarge {
            uri: uri.to_string(),
            limit: MAX_MIRROR_LIST_SIZE,
        });
    }

    let mut body = Vec::new();

    while let Some(chunk) = resp.chunk().await.map_err(|source| MirrorError::ReadBody {
        url: url.to_string(),
        source,
    })? {
        if body.len() as u64 + chunk.len() as u64 > MAX_MIRROR_LIST_SIZE {
            return Err(MirrorError::TooLarge {
                uri: uri.to_string(),
                limit: MAX_MIRROR_LIST_SIZE,
            });
        }

        body.extend_from_slice(&chunk);
    }

    String::from_utf8(body).map_err(|_| MirrorError::InvalidEncoding {
        uri: uri.to_string(),
    })
}

pub(super) async fn read_text(uri: &str, path: &Path) -> Result<String, MirrorError> {
    let metadata = tokio::fs::metadata(path)
        .await
        .map_err(|source| MirrorError::ReadFile {
            path: path.to_path_buf(),
            source,
        })?;

    if metadata.len() > MAX_MIRROR_LIST_SIZE {
        return Err(MirrorError::TooLarge {
            uri: uri.to_string(),
            limit: MAX_MIRROR_LIST_SIZE,
        });
    }

    let bytes = tokio::fs::read(path)
        .await
        .map_err(|source| MirrorError::ReadFile {
            path: path.to_path_buf(),
            source,
        })?;

    if bytes.len() as u64 > MAX_MIRROR_LIST_SIZE {
        return Err(MirrorError::TooLarge {
            uri: uri.to_string(),
            limit: MAX_MIRROR_LIST_SIZE,
        });
    }

    String::from_utf8(bytes).map_err(|_| MirrorError::InvalidEncoding {
        uri: uri.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use reqwest::ClientBuilder;
    use reqwest_middleware::ClientWithMiddleware;

    use super::*;
    use crate::mirror::MirrorList;

    fn client() -> ClientWithMiddleware {
        rustls::crypto::ring::default_provider()
            .install_default()
            .ok();

        ClientBuilder::new().build().unwrap().into()
    }

    fn temp_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        std::env::temp_dir().join(format!(
            "oma-fetch-mirror-{}-{stamp}-{name}",
            std::process::id()
        ))
    }

    #[tokio::test]
    async fn loads_a_local_list() {
        let path = temp_path("mirrors.list");
        tokio::fs::write(&path, "http://m1.example.com/\nfile:/srv/repo\n")
            .await
            .unwrap();
        let uri = format!("mirror+file:{}", path.display());

        let list = MirrorList::load(&uri, &client()).await.unwrap();

        assert_eq!(list.uri(), uri);
        assert_eq!(list.len(), 2);

        let _ = tokio::fs::remove_file(&path).await;
    }

    #[tokio::test]
    async fn loads_a_local_list_with_trailing_slash() {
        let path = temp_path("trailing.list");
        tokio::fs::write(&path, "http://m1.example.com/\n")
            .await
            .unwrap();
        let uri = format!("mirror+file:{}/", path.display());

        let list = MirrorList::load(&uri, &client()).await.unwrap();

        assert_eq!(list.len(), 1);

        let _ = tokio::fs::remove_file(&path).await;
    }

    #[tokio::test]
    async fn rejects_compressed_local_lists() {
        let path = temp_path("mirrors.list.gz");
        let uri = format!("mirror+file:{}", path.display());

        let err = MirrorList::load(&uri, &client()).await.unwrap_err();

        assert!(matches!(err, MirrorError::UnsupportedCompression { .. }));
    }

    #[tokio::test]
    async fn rejects_oversized_local_lists() {
        let path = temp_path("big.list");
        tokio::fs::write(&path, vec![b'#'; (MAX_MIRROR_LIST_SIZE + 1) as usize])
            .await
            .unwrap();
        let uri = format!("mirror+file:{}", path.display());

        let err = MirrorList::load(&uri, &client()).await.unwrap_err();

        assert!(matches!(err, MirrorError::TooLarge { .. }));

        let _ = tokio::fs::remove_file(&path).await;
    }

    #[tokio::test]
    async fn rejects_non_utf8_local_lists() {
        let path = temp_path("invalid.list");
        tokio::fs::write(&path, [0xff, 0xfe]).await.unwrap();
        let uri = format!("mirror+file:{}", path.display());

        let err = MirrorList::load(&uri, &client()).await.unwrap_err();

        assert!(matches!(err, MirrorError::InvalidEncoding { .. }));

        let _ = tokio::fs::remove_file(&path).await;
    }
}
