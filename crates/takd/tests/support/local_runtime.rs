#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tak_core::remote_inventory::RemoteInventory;
use takd::{LeaseManager, PeerManager, SharedLeaseManager, TorBroker};

pub fn in_memory_lease_manager() -> SharedLeaseManager {
    Arc::new(Mutex::new(LeaseManager::default()))
}

pub fn peer_manager(inventory: RemoteInventory) -> PeerManager {
    let peers = PeerManager::default();
    peers.apply_inventory(inventory);
    peers
}

pub fn tor_broker(dial_addr: impl AsRef<str>) -> TorBroker {
    TorBroker::for_direct_dial(dial_addr)
}

pub async fn run_local_server(
    socket_path: &Path,
    manager: SharedLeaseManager,
    broker: TorBroker,
) -> anyhow::Result<()> {
    takd::run_server_with_broker_and_peers(socket_path, manager, broker, PeerManager::default())
        .await
}

pub fn spawn_local_server(
    socket_path: PathBuf,
    broker: TorBroker,
) -> tokio::task::JoinHandle<anyhow::Result<()>> {
    tokio::spawn(
        async move { run_local_server(&socket_path, in_memory_lease_manager(), broker).await },
    )
}
