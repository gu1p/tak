use tak_core::remote_inventory::{RemoteInventory, RemoteRecord};
use tak_proto::worker_v2::{WorkerResources, WorkerSnapshot};
use takd::PeerManager;

pub fn peer_manager(base_url: &str) -> PeerManager {
    let peers = restarted_peer_manager(base_url);
    peers.mark_worker_snapshot("worker-a", snapshot("worker-a"));
    peers
}

pub fn restarted_peer_manager(base_url: &str) -> PeerManager {
    let peers = PeerManager::default();
    peers.apply_inventory(RemoteInventory {
        version: 1,
        remotes: ["worker-a", "worker-b"]
            .into_iter()
            .map(|node_id| record(node_id, base_url))
            .collect(),
    });
    peers
}

pub fn snapshot(node_id: &str) -> WorkerSnapshot {
    WorkerSnapshot {
        protocol_version: 2,
        node_id: node_id.into(),
        healthy: true,
        sampled_at_ms: 1,
        capacity: resources(8),
        usage: resources(0),
        queue_depth: 0,
        cached_content: vec![],
        processes: vec![],
    }
}

fn record(node_id: &str, base_url: &str) -> RemoteRecord {
    RemoteRecord {
        node_id: node_id.into(),
        display_name: node_id.into(),
        base_url: base_url.into(),
        bearer_token: "secret".into(),
        pools: vec![],
        tags: vec![],
        capabilities: vec![],
        transport: "direct".into(),
        enabled: true,
    }
}

fn resources(execution_slots: u32) -> WorkerResources {
    WorkerResources {
        cpu_millis: 8_000,
        memory_bytes: 16 * 1024 * 1024 * 1024,
        execution_slots,
    }
}
