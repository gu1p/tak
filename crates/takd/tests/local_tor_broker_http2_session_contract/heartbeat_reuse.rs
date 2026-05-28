use prost::Message;
use std::time::Duration;
use tak_core::remote_inventory::{RemoteInventory, RemoteRecord};
use tak_proto::NodePingResponse;
use takd::{PeerManager, TorBroker, new_shared_manager};

use crate::support;
use support::local_broker_http::send_broker_get_h2;

#[tokio::test(flavor = "multi_thread")]
async fn peer_heartbeat_warms_http2_session_reused_by_later_broker_request() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote = support::http2_remote::Http2Remote::spawn(node_ping_body()).await;
    let socket_path = temp.path().join("run/takd.sock");
    let server_socket_path = socket_path.clone();
    let broker = TorBroker::for_test_dial_addr(remote.addr.clone());
    let peers = PeerManager::from_inventory(RemoteInventory {
        version: 1,
        remotes: vec![RemoteRecord {
            node_id: "builder-h2".into(),
            display_name: "builder-h2".into(),
            base_url: "http://builder-h2.onion".into(),
            bearer_token: "secret".into(),
            pools: vec!["build".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "tor".into(),
            enabled: true,
        }],
    });
    peers.spawn_heartbeat_loop(broker.clone());
    let server = tokio::spawn(async move {
        takd::run_server_with_broker_and_peers(
            &server_socket_path,
            new_shared_manager(),
            broker,
            peers,
        )
        .await
    });

    wait_for_heartbeat_connection(&remote).await;
    let before = remote.connection_count();
    let _ = send_broker_get_h2(&socket_path, "builder-h2").await;

    assert_eq!(
        remote.connection_count(),
        before,
        "broker request should reuse heartbeat-warmed HTTP/2 session"
    );
    server.abort();
}

fn node_ping_body() -> Vec<u8> {
    NodePingResponse {
        node_id: "builder-h2".into(),
        protocol_version: "v1".into(),
        health: "healthy".into(),
        active_job_count: 0,
        queue_depth: 0,
        resource_summary: "cpu_available=8.00 memory_available_mb=16384".into(),
    }
    .encode_to_vec()
}

async fn wait_for_heartbeat_connection(remote: &support::http2_remote::Http2Remote) {
    for _ in 0..100 {
        if remote.connection_count() > 0 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("heartbeat did not connect to remote");
}
