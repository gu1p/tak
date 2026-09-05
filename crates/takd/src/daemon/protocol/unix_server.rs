use super::*;

mod lifecycle;
mod socket_binding;
#[cfg(test)]
mod socket_binding_tests;
mod socket_permissions;
#[cfg(test)]
mod socket_permissions_tests;

pub async fn run_server_with_broker_and_peers(
    socket_path: &Path,
    manager: SharedLeaseManager,
    broker: TorBroker,
    peers: crate::daemon::peer_manager::PeerManager,
) -> Result<()> {
    let run_store = RunStore::with_db_path(socket_path.with_extension("v2.sqlite"))?;
    run_server_with_broker_peers_and_run_store(socket_path, manager, broker, peers, run_store).await
}

pub async fn run_server_with_broker_peers_and_remote_inventory(
    socket_path: &Path,
    manager: SharedLeaseManager,
    broker: TorBroker,
    peers: crate::daemon::peer_manager::PeerManager,
    remote_inventory_path: PathBuf,
) -> Result<()> {
    let run_store = RunStore::with_db_path(socket_path.with_extension("v2.sqlite"))?;
    let executable = std::env::current_exe().context("resolve local attempt executable")?;
    run_server(
        socket_path,
        manager,
        broker,
        peers,
        run_store,
        executable,
        remote_inventory_path,
        Box::pin(std::future::pending()),
    )
    .await
}

pub async fn run_server_with_broker_peers_and_run_store(
    socket_path: &Path,
    manager: SharedLeaseManager,
    broker: TorBroker,
    peers: crate::daemon::peer_manager::PeerManager,
    run_store: RunStore,
) -> Result<()> {
    let executable = std::env::current_exe().context("resolve local attempt executable")?;
    run_server_with_local_attempt_executable(
        socket_path,
        manager,
        broker,
        peers,
        run_store,
        executable,
    )
    .await
}

pub async fn run_server_with_local_attempt_executable(
    socket_path: &Path,
    manager: SharedLeaseManager,
    broker: TorBroker,
    peers: crate::daemon::peer_manager::PeerManager,
    run_store: RunStore,
    local_attempt_executable: PathBuf,
) -> Result<()> {
    run_server(
        socket_path,
        manager,
        broker,
        peers,
        run_store,
        local_attempt_executable,
        default_remote_inventory_path()?,
        Box::pin(std::future::pending()),
    )
    .await
}

pub async fn run_server_with_local_attempt_executable_until_shutdown(
    socket_path: &Path,
    manager: SharedLeaseManager,
    broker: TorBroker,
    peers: crate::daemon::peer_manager::PeerManager,
    run_store: RunStore,
    local_attempt_executable: PathBuf,
    shutdown: tokio::sync::oneshot::Receiver<()>,
) -> Result<()> {
    run_server_with_local_attempt_executable_and_remote_inventory_until_shutdown(
        socket_path,
        manager,
        broker,
        peers,
        run_store,
        local_attempt_executable,
        default_remote_inventory_path()?,
        shutdown,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn run_server_with_local_attempt_executable_and_remote_inventory_until_shutdown(
    socket_path: &Path,
    manager: SharedLeaseManager,
    broker: TorBroker,
    peers: crate::daemon::peer_manager::PeerManager,
    run_store: RunStore,
    local_attempt_executable: PathBuf,
    remote_inventory_path: PathBuf,
    shutdown: tokio::sync::oneshot::Receiver<()>,
) -> Result<()> {
    run_server(
        socket_path,
        manager,
        broker,
        peers,
        run_store,
        local_attempt_executable,
        remote_inventory_path,
        Box::pin(async move {
            let _ = shutdown.await;
        }),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_server(
    socket_path: &Path,
    manager: SharedLeaseManager,
    broker: TorBroker,
    peers: crate::daemon::peer_manager::PeerManager,
    run_store: RunStore,
    local_attempt_executable: PathBuf,
    remote_inventory_path: PathBuf,
    shutdown: lifecycle::ServerShutdown,
) -> Result<()> {
    if let Some(parent) = socket_path.parent() {
        let created = socket_permissions::ensure_parent(parent).await?;
        if created || tak_core::runtime_paths::daemon_socket_parent_requires_owner_only(socket_path)
        {
            socket_permissions::set_parent_owner_only(parent).await?;
            socket_permissions::verify_parent_owner_only(parent).await?;
        }
    }

    let (listener, _socket_lock) = socket_binding::bind(socket_path).await?;
    socket_permissions::set_owner_only(socket_path).await?;
    let remote_access =
        crate::daemon::RemoteAccess::new(remote_inventory_path, broker.clone(), peers.clone())?;
    lifecycle::serve(
        listener,
        manager,
        broker,
        peers,
        run_store,
        local_attempt_executable,
        remote_access,
        shutdown,
    )
    .await
}

fn default_remote_inventory_path() -> Result<PathBuf> {
    tak_core::remote_inventory::default_remote_inventory_path()
        .context("resolve daemon remote inventory path")
}
