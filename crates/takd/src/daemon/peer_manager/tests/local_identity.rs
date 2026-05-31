use tak_proto::NodePingResponse;

use super::super::{
    LocalNodeIdentity, PeerEligibility, PeerManager, PeerPlacementRequest, PeerPlacementSelection,
};
use super::support::{inventory, record};

#[test]
fn inventory_excludes_local_node_by_id() {
    let peers = PeerManager::from_inventory_with_local_identity(
        inventory(vec![
            record("self", "tor", true, "secret"),
            record("builder-a", "tor", true, "secret"),
        ]),
        LocalNodeIdentity::new("self".into(), None),
    );
    let ids = peers
        .snapshots()
        .into_iter()
        .map(|peer| peer.node_id)
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["builder-a".to_string()]);
}

#[test]
fn inventory_excludes_local_node_by_endpoint() {
    let peers = PeerManager::from_inventory_with_local_identity(
        inventory(vec![record("builder-a", "tor", true, "secret")]),
        LocalNodeIdentity::new("other".into(), Some("http://builder-a.onion".into())),
    );
    assert!(peers.snapshots().is_empty());
}

#[test]
fn no_local_identity_keeps_every_peer() {
    let peers = PeerManager::from_inventory(inventory(vec![
        record("self", "tor", true, "secret"),
        record("builder-a", "tor", true, "secret"),
    ]));
    assert_eq!(peers.snapshots().len(), 2);
}

#[test]
fn select_placeable_skips_the_local_node() {
    // The peer is adopted before the local identity is known, then the local
    // node is learned: the defensive guard must still keep it out of placement.
    let peers = PeerManager::from_inventory(inventory(vec![record("self", "tor", true, "secret")]));
    peers.mark_ping_success("self", healthy_ping("self"), 1);
    peers.set_local_identity(LocalNodeIdentity::new("self".into(), None));

    let err = peers
        .select_placeable(PeerPlacementRequest {
            requirements: &PeerEligibility::default(),
            selection: PeerPlacementSelection::Sequential,
            task_run_id: "task-1",
            attempt: 1,
        })
        .expect_err("local node must never be placeable");
    assert!(format!("{err:#}").contains("no configured Tor peers"));
}

fn healthy_ping(node_id: &str) -> NodePingResponse {
    NodePingResponse {
        node_id: node_id.to_string(),
        protocol_version: "v1".to_string(),
        health: "healthy".to_string(),
        active_job_count: 0,
        queue_depth: 0,
        resource_summary: "cpu_available=4".to_string(),
    }
}
