use std::time::Duration;

use takd::{PeerManager, RunStore, TorBroker, new_shared_manager_with_db};

use super::super::takd_bin;

pub struct Origin {
    _temp: tempfile::TempDir,
    server: tokio::task::JoinHandle<anyhow::Result<()>>,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    pub store: RunStore,
}

impl Origin {
    pub async fn start(peers: PeerManager, broker: TorBroker) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let socket = temp.path().join("origin.sock");
        let db = socket.with_extension("v2.sqlite");
        let inventory = temp.path().join("remotes.toml");
        let store = RunStore::with_db_path(db.clone()).unwrap();
        let manager = new_shared_manager_with_db(db).unwrap();
        let server_store = store.clone();
        let server_socket = super::super::socket_path::bind_path(&socket);
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            takd::run_server_with_local_attempt_executable_and_remote_inventory_until_shutdown(
                &server_socket,
                manager,
                broker,
                peers,
                server_store,
                takd_bin(),
                inventory,
                shutdown_rx,
            )
            .await
        });
        tokio::time::timeout(Duration::from_secs(2), async {
            while !socket.exists() && !server.is_finished() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        if server.is_finished() {
            panic!("origin daemon exited before serving: {:?}", server.await);
        }
        Self {
            _temp: temp,
            server,
            shutdown: Some(shutdown_tx),
            store,
        }
    }

    pub async fn wait_for_terminal(&self, run_id: &str) {
        tokio::time::timeout(Duration::from_secs(10), async {
            while !self
                .store
                .summary(run_id)
                .unwrap()
                .unwrap()
                .state
                .is_terminal()
            {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap_or_else(|error| {
            panic!(
                "run did not finish: {error:?}; summary={:?}; events={:?}",
                self.store.summary(run_id).unwrap(),
                self.store.events_after(run_id, 0).unwrap(),
            )
        });
    }
}

impl Drop for Origin {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        self.server.abort();
    }
}
