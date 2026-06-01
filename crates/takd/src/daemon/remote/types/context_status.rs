use anyhow::{Result, anyhow};
use tak_proto::{NodePingResponse, NodeStatusResponse};

use super::RemoteNodeContext;
use crate::daemon::remote::execution_root::remote_execution_root_base;
use crate::daemon::remote::status_state::{ActiveJobMetadata, SharedNodeStatusState};

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
        guard.snapshot(
            &node,
            &remote_execution_root_base(self),
            self.image_cache.as_ref(),
            queued_jobs,
        )
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

fn compact_resource_summary(status: &NodeStatusResponse) -> String {
    let cpu = status
        .cpu
        .as_ref()
        .map(cpu_summary)
        .unwrap_or_else(|| "cpu=unknown".to_string());
    let memory = status
        .memory
        .as_ref()
        .map(memory_summary)
        .unwrap_or_else(|| "memory=unknown".to_string());
    let storage = status
        .storage
        .as_ref()
        .map(storage_summary)
        .unwrap_or_else(|| "storage=unknown".to_string());
    format!("{cpu} {memory} {storage}")
}

fn cpu_summary(cpu: &tak_proto::CpuUsage) -> String {
    let total = f64::from(cpu.logical_cores);
    let available = cpu
        .tak_admission_available_cores
        .map(|available| format!("cpu_available={available:.2}"))
        .unwrap_or_else(|| "cpu_available=unknown".to_string());
    format!("{available} cpu_total={total:.2}")
}

fn memory_summary(memory: &tak_proto::MemoryUsage) -> String {
    let total_mb = memory.total_bytes / 1024 / 1024;
    let available = memory
        .tak_admission_available_bytes
        .map(|available| format!("memory_available_mb={}", available / 1024 / 1024))
        .unwrap_or_else(|| "memory_available_mb=unknown".to_string());
    format!("{available} memory_total_mb={total_mb}")
}

fn storage_summary(storage: &tak_proto::StorageUsage) -> String {
    format!(
        "storage_available_mb={}",
        storage.available_bytes / 1024 / 1024
    )
}

#[cfg(test)]
mod tests;
