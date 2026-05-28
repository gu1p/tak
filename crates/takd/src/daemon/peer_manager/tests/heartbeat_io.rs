use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::super::heartbeat::{ping_peer, should_ping, unix_epoch_ms};
use super::super::{PeerEligibility, PeerManager, PeerState};
use super::support::{
    encoded_ping_body, inventory, read_http_request, record, request_contains_bearer_secret,
};
use crate::daemon::protocol::TorBroker;

#[tokio::test(flavor = "multi_thread")]
async fn heartbeat_uses_inventory_bearer_token_for_ping() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind ping");
    let addr = listener.local_addr().expect("listener addr");
    let server = tokio::spawn(async move {
        accept_and_close_http2_probes(&listener).await;
        let (mut stream, _) = listener.accept().await.expect("accept ping");
        let request = read_http_request(&mut stream).await;
        let authorized = request_contains_bearer_secret(&request);
        let body = if authorized {
            encoded_ping_body()
        } else {
            Vec::new()
        };
        let status = if authorized {
            "200 OK"
        } else {
            "401 Unauthorized"
        };
        stream
            .write_all(
                format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .as_bytes(),
            )
            .await
            .expect("write ping response head");
        stream
            .write_all(&body)
            .await
            .expect("write ping response body");
        request
    });
    let manager =
        PeerManager::from_inventory(inventory(vec![record("builder-a", "tor", true, "secret")]));
    let target = manager
        .heartbeat_targets_due(unix_epoch_ms())
        .pop()
        .unwrap();
    let broker = TorBroker::for_test_dial_addr(addr.to_string());

    ping_peer(&manager, &broker, &target).await;

    let request = server.await.expect("ping server exits");
    assert!(request_contains_bearer_secret(&request));
    assert_eq!(manager.snapshots()[0].state, PeerState::Connected);
}

#[tokio::test(flavor = "multi_thread")]
async fn heartbeat_marks_protocol_mismatch_for_unsupported_ping() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind ping");
    let addr = listener.local_addr().expect("listener addr");
    let server = tokio::spawn(async move {
        accept_and_close_http2_probes(&listener).await;
        let (mut stream, _) = listener.accept().await.expect("accept ping");
        let _request = read_http_request(&mut stream).await;
        stream
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await
            .expect("write ping response");
    });
    let manager =
        PeerManager::from_inventory(inventory(vec![record("builder-a", "tor", true, "secret")]));
    let target = manager
        .heartbeat_targets_due(unix_epoch_ms())
        .pop()
        .unwrap();
    let broker = TorBroker::for_test_dial_addr(addr.to_string());

    ping_peer(&manager, &broker, &target).await;
    server.await.expect("ping server exits");

    let snapshot = manager.snapshots().pop().expect("snapshot");
    assert_eq!(snapshot.state, PeerState::ProtocolMismatch);
    assert!(should_ping(snapshot.state));
    assert!(manager.eligible(&PeerEligibility::default()).is_empty());
}

async fn accept_and_close_http2_probes(listener: &TcpListener) {
    for _ in 0..2 {
        let (mut stream, _) = listener.accept().await.expect("accept h2 probe");
        let mut buffer = [0_u8; 64];
        let _ = stream.read(&mut buffer).await;
    }
}
