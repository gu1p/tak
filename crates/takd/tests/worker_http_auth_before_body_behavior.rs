use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::support::worker_http::start_server;

#[tokio::test]
async fn worker_rejects_unauthenticated_upload_without_waiting_for_body() {
    let server = start_server().await;
    for authorization in ["", "Authorization: Bearer wrong\r\n"] {
        let mut stream = tokio::net::TcpStream::connect(server.addr).await.unwrap();
        let headers = format!(
            "POST /v2/attempts/dispatch HTTP/1.1\r\nHost: localhost\r\n{authorization}Content-Length: 1024\r\n\r\n"
        );
        stream.write_all(headers.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_secs(2), stream.read_to_end(&mut response))
            .await
            .expect("authentication must reject before the peer sends its body")
            .unwrap();
        assert!(response.starts_with(b"HTTP/1.1 401 Unauthorized\r\n"));
    }
}
