fn image_cache_options(
    config: &super::types::RemoteImageCacheRuntimeConfig,
) -> tak_runner::ImageCacheOptions {
    tak_runner::ImageCacheOptions {
        db_path: config.db_path.clone(),
        budget_bytes: config.budget_bytes,
        mutable_tag_ttl_secs: config.mutable_tag_ttl_secs,
        sweep_interval_secs: config.sweep_interval_secs,
        low_disk_min_free_percent: config.low_disk_min_free_percent,
        low_disk_min_free_bytes: config.low_disk_min_free_bytes,
    }
}

fn execution_root_for_payload(
    idempotency_key: &str,
    execution_root_base: &Path,
    payload: &RemoteWorkerSubmitPayload,
) -> Result<PathBuf> {
    if matches!(
        payload.session.as_ref().map(|session| &session.reuse),
        Some(RemoteWorkerSessionReuse::ShareWorkspace)
    ) {
        let session = payload.session.as_ref().expect("checked session");
        return Ok(session_workspace_root(execution_root_base, &session.key));
    }
    Ok(execution_root_for_submit_key_at_base(
        idempotency_key,
        execution_root_base,
    ))
}

fn remote_container_user() -> Option<String> {
    match std::env::var("TAKD_REMOTE_CONTAINER_USER") {
        Ok(value) if value == "image" => None,
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => default_remote_container_user(),
        Err(std::env::VarError::NotUnicode(_)) => default_remote_container_user(),
    }
}

#[cfg(unix)]
fn default_remote_container_user() -> Option<String> {
    Some(format!(
        "{}:{}",
        unsafe { libc::geteuid() },
        unsafe { libc::getegid() }
    ))
}

#[cfg(not(unix))]
fn default_remote_container_user() -> Option<String> {
    None
}

fn prepare_execution_root(execution_root: &Path, payload: &RemoteWorkerSubmitPayload) -> Result<()> {
    if is_share_workspace(payload) && execution_root.exists() {
        return Ok(());
    }
    if execution_root.exists() {
        fs::remove_dir_all(execution_root).with_context(|| {
            format!(
                "failed to clear existing remote execution root {}",
                execution_root.display()
            )
        })?;
    }
    fs::create_dir_all(execution_root).with_context(|| {
        format!(
            "failed to create remote execution root {}",
            execution_root.display()
        )
    })
}

fn unpack_payload_workspace(payload: &RemoteWorkerSubmitPayload, execution_root: &Path) -> Result<()> {
    if is_share_workspace(payload) && execution_root.read_dir()?.next().is_some() {
        return Ok(());
    }
    unpack_remote_worker_workspace(&payload.workspace_zip, execution_root)
}

fn cleanup_execution_root(payload: &RemoteWorkerSubmitPayload, execution_root: &Path) -> Result<()> {
    if is_share_workspace(payload) {
        return Ok(());
    }
    match fs::remove_dir_all(execution_root) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| {
            format!(
                "failed to remove remote execution root {}",
                execution_root.display()
            )
        }),
    }
}
