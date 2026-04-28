fn execute_remote_worker_submit(
    idempotency_key: &str,
    execution_root_base: &Path,
    selected_node_id: &str,
    image_cache: Option<&super::types::RemoteImageCacheRuntimeConfig>,
    payload: &RemoteWorkerSubmitPayload,
    output_observer: Arc<dyn TaskOutputObserver>,
) -> Result<(
    tak_runner::RemoteWorkerExecutionResult,
    Vec<RemoteWorkerOutputRecord>,
)> {
    let execution_root =
        execution_root_for_payload(idempotency_key, execution_root_base, payload)?;
    let artifact_root = artifact_root_for_submit_key_at_base(idempotency_key, execution_root_base);
    prepare_execution_root(&execution_root, payload)?;

    let execution_result = (|| -> Result<_> {
        unpack_payload_workspace(payload, &execution_root)?;
        overlay_session_paths(execution_root_base, payload, &execution_root)?;

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("failed to create tokio runtime for remote worker execution")?;
        let task_label = parse_label(&payload.task_label, "//")
            .map_err(|err| anyhow!("invalid submit task label {}: {err}", payload.task_label))?;
        let result = runtime.block_on(execute_remote_worker_steps_with_output(
            &execution_root,
            &RemoteWorkerExecutionSpec {
                task_label,
                attempt: payload.attempt,
                steps: payload.steps.clone(),
                timeout_s: payload.timeout_s,
                runtime: payload.runtime.clone(),
                node_id: selected_node_id.to_string(),
                container_user: remote_container_user(),
                image_cache: image_cache.map(image_cache_options),
            },
            Some(output_observer),
        ))?;
        let outputs = collect_declared_remote_worker_outputs(
            &execution_root,
            &payload.outputs,
            result.success,
        )?;
        stage_remote_worker_outputs(&artifact_root, &execution_root, &outputs)?;
        if result.success {
            persist_session_paths(execution_root_base, payload, &execution_root)?;
        }

        Ok((result, outputs))
    })();

    let cleanup_result = cleanup_execution_root(payload, &execution_root);

    match (execution_result, cleanup_result) {
        (Ok(value), Ok(())) => Ok(value),
        (Ok(value), Err(err)) => {
            tracing::warn!(
                "remote worker submit {idempotency_key} completed successfully but cleanup failed: {err:#}"
            );
            Ok(value)
        }
        (Err(err), Ok(())) => Err(err),
        (Err(err), Err(cleanup_err)) => Err(err.context(cleanup_err.to_string())),
    }
}

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
