use tak_core::remote_inventory::{RemoteInventory, RemoteRecord};

use super::super::{PeerManager, PeerState};

#[test]
fn peer_manager_loads_enabled_tor_remotes_only() {
    let manager = PeerManager::from_inventory(inventory(vec![
        record("builder-a", "tor", true, "secret"),
        record("builder-disabled", "tor", false, "secret"),
        record("builder-direct", "direct", true, "secret"),
    ]));

    let snapshots = manager.snapshots();

    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].node_id, "builder-a");
    assert_eq!(snapshots[0].state, PeerState::Connecting);
}

#[test]
fn malformed_reload_preserves_last_good_peer_state() {
    let manager =
        PeerManager::from_inventory(inventory(vec![record("builder-a", "tor", true, "secret")]));

    manager.apply_inventory_result(Err(anyhow::anyhow!("invalid toml")));

    assert_eq!(manager.snapshots()[0].node_id, "builder-a");
}

#[test]
fn token_change_clears_sticky_auth_failed_state() {
    let manager =
        PeerManager::from_inventory(inventory(vec![record("builder-a", "tor", true, "old")]));
    manager.mark_auth_failed("builder-a", "unauthorized");

    manager.apply_inventory(inventory(vec![record("builder-a", "tor", true, "new")]));

    let snapshot = manager.snapshots().pop().expect("snapshot");
    assert_eq!(snapshot.state, PeerState::Connecting);
    assert_eq!(snapshot.last_error_summary, None);
}

#[test]
fn removed_or_changed_inventory_reports_sessions_to_evict() {
    let manager = PeerManager::from_inventory(inventory(vec![
        record("builder-removed", "tor", true, "old-secret"),
        record("builder-changed", "tor", true, "old-secret"),
    ]));

    let evicted = manager.apply_inventory(inventory(vec![record(
        "builder-changed",
        "tor",
        true,
        "new-secret",
    )]));
    let evicted_ids = evicted
        .iter()
        .map(|target| target.node_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(evicted_ids, vec!["builder-changed", "builder-removed"]);
}

fn inventory(remotes: Vec<RemoteRecord>) -> RemoteInventory {
    RemoteInventory {
        version: 1,
        remotes,
    }
}

fn record(node_id: &str, transport: &str, enabled: bool, bearer_token: &str) -> RemoteRecord {
    RemoteRecord {
        node_id: node_id.to_string(),
        display_name: node_id.to_string(),
        base_url: format!("http://{node_id}.onion"),
        bearer_token: bearer_token.to_string(),
        pools: vec!["build".to_string()],
        tags: vec!["linux".to_string()],
        capabilities: vec!["docker".to_string()],
        transport: transport.to_string(),
        enabled,
    }
}
