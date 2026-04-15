#![cfg(test)]

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

use super::probe_once;

#[tokio::test]
async fn node_probe_timeout_closes_the_connection() {
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
                .starts_with("GET /v1/node/info HTTP/1.1\r\n"),
            "unexpected request:\n{}",
            String::from_utf8_lossy(&request[..header_end])
        );
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhel",
            )
            .await
            .expect("write partial response");
        stream.flush().await.expect("flush partial response");

        let mut eof = [0_u8; 1];
        timeout(Duration::from_millis(500), async {
            loop {
                if stream.read(&mut eof).await.expect("read eof") == 0 {
                    break;
                }
            }
        })
        .await
        .expect("timed out node probe should close the connection");
    });

    assert!(
        timeout(
            Duration::from_millis(50),
            probe_once(
                Box::new(TcpStream::connect(addr).await.expect("connect client")),
                "builder-a.onion:80",
                "secret",
                "http://builder-a.onion",
            ),
        )
        .await
        .is_err(),
        "probe should time out"
    );

    server_task.await.expect("server task");
}
