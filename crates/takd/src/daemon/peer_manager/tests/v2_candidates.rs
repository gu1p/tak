use tak_core::remote_inventory::{RemoteInventory, RemoteRecord};
use tak_core::v2::RemoteRequirements;
use tak_proto::worker_v2::{WorkerResources, WorkerSnapshot};

use super::super::PeerManager;

#[test]
fn candidate_snapshot_includes_only_connected_matching_protocol_v2_workers() {
    let manager = PeerManager::default();
    manager.apply_inventory(RemoteInventory {
        version: 1,
        remotes: vec![
            record("worker-a", "direct", &["builder"], &["linux"]),
            record("worker-b", "tor", &["builder"], &["linux"]),
            record("worker-v1", "direct", &["builder"], &["linux"]),
        ],
    });
    manager.mark_worker_snapshot("worker-a", snapshot("worker-a", 2));
    manager.mark_worker_snapshot("worker-b", snapshot("worker-b", 2));
    manager.mark_worker_snapshot("worker-v1", snapshot("worker-v1", 1));
    let candidates = manager.remote_candidates(&RemoteRequirements {
        pool: Some("build".into()),
        required_tags: vec!["builder".into()],
        required_capabilities: vec!["linux".into()],
        transport: Some("direct".into()),
    });
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].node_id, "worker-a");
    assert_eq!(candidates[0].transport.as_deref(), Some("direct"));
    assert!(candidates[0].reason.contains("protocol-v2"));
    let target = manager.worker_target("worker-a", "direct").unwrap();
    assert_eq!(target.endpoint, "http://127.0.0.1:1/worker-a");
    assert_eq!(target.bearer_token, "secret");
    assert!(manager.worker_target("worker-a", "tor").is_none());
    assert!(!format!("{target:?}").contains("secret"));
}

#[test]
fn candidate_node_selector_accepts_the_existing_alias_and_id_prefix() {
    let manager = PeerManager::default();
    let node = "builder-node-123456";
    manager.apply_inventory(RemoteInventory {
        version: 1,
        remotes: vec![record(node, "direct", &[], &[])],
    });
    manager.mark_worker_snapshot(node, snapshot(node, 2));

    let alias = tak_core::remote_alias_for_node_id(node);
    for selector in [alias.as_str(), "builder-node"] {
        let candidates = manager.remote_candidates(&RemoteRequirements {
            pool: None,
            required_tags: Vec::new(),
            required_capabilities: vec![format!("node:{selector}")],
            transport: None,
        });
        assert_eq!(candidates.len(), 1, "selector {selector}");
    }
}

fn record(node: &str, transport: &str, tags: &[&str], capabilities: &[&str]) -> RemoteRecord {
    RemoteRecord {
        node_id: node.into(),
        display_name: node.into(),
        base_url: if transport == "direct" {
            format!("http://127.0.0.1:1/{node}")
        } else {
            format!("http://{node}.onion")
        },
        bearer_token: "secret".into(),
        pools: vec!["build".into()],
        tags: tags.iter().map(|value| (*value).into()).collect(),
        capabilities: capabilities.iter().map(|value| (*value).into()).collect(),
        transport: transport.into(),
        enabled: true,
    }
}

fn snapshot(node: &str, version: u16) -> WorkerSnapshot {
    WorkerSnapshot {
        node_id: node.into(),
        protocol_version: version,
        healthy: true,
        sampled_at_ms: 1,
        capacity: WorkerResources {
            cpu_millis: 8_000,
            memory_bytes: 16 * 1024 * 1024 * 1024,
            execution_slots: 8,
        },
        usage: WorkerResources {
            cpu_millis: 0,
            memory_bytes: 0,
            execution_slots: 0,
        },
        queue_depth: 0,
        cached_content: vec![],
        processes: vec![],
    }
}
