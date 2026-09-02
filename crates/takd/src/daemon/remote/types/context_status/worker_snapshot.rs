use anyhow::Result;
use tak_proto::worker_v2::{
    MAX_SNAPSHOT_BYTES, PROTOCOL_VERSION, WorkerResources, WorkerSnapshot,
    bounded_process_observations,
};

use super::RemoteNodeContext;
use crate::daemon::cache_locality::cached_path_content_keys;
use crate::daemon::remote::worker_v2_execution::cached_workspace_fingerprints;

impl RemoteNodeContext {
    pub(crate) fn worker_v2_snapshot(&self) -> Result<WorkerSnapshot> {
        let status = self.node_status()?;
        let node = status
            .node
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("node status did not include node metadata"))?;
        let envelope = status
            .resource_envelope
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("node status did not include resource capacity"))?;
        let admission = self.resource_admission.resource_snapshot()?;
        let cpu_capacity = cores_to_millis(envelope.workload_cpu_cores);
        let memory_capacity = envelope.workload_memory_bytes;
        let mut snapshot = WorkerSnapshot {
            protocol_version: PROTOCOL_VERSION,
            node_id: node.node_id.clone(),
            healthy: node.healthy,
            sampled_at_ms: u64::try_from(status.sampled_at_ms).unwrap_or_default(),
            capacity: WorkerResources {
                cpu_millis: cpu_capacity,
                memory_bytes: memory_capacity,
                execution_slots: admission.execution_capacity,
            },
            usage: WorkerResources {
                cpu_millis: cores_to_millis(admission.claimed.cpu_cores).min(cpu_capacity),
                memory_bytes: admission
                    .claimed
                    .memory_mb
                    .saturating_mul(1024 * 1024)
                    .min(memory_capacity),
                execution_slots: admission.execution_used,
            },
            queue_depth: u32::try_from(status.queued_jobs.len()).unwrap_or(u32::MAX),
            cached_content: cached_content(self),
            processes: Vec::new(),
        };
        let empty_size = serde_json::to_vec(&snapshot)?.len();
        let process_budget = MAX_SNAPSHOT_BYTES
            .saturating_sub(empty_size)
            .saturating_add(2);
        snapshot.processes = bounded_process_observations(
            crate::daemon::process_observation::current(),
            process_budget,
        );
        Ok(snapshot)
    }
}

fn cached_content(context: &RemoteNodeContext) -> Vec<String> {
    let mut keys = Vec::new();
    extend_cache_keys(
        &mut keys,
        "workspace",
        cached_workspace_fingerprints(context),
    );
    if let Some(config) = context.image_cache_config() {
        extend_cache_keys(
            &mut keys,
            "image",
            tak_runner::cached_image_content_keys(&config.db_path),
        );
    }
    if let Some(state_root) = context.state_root() {
        extend_cache_keys(&mut keys, "path", cached_path_content_keys(&state_root));
    }
    keys.sort();
    keys.dedup();
    keys
}

fn extend_cache_keys(keys: &mut Vec<String>, source: &str, observed: Result<Vec<String>>) {
    match observed {
        Ok(observed) => keys.extend(observed),
        Err(error) => tracing::warn!(
            cache_source = source,
            error = %error,
            "failed to observe advisory worker cache locality"
        ),
    }
}

fn cores_to_millis(cores: f64) -> u64 {
    if !cores.is_finite() || cores <= 0.0 {
        return 0;
    }
    (cores * 1_000.0).min(u64::MAX as f64) as u64
}
