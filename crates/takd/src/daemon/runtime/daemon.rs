use super::*;

pub async fn run_daemon(socket_path: &Path) -> Result<()> {
    let db_path = std::env::var("TAKD_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_state_db_path());
    spawn_optional_remote_v1_services(&db_path).await?;
    run_local_daemon(socket_path, &db_path).await
}

pub(crate) async fn run_local_daemon(socket_path: &Path, db_path: &Path) -> Result<()> {
    run_local_daemon_with_peers(
        socket_path,
        db_path,
        crate::daemon::peer_manager::PeerManager::default(),
    )
    .await
}

pub(crate) async fn run_local_daemon_with_peers(
    socket_path: &Path,
    db_path: &Path,
    peers: crate::daemon::peer_manager::PeerManager,
) -> Result<()> {
    run_local_daemon_with_broker_and_peers(
        socket_path,
        db_path,
        crate::daemon::protocol::TorBroker::new(),
        peers,
    )
    .await
}

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

async fn spawn_optional_remote_v1_services(db_path: &Path) -> Result<()> {
    if let Some(bind_addr) = remote_v1_bind_addr_from_env() {
        let listener = TcpListener::bind(bind_addr.as_str())
            .await
            .with_context(|| {
                format!("failed to bind takd remote v1 http listener at {bind_addr}")
            })?;
        let local_addr = listener
            .local_addr()
            .context("failed to read takd remote v1 local address")?;
        let store = SubmitAttemptStore::with_db_path(db_path.to_path_buf())
            .context("failed to open takd remote v1 sqlite store")?;
        let context = remote_node_context_from_env(Some(format!("http://{local_addr}")));
        tokio::spawn(async move {
            if let Err(err) = run_remote_v1_http_server(listener, store, context).await {
                tracing::error!("takd remote v1 http server failed: {err}");
            }
        });
        tracing::info!("takd remote v1 http listening on {local_addr}");
    }

    if let Some(config) = tor_hidden_service_runtime_config_from_env()? {
        let store = SubmitAttemptStore::with_db_path(db_path.to_path_buf())
            .context("failed to open takd tor hidden-service sqlite store")?;
        tokio::spawn(async move {
            if let Err(err) = run_remote_v1_tor_hidden_service(config, store).await {
                tracing::error!("takd remote v1 tor hidden-service failed: {err}");
            }
        });
    }

    Ok(())
}
