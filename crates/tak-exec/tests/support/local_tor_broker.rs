#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use takd::{PeerManager, TorBroker, new_shared_manager, run_server_with_broker_and_peers};
use tokio::net::UnixStream;
use tokio::task::JoinHandle;

use super::EnvGuard;

#[path = "local_tor_broker/broker_error.rs"]
mod broker_error;
pub use broker_error::spawn_broker_error;

pub struct LocalTorBroker {
    handle: JoinHandle<anyhow::Result<()>>,
    socket_path: PathBuf,
    broker: TorBroker,
}

impl LocalTorBroker {
    pub async fn spawn(root: &Path, dial_addr: &str, env: &mut EnvGuard) -> Self {
        let socket_path = root.join("run/takd-broker.sock");
        env.set("TAKD_SOCKET", socket_path.display().to_string());
        env.set("TAK_REMOTE_WORKSPACE_TRANSFER", "tor-stream");
        let broker = TorBroker::for_test_dial_addr(dial_addr.to_string());
        let peers = peer_manager_from_current_inventory(broker.clone());
        // Mirror production: warm peer connections via heartbeats so placement
        // sees a Connected (not merely Connecting) peer.
        peers.spawn_heartbeat_loop(broker.clone());
        let broker_for_task = broker.clone();
        let socket_for_task = socket_path.clone();
        let handle = tokio::spawn(async move {
            run_server_with_broker_and_peers(
                &socket_for_task,
                new_shared_manager(),
                broker_for_task,
                peers,
            )
            .await
        });
        wait_for_socket(&socket_path).await;
        Self {
            handle,
            socket_path,
            broker,
        }
    }

    pub fn bootstrap_count(&self) -> usize {
        self.broker.bootstrap_count()
    }
}

fn peer_manager_from_current_inventory(broker: TorBroker) -> PeerManager {
    let Ok(path) = tak_core::remote_inventory::default_remote_inventory_path() else {
        return PeerManager::default();
    };
    let peers = tak_core::remote_inventory::load_remote_inventory_at(&path)
        .map(PeerManager::from_inventory)
        .unwrap_or_default();
    peers.spawn_inventory_reloader_with_broker(path, broker);
    peers
}

impl Drop for LocalTorBroker {
    fn drop(&mut self) {
        self.handle.abort();
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

async fn wait_for_socket(socket_path: &Path) {
    for _ in 0..50 {
        if UnixStream::connect(socket_path).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!(
        "timed out waiting for broker socket {}",
        socket_path.display()
    );
}
