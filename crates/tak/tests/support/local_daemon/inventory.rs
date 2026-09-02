use std::path::{Path, PathBuf};

use tak_core::model::WorkspaceSpec;
use takd::{PeerManager, TorBroker};

use super::{LocalDaemonGuard, non_tor_broker, takd_bin};

impl LocalDaemonGuard {
    pub fn spawn_with_tor_inventory(
        socket_path: &Path,
        spec: &WorkspaceSpec,
        dial_addr: String,
        inventory_path: PathBuf,
    ) -> Self {
        let broker = TorBroker::for_direct_dial(dial_addr);
        let peers = peers_from_inventory(&inventory_path);
        Self::spawn_inner(socket_path, spec, broker, peers, takd_bin())
    }

    pub fn spawn_with_inventory(
        socket_path: &Path,
        spec: &WorkspaceSpec,
        inventory_path: PathBuf,
    ) -> Self {
        let peers = peers_from_inventory(&inventory_path);
        Self::spawn_inner(socket_path, spec, non_tor_broker(), peers, takd_bin())
    }
}

fn peers_from_inventory(inventory_path: &Path) -> PeerManager {
    let inventory = tak_core::remote_inventory::load_remote_inventory_at(inventory_path)
        .expect("load client remote inventory for local daemon");
    let peers = PeerManager::default();
    peers.apply_inventory(inventory);
    peers
}
