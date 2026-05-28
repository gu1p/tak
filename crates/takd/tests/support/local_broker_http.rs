#![allow(dead_code)]

use std::path::Path;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

pub async fn send_broker_get(socket_path: &Path, node_id: &str) -> Vec<u8> {
    let request = format!(
        "GET /v1/node/info HTTP/1.1\r\nHost: {node_id}.onion\r\nAuthorization: Bearer secret\r\nX-Tak-Broker-Version: 1\r\nX-Tak-Remote-Node: {node_id}\r\nX-Tak-Remote-Endpoint: http://{node_id}.onion\r\nX-Tak-Remote-Transport: tor\r\nConnection: close\r\n\r\n"
    );
    send_raw_http(socket_path, request.as_bytes()).await
}

pub async fn send_broker_get_h2(socket_path: &Path, node_id: &str) -> Vec<u8> {
    let request = format!(
        "GET /v1/node/info HTTP/1.1\r\nHost: {node_id}.onion\r\nAuthorization: Bearer secret\r\nX-Tak-Broker-Version: 1\r\nX-Tak-Remote-Node: {node_id}\r\nX-Tak-Remote-Endpoint: http://{node_id}.onion\r\nX-Tak-Remote-Protocol: h2\r\nX-Tak-Remote-Transport: tor\r\nConnection: close\r\n\r\n"
    );
    send_raw_http(socket_path, request.as_bytes()).await
}

pub async fn send_raw_http(socket_path: &Path, request: &[u8]) -> Vec<u8> {
    for _ in 0..50 {
        if let Ok(mut stream) = UnixStream::connect(socket_path).await {
            stream.write_all(request).await.expect("write request");
            stream.shutdown().await.expect("shutdown write");
            let mut response = Vec::new();
            stream
                .read_to_end(&mut response)
                .await
                .expect("read response");
            return response;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("timed out connecting to socket {}", socket_path.display());
}

pub fn response_body(response: &[u8]) -> &[u8] {
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
        .expect("response header split");
    &response[split..]
}
