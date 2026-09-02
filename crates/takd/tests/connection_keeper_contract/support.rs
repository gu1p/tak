use std::time::Duration;

use prost::Message;
use tak_core::remote_inventory::{RemoteInventory, RemoteRecord};
use tak_proto::NodePingResponse;
use takd::PeerManager;

use crate::support::http2_remote::Http2Remote;

pub(super) const NODE: &str = "builder-keeper";
pub(super) const ENDPOINT: &str = "http://builder-keeper.onion";

pub(super) fn peers() -> PeerManager {
    crate::support::local_runtime::peer_manager(RemoteInventory {
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

pub(super) async fn wait_for_connections(remote: &Http2Remote, want: usize) {
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

pub(super) fn ping_body() -> Vec<u8> {
    let payload = NodePingResponse {
        node_id: NODE.into(),
        protocol_version: "v2".into(),
        health: "healthy".into(),
        active_job_count: 0,
        queue_depth: 0,
        resource_summary: "cpu_available=8.00 memory_available_mb=16384".into(),
    }
    .encode_to_vec();
    tak_proto::worker_v2::encode_display_payload(&payload).unwrap()
}
