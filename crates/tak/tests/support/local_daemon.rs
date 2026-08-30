#![allow(dead_code)]

use super::local_daemon_manager::manager_for;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};
use tak_core::model::WorkspaceSpec;
use takd::{PeerManager, RunStore, TorBroker, run_server_with_local_attempt_executable};

pub struct LocalDaemonGuard {
    runtime: tokio::runtime::Runtime,
    task: tokio::task::JoinHandle<()>,
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
        Self::spawn_with_broker(socket_path, spec, TorBroker::for_direct_dial(dial_addr))
    }
    pub fn spawn_with_tor_inventory(
        socket_path: &Path,
        spec: &WorkspaceSpec,
        dial_addr: String,
        inventory_path: PathBuf,
    ) -> Self {
        let broker = TorBroker::for_direct_dial(dial_addr);
        let inventory = tak_core::remote_inventory::load_remote_inventory_at(&inventory_path)
            .expect("load client remote inventory for local daemon");
        let peers = PeerManager::default();
        peers.apply_inventory(inventory);
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
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        let manager = Arc::clone(&manager_for(spec));
        let socket_path = socket_path.to_path_buf();
        let serve_path = socket_path.clone();
        let run_store = RunStore::with_db_path(socket_path.with_extension("v2.sqlite")).unwrap();
        let attempt_executable = super::takd_binary::takd_bin();
        let (startup_tx, startup_rx) = mpsc::channel();
        let task = runtime.spawn(async move {
            let exit = run_server_with_local_attempt_executable(
                &serve_path,
                manager,
                broker,
                peers,
                run_store,
                attempt_executable,
            )
            .await;
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
            std::thread::sleep(Duration::from_millis(20));
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
