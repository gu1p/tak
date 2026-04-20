use std::env;
use std::path::{Path, PathBuf};

use tak_core::model::RemoteRuntimeSpec;

mod client;
mod podman;
mod probe;
mod probe_image;
mod simulation;

use super::RemoteRuntimeConfig;
use super::types::RemoteNodeContext;
use simulation::should_skip_probe;

use super::query_helpers::sanitize_submit_idempotency_key;

const REMOTE_EXEC_ROOT_DIR: &str = "takd-remote-exec";
const REMOTE_ARTIFACT_ROOT_DIR: &str = "takd-remote-artifacts";
const PROBE_IMAGE_X86_64: &str = "takd-exec-root-probe:x86_64-v1";
const PROBE_IMAGE_AARCH64: &str = "takd-exec-root-probe:aarch64-v1";
const PROBE_FALLBACK_IMAGE: &str = "alpine:3.20";
const PROBE_HELPER_BINARY: &str = "/tak-probe-busybox";
const PROBE_MOUNT: &str = "/tak-probe";
const PROBE_SENTINEL: &str = ".tak-mount-visible";

pub(super) fn remote_execution_root_base(context: &RemoteNodeContext) -> PathBuf {
    context
        .runtime_state()
        .execution_root_selection()
        .selected_root
}

pub(super) fn ensure_remote_execution_root_base(
    context: &RemoteNodeContext,
    runtime: Option<&RemoteRuntimeSpec>,
) -> PathBuf {
    let runtime_state = context.runtime_state();
    let config = &runtime_state.config;
    let entry = runtime_state.execution_root_selection();
    let should_probe = config.explicit_remote_exec_root().is_none()
        && !entry.probed
        && !should_skip_probe(config)
        && matches!(runtime, Some(RemoteRuntimeSpec::Containerized { .. }));
    if !should_probe {
        return entry.selected_root;
    }

    let fallback_root = entry.selected_root;
    match probe::probe_remote_execution_root_candidates(config) {
        Ok(root) => {
            runtime_state.update_execution_root_selection(root.clone());
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

pub(super) fn default_remote_execution_root_base(config: &RemoteRuntimeConfig) -> PathBuf {
    config.default_remote_execution_root_base()
}

fn candidate_remote_execution_root_bases(config: &RemoteRuntimeConfig) -> Vec<PathBuf> {
    let mut candidates = vec![default_remote_execution_root_base(config)];
    let temp_candidate = config.temp_dir().join(REMOTE_EXEC_ROOT_DIR);
    if !candidates.contains(&temp_candidate) {
        candidates.push(temp_candidate);
    }
    candidates
}
