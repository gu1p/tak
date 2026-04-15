use std::env;
use std::path::{Path, PathBuf};

use tak_core::model::RemoteRuntimeSpec;

mod cache;
mod client;
mod podman;
mod probe;
mod simulation;

use cache::{
    RemoteExecRootCacheKey, current_remote_exec_root_cache_key,
    current_remote_execution_root_entry, update_remote_execution_root_entry,
};
use simulation::should_skip_probe;

use super::query_helpers::sanitize_submit_idempotency_key;

const REMOTE_EXEC_ROOT_DIR: &str = "takd-remote-exec";
const REMOTE_ARTIFACT_ROOT_DIR: &str = "takd-remote-artifacts";
const PROBE_IMAGE: &str = "alpine:3.20";
const PROBE_MOUNT: &str = "/tak-probe";
const PROBE_SENTINEL: &str = ".tak-mount-visible";

pub(super) fn remote_execution_root_base() -> PathBuf {
    current_remote_execution_root_entry().selected_root
}

pub(super) fn ensure_remote_execution_root_base(runtime: Option<&RemoteRuntimeSpec>) -> PathBuf {
    let key = current_remote_exec_root_cache_key();
    let entry = current_remote_execution_root_entry();
    let should_probe = key.explicit_root.is_none()
        && !entry.probed
        && !should_skip_probe()
        && matches!(runtime, Some(RemoteRuntimeSpec::Containerized { .. }));
    if !should_probe {
        return entry.selected_root;
    }

    let fallback_root = entry.selected_root;
    match probe::probe_remote_execution_root_candidates(&key) {
        Ok(root) => {
            update_remote_execution_root_entry(key, root.clone());
            root
        }
        Err(err) => {
            tracing::warn!(
                "failed to validate remote container execution root; falling back to {}: {err:#}",
                fallback_root.display()
            );
            fallback_root
        }
    }
}

pub(super) fn execution_root_for_submit_key_at_base(
    idempotency_key: &str,
    execution_root_base: &Path,
) -> PathBuf {
    execution_root_base.join(sanitize_submit_idempotency_key(idempotency_key))
}

pub(super) fn artifact_root_for_submit_key_at_base(
    idempotency_key: &str,
    execution_root_base: &Path,
) -> PathBuf {
    artifact_root_base_for_execution_root_base(execution_root_base)
        .join(sanitize_submit_idempotency_key(idempotency_key))
}

pub(super) fn artifact_root_base_for_execution_root_base(execution_root_base: &Path) -> PathBuf {
    execution_root_base
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(env::temp_dir)
        .join(REMOTE_ARTIFACT_ROOT_DIR)
}

fn explicit_remote_execution_root_base() -> Option<PathBuf> {
    env::var("TAKD_REMOTE_EXEC_ROOT")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn default_remote_execution_root_base(key: &RemoteExecRootCacheKey) -> PathBuf {
    if cfg!(unix) {
        return PathBuf::from("/var/tmp").join(REMOTE_EXEC_ROOT_DIR);
    }
    key.temp_dir.join(REMOTE_EXEC_ROOT_DIR)
}

fn candidate_remote_execution_root_bases(key: &RemoteExecRootCacheKey) -> Vec<PathBuf> {
    let mut candidates = vec![default_remote_execution_root_base(key)];
    let temp_candidate = key.temp_dir.join(REMOTE_EXEC_ROOT_DIR);
    if !candidates.contains(&temp_candidate) {
        candidates.push(temp_candidate);
    }
    candidates
}
