use anyhow::{Result, anyhow};
use tak_proto::{
    NodePingResponse, NodeStatusResponse, ResourceEnvelopeStatus, ResourcePressureStatus,
};

use super::RemoteNodeContext;
use crate::daemon::remote::execution_root::remote_execution_root_base;
use crate::daemon::remote::status_state::{ActiveJobMetadata, SharedNodeStatusState};

mod summary;

use summary::compact_resource_summary;

impl RemoteNodeContext {
    pub(crate) fn register_active_job(
        &self,
        idempotency_key: String,
        job: ActiveJobMetadata,
    ) -> Result<()> {
        let mut guard = self
            .status_state
            .lock()
            .map_err(|_| anyhow!("node status state lock poisoned"))?;
        guard.register_job(idempotency_key, job);
        Ok(())
    }

    pub(crate) fn finish_active_job(&self, idempotency_key: &str) -> Result<()> {
        let mut guard = self
            .status_state
            .lock()
            .map_err(|_| anyhow!("node status state lock poisoned"))?;
        guard.finish_job(idempotency_key);
        Ok(())
    }

    pub(crate) fn update_active_job_label(
        &self,
        idempotency_key: &str,
        task_label: &str,
        execution_label: Option<String>,
    ) -> Result<()> {
        let mut guard = self
            .status_state
            .lock()
            .map_err(|_| anyhow!("node status state lock poisoned"))?;
        guard.update_job_label(idempotency_key, task_label, execution_label);
        Ok(())
    }

    pub(crate) fn node_status(&self) -> Result<NodeStatusResponse> {
        let node = self.node_info()?;
        let queued_jobs = self.resource_admission.queued_jobs()?;
        let mut guard = self
            .status_state
            .lock()
            .map_err(|_| anyhow!("node status state lock poisoned"))?;
        let mut status = guard.snapshot(
            &node,
            &remote_execution_root_base(self),
            self.image_cache.as_ref(),
            queued_jobs,
        )?;
        let (swap_total_bytes, swap_available_bytes) = guard.swap_status();
        drop(guard);

        let non_tak = super::super::resource_admission::ResourceCapacity {
            cpu_cores: status
                .cpu
                .as_ref()
                .and_then(|cpu| cpu.non_tak_used_cores)
                .unwrap_or(0.0),
            memory_mb: status
                .memory
                .as_ref()
                .and_then(|memory| memory.non_tak_used_bytes)
                .unwrap_or(0)
                .div_ceil(1024 * 1024),
        };
        let host_available_memory_mb = status
            .memory
            .as_ref()
            .and_then(|memory| memory.available_bytes)
            .unwrap_or(0)
            / 1024
            / 1024;
        let admission = self
            .resource_admission
            .resource_snapshot(non_tak, host_available_memory_mb)?;
        if let Some(cpu) = status.cpu.as_mut() {
            cpu.tak_admission_available_cores = Some(admission.admittable.cpu_cores);
        }
        if let Some(memory) = status.memory.as_mut() {
            memory.tak_admission_available_bytes =
                Some(admission.admittable.memory_mb.saturating_mul(1024 * 1024));
        }
        let envelope = self.resource_policy.envelope();
        status.resource_envelope = Some(ResourceEnvelopeStatus {
            host_cpu_total_cores: envelope.total.cpu_cores,
            reserve_cpu_cores: envelope.host_reserve.cpu_cores,
            workload_cpu_cores: envelope.workload.cpu_cores,
            tak_usage_cpu_cores: admission.actual.cpu_cores,
            non_tak_cpu_cores: non_tak.cpu_cores,
            reserved_cpu_cores: admission.reserved.cpu_cores + admission.pending_startup.cpu_cores,
            admittable_cpu_cores: admission.admittable.cpu_cores,
            host_memory_total_bytes: envelope.total.memory_mb.saturating_mul(1024 * 1024),
            reserve_memory_bytes: envelope.host_reserve.memory_mb.saturating_mul(1024 * 1024),
            workload_memory_bytes: envelope.workload.memory_mb.saturating_mul(1024 * 1024),
            tak_usage_memory_bytes: admission.actual.memory_mb.saturating_mul(1024 * 1024),
            non_tak_memory_bytes: non_tak.memory_mb.saturating_mul(1024 * 1024),
            reserved_memory_bytes: admission
                .reserved
                .memory_mb
                .saturating_add(admission.pending_startup.memory_mb)
                .saturating_mul(1024 * 1024),
            admittable_memory_bytes: admission.admittable.memory_mb.saturating_mul(1024 * 1024),
            swap_total_bytes,
            swap_available_bytes,
        });
        let pressure = self.resource_pressure_snapshot()?;
        status.resource_pressure = Some(ResourcePressureStatus {
            state: pressure.state_name().to_string(),
            episode_started_at_ms: pressure.episode_started_at_ms(),
            healthy_samples: u32::try_from(pressure.healthy_samples()).unwrap_or(u32::MAX),
        });
        Ok(status)
    }

    pub(crate) fn node_ping(&self) -> Result<NodePingResponse> {
        let status = self.node_status()?;
        let node = status
            .node
            .as_ref()
            .ok_or_else(|| anyhow!("node status did not include node metadata"))?;
        Ok(NodePingResponse {
            node_id: node.node_id.clone(),
            protocol_version: "v1".to_string(),
            health: ping_health(node),
            active_job_count: bounded_len(status.active_jobs.len()),
            queue_depth: bounded_len(status.queued_jobs.len()),
            resource_summary: compact_resource_summary(&status),
        })
    }

    pub(crate) fn shared_status_state(&self) -> SharedNodeStatusState {
        self.status_state.clone()
    }
}

fn ping_health(node: &tak_proto::NodeInfo) -> String {
    if node.healthy {
        "healthy".to_string()
    } else if node.transport_state.is_empty() {
        "unhealthy".to_string()
    } else {
        node.transport_state.clone()
    }
}

fn bounded_len(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests;
