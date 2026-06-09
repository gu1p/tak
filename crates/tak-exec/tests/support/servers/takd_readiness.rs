use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Blocks until the in-process takd v1 server answers `/v1/node/info` with 200, or panics
/// after a deadline. Ensures the server is ready before tests place tasks against it.
pub(super) async fn wait_for_node_info(bind_addr: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let result = tokio::time::timeout(
            Duration::from_millis(250),
            fetch_node_info_status(bind_addr),
        )
        .await;
        if matches!(result, Ok(Ok(status)) if status.starts_with("HTTP/1.1 200")) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "remote v1 test server did not become ready at {bind_addr}"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn fetch_node_info_status(bind_addr: &str) -> std::io::Result<String> {
    let mut stream = TcpStream::connect(bind_addr).await?;
    let request = format!(
        "GET /v1/node/info HTTP/1.1\r\nHost: {bind_addr}\r\nAuthorization: Bearer secret\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).await?;
    let mut response = String::new();
    stream.read_to_string(&mut response).await?;
    Ok(response.lines().next().unwrap_or_default().to_string())
}
