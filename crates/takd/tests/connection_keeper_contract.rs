use std::time::Duration;

use prost::Message;
use tak_core::remote_inventory::{RemoteInventory, RemoteRecord};
use tak_proto::NodePingResponse;
use takd::{PeerManager, TorBroker};

use crate::support;
use support::http2_remote::Http2Remote;

const NODE: &str = "builder-keeper";
const ENDPOINT: &str = "http://builder-keeper.onion";

#[tokio::test(flavor = "multi_thread")]
async fn keeper_eagerly_holds_a_warm_connection_without_any_submit() {
    let remote = Http2Remote::spawn(ping_body()).await;
    let broker = TorBroker::for_test_dial_addr(remote.addr.clone());
    // No heartbeat and no submit: the keeper alone must open the connection.
    peers().spawn_connection_keeper(broker.clone());

    wait_for_connections(&remote, 1).await;
    assert!(remote.connection_count() >= 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn keeper_redials_immediately_after_the_connection_is_lost() {
    let remote = Http2Remote::spawn(ping_body()).await;
    let broker = TorBroker::for_test_dial_addr(remote.addr.clone());
    peers().spawn_connection_keeper(broker.clone());
    wait_for_connections(&remote, 1).await;

    // Simulate a lost link by evicting the pooled session; the keeper must
    // re-establish it on its next tick. (Real silent-transport loss is detected
    // by hyper keep-alive and exercised end to end by the live-Tor example.)
    broker
        .evict_http2_session_for_peer(ENDPOINT, NODE, "secret")
        .await;

    wait_for_connections(&remote, 2).await;
    assert!(remote.connection_count() >= 2);

    // The redialed connection must actually work, not merely accept a TCP socket.
    let (status, _body) = broker
        .get_protobuf(ENDPOINT, NODE, "/v1/node/ping", "secret")
        .await
        .expect("redialed warm connection serves a request");
    assert_eq!(status, 200);
}

fn peers() -> PeerManager {
    PeerManager::from_inventory(RemoteInventory {
        version: 1,
        remotes: vec![RemoteRecord {
            node_id: NODE.into(),
            display_name: NODE.into(),
            base_url: ENDPOINT.into(),
            bearer_token: "secret".into(),
            pools: vec!["build".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "tor".into(),
            enabled: true,
        }],
    })
}

async fn wait_for_connections(remote: &Http2Remote, want: usize) {
    for _ in 0..200 {
        if remote.connection_count() >= want {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!(
        "keeper did not reach {want} connection(s); saw {}",
        remote.connection_count()
    );
}

fn ping_body() -> Vec<u8> {
    NodePingResponse {
        node_id: NODE.into(),
        protocol_version: "v1".into(),
        health: "healthy".into(),
        active_job_count: 0,
        queue_depth: 0,
        resource_summary: "cpu_available=8.00 memory_available_mb=16384".into(),
    }
    .encode_to_vec()
}
