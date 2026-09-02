use super::super::LocalNodeIdentity;
use super::support::{inventory, record};

#[test]
fn inventory_excludes_local_node_by_id() {
    let peers = super::fixtures::peer_manager_with_local_identity(
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
    let peers = super::fixtures::peer_manager_with_local_identity(
        inventory(vec![record("builder-a", "tor", true, "secret")]),
        LocalNodeIdentity::new("other".into(), Some("http://builder-a.onion".into())),
    );
    assert!(peers.snapshots().is_empty());
}

#[test]
fn no_local_identity_keeps_every_peer() {
    let peers = super::fixtures::peer_manager(inventory(vec![
        record("self", "tor", true, "secret"),
        record("builder-a", "tor", true, "secret"),
    ]));
    assert_eq!(peers.snapshots().len(), 2);
}
