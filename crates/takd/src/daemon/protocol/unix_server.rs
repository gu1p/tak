use super::*;

pub async fn run_server(socket_path: &Path, manager: SharedLeaseManager) -> Result<()> {
    run_server_with_broker(socket_path, manager, TorBroker::new()).await
}

pub async fn run_server_with_broker(
    socket_path: &Path,
    manager: SharedLeaseManager,
    broker: TorBroker,
) -> Result<()> {
    run_server_with_broker_and_peers(
        socket_path,
        manager,
        broker,
        crate::daemon::peer_manager::PeerManager::default(),
    )
    .await
}

pub async fn run_server_with_broker_and_peers(
    socket_path: &Path,
    manager: SharedLeaseManager,
    broker: TorBroker,
    peers: crate::daemon::peer_manager::PeerManager,
) -> Result<()> {
    if let Some(parent) = socket_path.parent() {
        let created = ensure_socket_parent(parent).await?;
        if created || tak_core::runtime_paths::daemon_socket_parent_requires_owner_only(socket_path)
        {
            set_owner_only_dir_permissions(parent).await?;
            verify_owner_only_dir(parent).await?;
        }
    }

    if socket_path.exists() {
        tokio::fs::remove_file(socket_path).await.with_context(|| {
            format!("failed to remove existing socket {}", socket_path.display())
        })?;
    }

    let listener = UnixListener::bind(socket_path)
        .with_context(|| format!("failed to bind socket {}", socket_path.display()))?;
    set_owner_only_socket_permissions(socket_path).await?;
    let warm_broker = broker.clone();
    let tasks = DaemonTaskHandles::default();
    tokio::spawn(async move {
        if let Err(err) = warm_broker.warm().await {
            tracing::debug!("local Tor broker warmup failed: {err:#}");
        }
    });

    loop {
        let (stream, _) = listener.accept().await.context("accept failed")?;
        let manager = Arc::clone(&manager);
        let broker = broker.clone();
        let peers = peers.clone();
        let tasks = tasks.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_client(stream, manager, broker, peers, tasks).await {
                tracing::error!("client handling error: {err}");
            }
        });
    }
}

async fn ensure_socket_parent(path: &Path) -> Result<bool> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) if metadata.is_dir() => Ok(false),
        Ok(_) => bail!("socket parent is not a directory: {}", path.display()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            tokio::fs::create_dir_all(path)
                .await
                .with_context(|| format!("failed to create socket directory {}", path.display()))?;
            Ok(true)
        }
        Err(err) => Err(err)
            .with_context(|| format!("failed to inspect socket directory {}", path.display())),
    }
}

#[cfg(unix)]
async fn verify_owner_only_dir(path: &Path) -> Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let metadata = tokio::fs::symlink_metadata(path)
        .await
        .with_context(|| format!("inspect socket directory {}", path.display()))?;
    if !metadata.is_dir() {
        bail!("socket parent is not a directory: {}", path.display());
    }
    let expected_uid = unsafe { libc::geteuid() };
    if metadata.uid() != expected_uid {
        bail!(
            "socket parent is not owned by current user: {}",
            path.display()
        );
    }
    let mode = metadata.permissions().mode() & 0o777;
    if mode != 0o700 {
        bail!("socket parent is not owner-only: {}", path.display());
    }
    Ok(())
}

#[cfg(not(unix))]
async fn verify_owner_only_dir(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
async fn set_owner_only_dir_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .await
        .with_context(|| format!("set socket directory permissions {}", path.display()))
}

#[cfg(not(unix))]
async fn set_owner_only_dir_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
async fn set_owner_only_socket_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .await
        .with_context(|| format!("set socket permissions {}", path.display()))
}

#[cfg(not(unix))]
async fn set_owner_only_socket_permissions(_path: &Path) -> Result<()> {
    Ok(())
}
