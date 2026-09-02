use std::net::SocketAddr;

use tak_core::remote_inventory::{RemoteInventory, RemoteRecord};
use tak_proto::worker_v2::{WorkerResources, WorkerSnapshot};
use takd::PeerManager;

use super::super::worker_http::RunningServer;

pub struct WorkerSpec {
    pub node_id: String,
    pub endpoint: String,
    pub transport: String,
    slots: u32,
}

impl WorkerSpec {
    pub fn direct(node_id: &str, address: SocketAddr, slots: u32) -> Self {
        Self::new(node_id, format!("http://{address}"), "direct", slots)
    }

    pub fn tor(node_id: &str, endpoint: &str, slots: u32) -> Self {
        Self::new(node_id, endpoint.to_owned(), "tor", slots)
    }

    fn new(node_id: &str, endpoint: String, transport: &str, slots: u32) -> Self {
        Self {
            node_id: node_id.to_owned(),
            endpoint,
            transport: transport.to_owned(),
            slots,
        }
    }
}

pub fn peers(workers: &[WorkerSpec]) -> PeerManager {
    let peers = PeerManager::default();
    peers.apply_inventory(RemoteInventory {
        version: 1,
        remotes: workers.iter().map(remote_record).collect(),
    });
    for worker in workers {
        mark_snapshot(&peers, &worker.node_id, worker.slots, 0);
    }
    peers
}

pub fn mark_snapshot(peers: &PeerManager, node_id: &str, capacity: u32, used: u32) {
    peers.mark_worker_snapshot(
        node_id,
        WorkerSnapshot {
            protocol_version: 2,
            node_id: node_id.to_owned(),
            healthy: true,
            sampled_at_ms: 1,
            capacity: resources(capacity),
            usage: resources(used),
            queue_depth: 0,
            cached_content: vec![],
            processes: vec![],
        },
    );
}

pub fn attempt_count(worker: &RunningServer) -> u64 {
    let connection = rusqlite::Connection::open(worker.state_root.join("takd.sqlite")).unwrap();
    let count = connection
        .query_row("SELECT COUNT(*) FROM worker_v2_attempts", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap();
    u64::try_from(count).unwrap()
}

fn remote_record(worker: &WorkerSpec) -> RemoteRecord {
    RemoteRecord {
        node_id: worker.node_id.clone(),
        display_name: worker.node_id.clone(),
        base_url: worker.endpoint.clone(),
        bearer_token: "secret".into(),
        pools: vec![],
        tags: vec![],
        capabilities: vec![],
        transport: worker.transport.clone(),
        enabled: true,
    }
}

fn resources(slots: u32) -> WorkerResources {
    WorkerResources {
        cpu_millis: 8_000,
        memory_bytes: 16_000,
        execution_slots: slots,
    }
}
