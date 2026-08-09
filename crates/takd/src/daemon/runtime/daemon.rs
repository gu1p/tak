use super::*;

pub(crate) async fn run_local_daemon_with_broker_and_peers(
    socket_path: &Path,
    db_path: &Path,
    broker: crate::daemon::protocol::TorBroker,
    peers: crate::daemon::peer_manager::PeerManager,
) -> Result<()> {
    let manager = local_daemon_manager(db_path)?;
    crate::daemon::protocol::run_server_with_broker_and_peers(socket_path, manager, broker, peers)
        .await
}

fn local_daemon_manager(db_path: &Path) -> Result<crate::daemon::lease::SharedLeaseManager> {
    let manager = new_shared_manager_with_db(db_path.to_path_buf())?;
    let mut guard = manager
        .lock()
        .map_err(|_| anyhow!("lease manager lock poisoned"))?;
    guard.set_capacity("cpu", Scope::Machine, None, 8.0);
    guard.set_capacity("ram_gib", Scope::Machine, None, 32.0);
    drop(guard);
    Ok(manager)
}
