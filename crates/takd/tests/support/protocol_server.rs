use std::path::PathBuf;

use tak_core::model::Scope;
use takd::{AcquireLeaseResponse, SharedLeaseManager, new_shared_manager_with_db, run_server};

use crate::support::protocol::acquire_request;

pub fn spawn_protocol_server(
    db_path: PathBuf,
    socket_path: PathBuf,
) -> tokio::task::JoinHandle<anyhow::Result<()>> {
    let manager = new_shared_manager_with_db(db_path).expect("manager");
    configure_manager(&manager);
    spawn_protocol_server_with_manager(socket_path, manager)
}

pub fn seeded_protocol_server(
    db_path: PathBuf,
    socket_path: PathBuf,
    request_id: &str,
) -> (tokio::task::JoinHandle<anyhow::Result<()>>, String) {
    let manager = new_shared_manager_with_db(db_path).expect("manager");
    configure_manager(&manager);
    let lease_id = {
        let mut guard = manager.lock().expect("lease manager lock");
        match guard.acquire(acquire_request(request_id)) {
            AcquireLeaseResponse::LeaseGranted { lease } => lease.lease_id,
            AcquireLeaseResponse::LeasePending { .. } => panic!("expected seeded lease"),
        }
    };
    (
        spawn_protocol_server_with_manager(socket_path, manager),
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
) -> tokio::task::JoinHandle<anyhow::Result<()>> {
    tokio::spawn(async move { run_server(&socket_path, manager).await })
}
