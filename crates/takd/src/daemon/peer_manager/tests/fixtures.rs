use tak_core::remote_inventory::RemoteInventory;

use super::super::{LocalNodeIdentity, PeerManager};
use crate::daemon::protocol::TorBroker;

pub(super) fn peer_manager(inventory: RemoteInventory) -> PeerManager {
    let manager = PeerManager::default();
    manager.apply_inventory(inventory);
    manager
}

pub(super) fn peer_manager_with_local_identity(
    inventory: RemoteInventory,
    identity: LocalNodeIdentity,
) -> PeerManager {
    let manager = PeerManager::default();
    manager.set_local_identity(identity);
    manager.apply_inventory(inventory);
    manager
}

pub(super) fn broker_for_dial_addr(dial_addr: String) -> TorBroker {
    TorBroker::for_direct_dial(dial_addr)
}
