use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};

use super::*;

#[cfg(test)]
mod cleanup_janitor_permission_tests;

pub(crate) fn spawn_remote_cleanup_janitor(context: RemoteNodeContext, store: SubmitAttemptStore) {
    spawn_remote_execution_cleanup_janitor(context.clone(), store);
    spawn_remote_image_cache_janitor(context);
}

fn spawn_remote_execution_cleanup_janitor(context: RemoteNodeContext, store: SubmitAttemptStore) {
    let interval = context.runtime_config().remote_cleanup_interval();
    tokio::spawn(async move {
        if let Err(err) = run_remote_cleanup_once(&context, &store) {
            tracing::warn!("remote cleanup janitor startup sweep failed: {err:#}");
        }

        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            if let Err(err) = run_remote_cleanup_once(&context, &store) {
                tracing::warn!("remote cleanup janitor sweep failed: {err:#}");
            }
        }
    });
}

fn spawn_remote_image_cache_janitor(context: RemoteNodeContext) {
    let Some(image_cache) = context.image_cache_config() else {
        return;
    };
    let interval = Duration::from_secs(image_cache.sweep_interval_secs.max(1));
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            if let Err(err) = run_remote_image_cache_cleanup_once(&context).await {
                tracing::warn!("image cache janitor sweep failed: {err:#}");
            }
        }
    });
}

pub(crate) fn run_remote_cleanup_once(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
) -> Result<()> {
    let active_jobs = active_job_keys(&context.shared_status_state())?;
    let ttl = context.runtime_config().remote_cleanup_ttl();
    for root in cleanup_roots(context, store)? {
        cleanup_stale_remote_entries(&root, &active_jobs, ttl)?;
    }
    Ok(())
}

async fn run_remote_image_cache_cleanup_once(context: &RemoteNodeContext) -> Result<()> {
    let Some(image_cache) = context.image_cache_config() else {
        return Ok(());
    };
    if !active_job_keys(&context.shared_status_state())?.is_empty() {
        return Ok(());
    }
    tak_runner::run_image_cache_janitor_once(&image_cache_options(image_cache)).await
}

fn image_cache_options(config: RemoteImageCacheRuntimeConfig) -> tak_runner::ImageCacheOptions {
    tak_runner::ImageCacheOptions {
        db_path: config.db_path,
        budget_bytes: config.budget_bytes,
        mutable_tag_ttl_secs: config.mutable_tag_ttl_secs,
        sweep_interval_secs: config.sweep_interval_secs,
        low_disk_min_free_percent: config.low_disk_min_free_percent,
        low_disk_min_free_bytes: config.low_disk_min_free_bytes,
    }
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

fn cleanup_stale_remote_entries(
    root: &Path,
    active_jobs: &BTreeSet<String>,
    ttl: Duration,
) -> Result<()> {
    cleanup_stale_remote_entries_with(root, active_jobs, ttl, remove_stale_remote_entry)
}

fn cleanup_stale_remote_entries_with<F>(
    root: &Path,
    active_jobs: &BTreeSet<String>,
    ttl: Duration,
    mut remove_stale: F,
) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    let read_dir = match std::fs::read_dir(root) {
        Ok(read_dir) => read_dir,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(err)
                .with_context(|| format!("failed to read cleanup root {}", root.display()));
        }
    };

    for entry in read_dir {
        let entry = entry
            .with_context(|| format!("failed to read cleanup entry under {}", root.display()))?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if active_jobs.contains(name) || !is_stale(&path, ttl)? {
            continue;
        }
        if let Err(err) = remove_stale(&path) {
            if is_permission_denied(&err) {
                tracing::warn!(
                    "remote cleanup janitor skipped stale entry {}: {err:#}",
                    path.display()
                );
                continue;
            }
            return Err(err);
        }
    }

    Ok(())
}

fn is_stale(path: &Path, ttl: Duration) -> Result<bool> {
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("failed to stat cleanup path {}", path.display()))?;
    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or_else(|_| Duration::from_secs(0));
    Ok(age >= ttl)
}

fn remove_stale_remote_entry(path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("failed to stat stale cleanup path {}", path.display()))?;
    let file_type = metadata.file_type();
    if file_type.is_dir() && !file_type.is_symlink() {
        std::fs::remove_dir_all(path)
            .with_context(|| format!("failed to remove stale directory {}", path.display()))?;
    } else {
        std::fs::remove_file(path)
            .with_context(|| format!("failed to remove stale file {}", path.display()))?;
    }
    Ok(())
}

fn is_permission_denied(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|err| err.kind() == ErrorKind::PermissionDenied)
    })
}
