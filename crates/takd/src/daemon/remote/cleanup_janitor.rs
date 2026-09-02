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
#[path = "cleanup_janitor/quarantine.rs"]
mod quarantine;
#[cfg(test)]
mod quarantine_tests;
#[path = "cleanup_janitor/storage.rs"]
mod storage;
#[cfg(test)]
mod storage_race_tests;
#[cfg(test)]
mod workspace_uploads_tests;

use storage::{
    cleanup_stale_remote_entries_with, cleanup_stale_workspace_uploads, remove_stale_remote_entry,
};

const CLEANUP_TOMBSTONE_PREFIX: &str = ".tak-cleanup@";
const WORKSPACE_UPLOADS_DIR_NAME: &str = ".workspace-uploads";

pub(crate) fn spawn_remote_cleanup_janitor(
    context: RemoteNodeContext,
    store: SubmitAttemptStore,
) -> Vec<tokio::task::JoinHandle<()>> {
    let mut handles = vec![
        spawn_worker_cache_cleanup_janitor(context.clone(), store.clone()),
        spawn_remote_execution_cleanup_janitor(context.clone(), store),
    ];
    if let Some(handle) = spawn_remote_image_cache_janitor(context) {
        handles.push(handle);
    }
    handles
}

fn spawn_worker_cache_cleanup_janitor(
    context: RemoteNodeContext,
    store: SubmitAttemptStore,
) -> tokio::task::JoinHandle<()> {
    let interval = context.runtime_config().remote_cleanup_interval();
    tokio::spawn(async move {
        if let Err(err) = worker_cache_gc::enforce(&context, &store) {
            tracing::warn!("worker cache janitor startup sweep failed: {err:#}");
        }

        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            if let Err(err) = worker_cache_gc::enforce(&context, &store) {
                tracing::warn!("worker cache janitor sweep failed: {err:#}");
            }
        }
    })
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
        cleanup_stale_remote_entries_with(&root, &active_jobs, ttl, |path| {
            remove_stale_remote_entry(path)
        })?;
        quarantine::cleanup_quarantined_remote_entries(&root)?;
        cleanup_stale_workspace_uploads(&root.join(WORKSPACE_UPLOADS_DIR_NAME), ttl)?;
    }
    if !tak_core::mock::mock_container_enabled() {
        containers::cleanup_inactive_takd_containers(context, &active_jobs).await?;
    }
    Ok(())
}

fn cleanup_protected_job_keys(context: &RemoteNodeContext) -> Result<BTreeSet<String>> {
    Ok(context
        .resource_admission()
        .reservation_keys()?
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
