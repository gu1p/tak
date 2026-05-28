use tokio::net::TcpListener;

use super::super::PeerManager;
use super::super::heartbeat::{ping_peer, unix_epoch_ms};
use super::support::{inventory, record};
use crate::daemon::protocol::TorBroker;

#[tokio::test(flavor = "multi_thread")]
async fn heartbeat_ping_has_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind ping");
    let addr = listener.local_addr().expect("listener addr");
    let server = tokio::spawn(async move {
        let (_stream, _) = listener.accept().await.expect("accept ping");
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    });
    let manager =
        PeerManager::from_inventory(inventory(vec![record("builder-a", "tor", true, "secret")]));
    let target = manager
        .heartbeat_targets_due(unix_epoch_ms())
        .pop()
        .unwrap();
    let broker = TorBroker::for_test_dial_addr(addr.to_string());
    unsafe { std::env::set_var("TAKD_PEER_HEARTBEAT_TIMEOUT_MS", "100") };

    tokio::time::timeout(
        std::time::Duration::from_secs(1),
        ping_peer(&manager, &broker, &target),
    )
    .await
    .expect("heartbeat should time out internally");

    let snapshot = manager.snapshots().pop().expect("snapshot");
    assert!(snapshot.last_error_summary.unwrap().contains("timed out"));
    unsafe { std::env::remove_var("TAKD_PEER_HEARTBEAT_TIMEOUT_MS") };
    server.abort();
}
