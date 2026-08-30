use std::path::PathBuf;

use tak_core::model::Scope;
use takd::{AcquireLeaseResponse, SharedLeaseManager, new_shared_manager_with_db};

use crate::support::protocol::acquire_request;

pub fn spawn_protocol_server(
    db_path: PathBuf,
    socket_path: PathBuf,
) -> tokio::task::JoinHandle<anyhow::Result<()>> {
    let manager = new_shared_manager_with_db(db_path.clone()).expect("manager");
    configure_manager(&manager);
    let run_store = takd::RunStore::with_db_path(db_path).expect("run store");
    spawn_protocol_server_with_manager(socket_path, manager, run_store)
}

pub fn seeded_protocol_server(
    db_path: PathBuf,
    socket_path: PathBuf,
    request_id: &str,
) -> (tokio::task::JoinHandle<anyhow::Result<()>>, String) {
    let manager = new_shared_manager_with_db(db_path.clone()).expect("manager");
    configure_manager(&manager);
    let lease_id = {
        let mut guard = manager.lock().expect("lease manager lock");
        match guard.acquire(acquire_request(request_id)) {
            AcquireLeaseResponse::LeaseGranted { lease } => lease.lease_id,
            AcquireLeaseResponse::LeasePending { .. } => panic!("expected seeded lease"),
        }
    };
    (
        spawn_protocol_server_with_manager(
            socket_path,
            manager,
            takd::RunStore::with_db_path(db_path).expect("run store"),
        ),
        lease_id,
    )
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
    tokio::spawn(async move {
        takd::run_server_with_broker_peers_and_run_store(
            &socket_path,
            manager,
            takd::TorBroker::new(),
            takd::PeerManager::default(),
            run_store,
        )
        .await
    })
}
