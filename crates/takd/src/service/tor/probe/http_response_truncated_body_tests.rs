use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};

use super::probe_node_info;

#[tokio::test]
async fn startup_probe_rejects_truncated_bodies() {
    let (client, mut server) = duplex(256);
    let server_task = tokio::spawn(async move {
        let mut request = [0_u8; 256];
        let _ = server.read(&mut request).await.expect("read request");
        server
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nabc")
            .await
            .expect("write truncated response");
        server.flush().await.expect("flush truncated response");
    });

    let err = probe_node_info(
        Box::new(client),
        "builder-a.onion:80",
        "secret",
        "http://builder-a.onion",
    )
    .await
    .expect_err("truncated body should fail");
    assert!(format!("{err:#}").contains("truncated HTTP response body"));
    server_task.await.expect("server task");
}
