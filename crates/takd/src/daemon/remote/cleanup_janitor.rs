use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};

use super::*;

const DEFAULT_REMOTE_CLEANUP_TTL_MS: u64 = 15 * 60 * 1000;
const DEFAULT_REMOTE_CLEANUP_INTERVAL_MS: u64 = 60 * 1000;

pub(crate) fn spawn_remote_cleanup_janitor(
    status_state: status_state::SharedNodeStatusState,
    store: SubmitAttemptStore,
) {
    let interval = remote_cleanup_interval();
    tokio::spawn(async move {
        if let Err(err) = run_remote_cleanup_once(&status_state, &store) {
            tracing::warn!("remote cleanup janitor startup sweep failed: {err:#}");
        }

        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            if let Err(err) = run_remote_cleanup_once(&status_state, &store) {
                tracing::warn!("remote cleanup janitor sweep failed: {err:#}");
            }
        }
    });
}

pub(crate) fn run_remote_cleanup_once(
    status_state: &status_state::SharedNodeStatusState,
    store: &SubmitAttemptStore,
) -> Result<()> {
    let active_jobs = active_job_keys(status_state)?;
    let ttl = remote_cleanup_ttl();
    for root in cleanup_roots(store)? {
        cleanup_stale_remote_entries(&root, &active_jobs, ttl)?;
    }
    Ok(())
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

fn cleanup_roots(store: &SubmitAttemptStore) -> Result<Vec<PathBuf>> {
    let mut execution_roots = store.known_execution_root_bases()?;
    let current_root = remote_execution_root_base();
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
        remove_stale_remote_entry(&path)?;
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

fn remote_cleanup_ttl() -> Duration {
    Duration::from_millis(env_duration_ms(
        "TAKD_REMOTE_CLEANUP_TTL_MS",
        DEFAULT_REMOTE_CLEANUP_TTL_MS,
    ))
}

fn remote_cleanup_interval() -> Duration {
    Duration::from_millis(env_duration_ms(
        "TAKD_REMOTE_CLEANUP_INTERVAL_MS",
        DEFAULT_REMOTE_CLEANUP_INTERVAL_MS,
    ))
}

fn env_duration_ms(name: &str, default_ms: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default_ms)
}
