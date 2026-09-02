use std::path::PathBuf;

use tak_core::model::Scope;
use takd::{SharedLeaseManager, new_shared_manager_with_db};

pub fn spawn_protocol_server(
    db_path: PathBuf,
    socket_path: PathBuf,
) -> tokio::task::JoinHandle<anyhow::Result<()>> {
    let manager = new_shared_manager_with_db(db_path.clone()).expect("manager");
    configure_manager(&manager);
    let run_store = takd::RunStore::with_db_path(db_path).expect("run store");
    spawn_protocol_server_with_manager(socket_path, manager, run_store)
}

fn configure_manager(manager: &SharedLeaseManager) {
    let mut guard = manager.lock().expect("lease manager lock");
    guard.set_capacity("cpu", Scope::Machine, None, 8.0);
    guard.set_capacity("ram_gib", Scope::Machine, None, 32.0);
}

fn spawn_protocol_server_with_manager(
    socket_path: PathBuf,
    manager: SharedLeaseManager,
    run_store: takd::RunStore,
) -> tokio::task::JoinHandle<anyhow::Result<()>> {
    let bind_path = super::socket_path::bind_path(&socket_path);
    tokio::spawn(async move {
        let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let inventory = socket_path.with_extension("remotes.toml");
        takd::run_server_with_local_attempt_executable_and_remote_inventory_until_shutdown(
            &bind_path,
            manager,
            takd::TorBroker::for_direct_dial("127.0.0.1:9"),
            takd::PeerManager::default(),
            run_store,
            super::takd_bin(),
            inventory,
            shutdown_rx,
        )
        .await
    })
}
