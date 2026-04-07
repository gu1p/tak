use super::*;

pub async fn run_daemon(socket_path: &Path) -> Result<()> {
    let db_path = std::env::var("TAKD_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_state_db_path());
    let manager = new_shared_manager_with_db(db_path.clone())?;

    {
        let mut guard = manager
            .lock()
            .map_err(|_| anyhow!("lease manager lock poisoned"))?;
        guard.set_capacity("cpu", Scope::Machine, None, 8.0);
        guard.set_capacity("ram_gib", Scope::Machine, None, 32.0);
    }

    spawn_optional_remote_v1_services(&db_path).await?;
    run_server(socket_path, manager).await
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
                eprintln!("takd remote v1 http server failed: {err}");
            }
        });
        eprintln!("takd remote v1 http listening on {local_addr}");
    }

    if let Some(config) = tor_hidden_service_runtime_config_from_env()? {
        let store = SubmitAttemptStore::with_db_path(db_path.to_path_buf())
            .context("failed to open takd tor hidden-service sqlite store")?;
        tokio::spawn(async move {
            if let Err(err) = run_remote_v1_tor_hidden_service(config, store).await {
                eprintln!("takd remote v1 tor hidden-service failed: {err}");
            }
        });
    }

    Ok(())
}
