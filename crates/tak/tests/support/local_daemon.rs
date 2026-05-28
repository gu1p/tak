#![allow(dead_code)]

use super::local_daemon_manager::manager_for;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use tak_core::model::WorkspaceSpec;
use takd::{PeerManager, TorBroker, run_server_with_broker_and_peers};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

pub struct LocalDaemonGuard {
    runtime: Runtime,
    task: JoinHandle<()>,
    socket_path: PathBuf,
}
impl LocalDaemonGuard {
    pub fn spawn(socket_path: &Path, spec: &WorkspaceSpec) -> Self {
        Self::spawn_with_broker(socket_path, spec, TorBroker::new())
    }

    pub fn spawn_with_tor_dial_addr(
        socket_path: &Path,
        spec: &WorkspaceSpec,
        dial_addr: String,
    ) -> Self {
        Self::spawn_with_broker(socket_path, spec, TorBroker::for_test_dial_addr(dial_addr))
    }
    pub fn spawn_with_tor_inventory(
        socket_path: &Path,
        spec: &WorkspaceSpec,
        dial_addr: String,
        inventory_path: PathBuf,
    ) -> Self {
        let broker = TorBroker::for_test_dial_addr(dial_addr);
        let inventory = tak_core::remote_inventory::load_remote_inventory_at(&inventory_path)
            .expect("load client remote inventory for local daemon");
        let peers = PeerManager::from_inventory(inventory);
        Self::spawn_with_broker_and_peers(socket_path, spec, broker, peers)
    }
    fn spawn_with_broker(socket_path: &Path, spec: &WorkspaceSpec, broker: TorBroker) -> Self {
        Self::spawn_with_broker_and_peers(socket_path, spec, broker, PeerManager::default())
    }
    fn spawn_with_broker_and_peers(
        socket_path: &Path,
        spec: &WorkspaceSpec,
        broker: TorBroker,
        peers: PeerManager,
    ) -> Self {
        let manager = manager_for(spec);
        let runtime = Runtime::new().expect("tokio runtime");
        let manager = Arc::clone(&manager);
        let socket_path = socket_path.to_path_buf();
        let serve_path = socket_path.clone();
        let (startup_tx, startup_rx) = mpsc::channel();
        let task = runtime.spawn(async move {
            let exit = run_server_with_broker_and_peers(&serve_path, manager, broker, peers).await;
            let message = match exit {
                Ok(()) => "server exited before local daemon socket appeared".to_string(),
                Err(err) => format!("{err:#}"),
            };
            let _ = startup_tx.send(message);
        });
        let deadline = Instant::now() + Duration::from_secs(30);
        while !socket_path.exists() {
            if let Ok(message) = startup_rx.try_recv() {
                panic!(
                    "local daemon exited before socket {} was ready: {message}",
                    socket_path.display()
                );
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for local daemon socket {}",
                socket_path.display()
            );
            thread::sleep(Duration::from_millis(20));
        }
        Self {
            runtime,
            task,
            socket_path,
        }
    }
}
impl Drop for LocalDaemonGuard {
    fn drop(&mut self) {
        self.task.abort();
        self.runtime
            .block_on(async { tokio::time::sleep(Duration::from_millis(20)).await });
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
