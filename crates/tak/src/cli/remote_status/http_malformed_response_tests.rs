#![cfg(test)]

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use super::fetch_status_once;

#[tokio::test]
async fn node_status_reports_malformed_http_responses_with_base_url_context() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let server_task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept request");
        let mut request = Vec::new();
        let mut chunk = [0_u8; 256];
        let header_end = loop {
            let read = stream.read(&mut chunk).await.expect("read request");
            assert!(read > 0, "client closed before sending the request");
            request.extend_from_slice(&chunk[..read]);
            if let Some(index) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                break index + 4;
            }
        };
        assert!(
            String::from_utf8_lossy(&request[..header_end])
                .starts_with("GET /v1/node/status HTTP/1.1\r\n"),
            "unexpected request:\n{}",
            String::from_utf8_lossy(&request[..header_end])
        );
        stream
            .write_all(b"not-http\r\n\r\n")
            .await
            .expect("write malformed response");
        stream.flush().await.expect("flush malformed response");
    });

    let err = fetch_status_once(
        TcpStream::connect(addr).await.expect("connect client"),
        "builder-a.onion:80",
        "secret",
        "http://builder-a.onion",
    )
    .await
    .expect_err("status request should reject malformed HTTP");
    assert!(
        err.into_anyhow()
            .to_string()
            .contains("malformed HTTP response from http://builder-a.onion"),
        "expected explicit malformed-response diagnostic",
    );

    server_task.await.expect("server task");
}
