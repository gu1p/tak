use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;

use super::*;

#[cfg(test)]
mod cleanup_janitor_permission_tests;
#[path = "cleanup_janitor/containers.rs"]
mod containers;
#[path = "cleanup_janitor/image_cache.rs"]
mod image_cache;
#[path = "cleanup_janitor/storage.rs"]
mod storage;
#[cfg(test)]
mod workspace_uploads_tests;

use storage::{cleanup_stale_remote_entries, cleanup_stale_workspace_uploads};

pub(crate) fn spawn_remote_cleanup_janitor(
    context: RemoteNodeContext,
    store: SubmitAttemptStore,
) -> Vec<tokio::task::JoinHandle<()>> {
    let mut handles = vec![spawn_remote_execution_cleanup_janitor(
        context.clone(),
        store,
    )];
    if let Some(handle) = spawn_remote_image_cache_janitor(context) {
        handles.push(handle);
    }
    handles
}

fn spawn_remote_execution_cleanup_janitor(
    context: RemoteNodeContext,
    store: SubmitAttemptStore,
) -> tokio::task::JoinHandle<()> {
    let interval = context.runtime_config().remote_cleanup_interval();
    tokio::spawn(async move {
        if let Err(err) = run_remote_cleanup_once(&context, &store).await {
            tracing::warn!("remote cleanup janitor startup sweep failed: {err:#}");
        }

        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            if let Err(err) = run_remote_cleanup_once(&context, &store).await {
                tracing::warn!("remote cleanup janitor sweep failed: {err:#}");
            }
        }
    })
}

fn spawn_remote_image_cache_janitor(
    context: RemoteNodeContext,
) -> Option<tokio::task::JoinHandle<()>> {
    let image_cache = context.image_cache_config()?;
    let interval = Duration::from_secs(image_cache.sweep_interval_secs.max(1));
    Some(tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            if let Err(err) = image_cache::run_remote_image_cache_cleanup_once(&context).await {
                tracing::warn!("image cache janitor sweep failed: {err:#}");
            }
        }
    }))
}

pub(crate) async fn run_remote_cleanup_once(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
) -> Result<()> {
    let active_jobs = cleanup_protected_job_keys(context)?;
    let ttl = context.runtime_config().remote_cleanup_ttl();
    for root in cleanup_roots(context, store)? {
        cleanup_stale_remote_entries(&root, &active_jobs, ttl)?;
        cleanup_stale_workspace_uploads(&root.join(WORKSPACE_UPLOADS_DIR_NAME), ttl)?;
    }
    if !tak_core::mock::mock_container_enabled() {
        containers::cleanup_inactive_takd_containers(context, &active_jobs).await?;
    }
    Ok(())
}

fn cleanup_protected_job_keys(context: &RemoteNodeContext) -> Result<BTreeSet<String>> {
    let mut keys = active_job_keys(&context.shared_status_state())?;
    for key in context.active_execution_keys()? {
        keys.insert(sanitize_submit_idempotency_key(&key));
    }
    Ok(keys)
}

fn active_job_keys(status_state: &status_state::SharedNodeStatusState) -> Result<BTreeSet<String>> {
    let guard = status_state
        .lock()
        .map_err(|_| anyhow!("node status state lock poisoned"))?;
    Ok(guard
        .active_job_keys()
        .into_iter()
        .map(|key| sanitize_submit_idempotency_key(&key))
        .collect())
}

fn cleanup_roots(context: &RemoteNodeContext, store: &SubmitAttemptStore) -> Result<Vec<PathBuf>> {
    let mut execution_roots = store.known_execution_root_bases()?;
    let current_root = remote_execution_root_base(context);
    if !execution_roots.contains(&current_root) {
        execution_roots.push(current_root);
    }

    let mut roots = Vec::with_capacity(execution_roots.len() * 2);
    for execution_root in execution_roots {
        if !roots.contains(&execution_root) {
            roots.push(execution_root.clone());
        }
        let artifact_root = artifact_root_base_for_execution_root_base(&execution_root);
        if !roots.contains(&artifact_root) {
            roots.push(artifact_root);
        }
    }
    Ok(roots)
}
