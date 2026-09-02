use std::time::Duration;

use tak_proto::worker_v2::{WorkerIdentity, encode_identity};
use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};
use tokio::time::{sleep, timeout};

use super::probe_worker_identity;

#[tokio::test]
async fn startup_probe_reads_a_complete_http_body_without_waiting_for_eof() {
    let (client, mut server) = duplex(1024);
    let body = encode_identity(&WorkerIdentity {
        protocol_version: 2,
        node_id: "builder-a".into(),
        display_name: "builder-a".into(),
        base_url: "http://builder-a.onion".into(),
        pools: vec!["default".into()],
        tags: vec!["builder".into()],
        capabilities: vec!["linux".into()],
        transport: "tor".into(),
    })
    .unwrap();

    let server_task = tokio::spawn(async move {
        let mut request = [0_u8; 256];
        let _ = server.read(&mut request).await.expect("read request");
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        server.write_all(head.as_bytes()).await.expect("write head");
        server.write_all(&body).await.expect("write body");
        server.flush().await.expect("flush body");
        sleep(Duration::from_millis(200)).await;
    });

    timeout(
        Duration::from_millis(50),
        probe_worker_identity(
            Box::new(client),
            "builder-a.onion:80",
            "secret",
            "http://builder-a.onion",
        ),
    )
    .await
    .expect("probe should finish from Content-Length alone")
    .expect("probe should decode node info");
    server_task.await.expect("server task");
}

#[tokio::test]
async fn startup_probe_rejects_invalid_content_length() {
    let (client, mut server) = duplex(256);
    let server_task = tokio::spawn(async move {
        let mut request = [0_u8; 256];
        let _ = server.read(&mut request).await.expect("read request");
        server
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: nope\r\n\r\n")
            .await
            .expect("write invalid response");
        server.flush().await.expect("flush invalid response");
    });

    let err = probe_worker_identity(
        Box::new(client),
        "builder-a.onion:80",
        "secret",
        "http://builder-a.onion",
    )
    .await
    .expect_err("invalid content-length should fail");
    assert!(format!("{err:#}").contains("malformed HTTP response"));
    server_task.await.expect("server task");
}
