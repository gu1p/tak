use super::broker_arti_root;
use crate::agent::{arti_cache_dir, arti_state_dir};
use std::path::Path;

// Regression guard for the peer-unreachable outage: the outbound broker's Arti
// directories must never coincide with (or nest inside) the hidden-service
// client's. Sharing one state directory makes the second TorClient lose the
// on-disk lock, drop to read-only mode, and never finish bootstrap, so every
// heartbeat timed out before dialing and all peers stayed `unreachable`.
#[test]
fn broker_arti_dirs_are_separate_from_hidden_service_dirs() {
    let root = Path::new("/var/lib/takd-regression");
    let broker = broker_arti_root(root);
    assert_ne!(broker.join("state"), arti_state_dir(root));
    assert_ne!(broker.join("cache"), arti_cache_dir(root));

    // The two Arti roots must be disjoint so neither holds the other's lock.
    let hidden_service_root = arti_state_dir(root)
        .parent()
        .expect("hidden-service arti state dir has a parent")
        .to_path_buf();
    assert!(!broker.starts_with(&hidden_service_root));
    assert!(!hidden_service_root.starts_with(&broker));
}
