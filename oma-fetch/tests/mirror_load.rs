use oma_fetch::mirror::{
    MAX_MIRROR_LIST_SIZE, MirrorError, MirrorList, MirrorTarget, MirrorTransport, TargetKind,
};
use reqwest::ClientBuilder;
use reqwest_middleware::ClientWithMiddleware;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

fn client() -> ClientWithMiddleware {
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();

    ClientBuilder::new().build().unwrap().into()
}

/// Serve a single HTTP response and return the bound port.
async fn serve(status: &'static str, body: Vec<u8>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = [0u8; 4096];
            let _ = socket.read(&mut buf).await;

            let header = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = socket.write_all(header.as_bytes()).await;
            let _ = socket.write_all(&body).await;
            let _ = socket.shutdown().await;
        }
    });

    port
}

#[tokio::test]
async fn loads_a_remote_list_and_rejects_file_mirrors() {
    let body = b"http://m1.example.com/debian/\nfile:/srv/repo\n".to_vec();
    let port = serve("200 OK", body).await;
    let uri = format!("mirror+http://127.0.0.1:{port}/mirrors.list");

    let list = MirrorList::load(&uri, &client()).await.unwrap();
    assert_eq!(list.len(), 1);

    let download_url = format!("{uri}/dists/bookworm/InRelease");
    let target = MirrorTarget {
        kind: TargetKind::Index,
        ..Default::default()
    };
    let selected = list.resolve(&uri, &download_url, &target).unwrap();

    assert_eq!(selected.len(), 1);
    assert_eq!(
        selected[0].url,
        "http://m1.example.com/debian/dists/bookworm/InRelease"
    );
    assert_eq!(selected[0].transport, MirrorTransport::Http);
}

#[tokio::test]
async fn reports_http_failures() {
    let port = serve("404 Not Found", Vec::new()).await;
    let uri = format!("mirror+http://127.0.0.1:{port}/mirrors.list");

    let err = MirrorList::load(&uri, &client()).await.unwrap_err();

    assert!(matches!(err, MirrorError::Fetch { .. }));
}

#[tokio::test]
async fn rejects_non_utf8_remote_lists() {
    let port = serve("200 OK", vec![0xff, 0xfe]).await;
    let uri = format!("mirror+http://127.0.0.1:{port}/mirrors.list");

    let err = MirrorList::load(&uri, &client()).await.unwrap_err();

    assert!(matches!(err, MirrorError::InvalidEncoding { .. }));
}

#[tokio::test]
async fn rejects_oversized_remote_lists() {
    let body = vec![b'#'; (MAX_MIRROR_LIST_SIZE + 1) as usize];
    let port = serve("200 OK", body).await;
    let uri = format!("mirror+http://127.0.0.1:{port}/mirrors.list");

    let err = MirrorList::load(&uri, &client()).await.unwrap_err();

    assert!(matches!(err, MirrorError::TooLarge { .. }));
}

#[tokio::test]
async fn rejects_remote_lists_with_only_file_mirrors() {
    let port = serve("200 OK", b"file:/srv/repo\n".to_vec()).await;
    let uri = format!("mirror+http://127.0.0.1:{port}/mirrors.list");

    let err = MirrorList::load(&uri, &client()).await.unwrap_err();

    assert!(matches!(err, MirrorError::NoSupportedMirrors { .. }));
}
