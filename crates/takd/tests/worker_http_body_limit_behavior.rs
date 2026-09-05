use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::support::worker_http::start_server;

#[tokio::test]
async fn worker_rejects_oversized_upload_from_headers_without_reading_body() {
    let server = start_server().await;
    let mut stream = tokio::net::TcpStream::connect(server.addr).await.unwrap();
    stream
        .write_all(
            concat!(
                "POST /v2/attempts/dispatch HTTP/1.1\r\nHost: localhost\r\n",
                "Authorization: Bearer secret\r\nContent-Length: 536870913\r\n\r\n"
            )
            .as_bytes(),
        )
        .await
        .unwrap();
    let mut response = Vec::new();
    tokio::time::timeout(Duration::from_secs(2), stream.read_to_end(&mut response))
        .await
        .expect("oversized requests must fail before allocating or reading their bodies")
        .unwrap();
    assert!(response.starts_with(b"HTTP/1.1 413 Payload Too Large\r\n"));
}
