use std::time::Duration;

use prost::Message;
use tak_proto::NodeInfo;
use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};
use tokio::time::{sleep, timeout};

use super::probe_once;

#[tokio::test]
async fn node_probe_reads_a_complete_http_body_without_waiting_for_eof() {
    let (client, mut server) = duplex(1024);
    let expected = NodeInfo {
        node_id: "builder-a".into(),
        display_name: "builder-a".into(),
        base_url: "http://builder-a.onion".into(),
        healthy: true,
        pools: vec!["default".into()],
        tags: vec!["builder".into()],
        capabilities: vec!["linux".into()],
        transport: "tor".into(),
        transport_state: "ready".into(),
        transport_detail: String::new(),
    };
    let body = expected.encode_to_vec();

    let server_task = tokio::spawn(async move {
        let mut request = [0_u8; 256];
        let _ = server.read(&mut request).await.expect("read request");
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        server.write_all(head.as_bytes()).await.expect("write head");
        server.write_all(&body).await.expect("write body");
        server.flush().await.expect("flush body");
        sleep(Duration::from_millis(200)).await;
    });

    let node = timeout(
        Duration::from_millis(50),
        probe_once(
            Box::new(client),
            "builder-a.onion:80",
            "secret",
            "http://builder-a.onion",
        ),
    )
    .await
    .expect("probe should finish from Content-Length alone")
    .unwrap_or_else(|_| panic!("probe should decode node info"));
    assert_eq!(node.node_id, expected.node_id);
    server_task.await.expect("server task");
}
