use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use sysinfo::{CpuRefreshKind, DiskRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};
use tak_proto::{CpuUsage, MemoryUsage, NodeInfo, NodeStatusResponse};

use super::query_helpers::unix_epoch_ms;
use super::resource_admission::ResourceRequest;
use super::status_resources::{
    cpu_admission_available, host_cpu_cores_used, memory_admission_available, non_tak_cpu_cores,
    non_tak_memory_bytes,
};
use super::status_state_helpers::{active_job_value, aggregate_need_usage, storage_usage};
use super::tak_container_usage::SharedTakContainerUsage;
use super::types::RemoteImageCacheRuntimeConfig;

pub(crate) use super::status_job_metadata::{ActiveJobMetadata, ActiveJobMetadataInput};

pub(crate) type SharedNodeStatusState = Arc<Mutex<NodeStatusState>>;

pub(crate) struct NodeStatusState {
    system: System,
    disks: Disks,
    cpu_usage_ready: bool,
    tak_container_usage: SharedTakContainerUsage,
    active_jobs: BTreeMap<String, ActiveJobMetadata>,
}

pub(crate) fn new_shared_node_status_state(
    tak_container_usage: SharedTakContainerUsage,
) -> SharedNodeStatusState {
    let mut system = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );
    system.refresh_memory();
    system.refresh_cpu_all();

    Arc::new(Mutex::new(NodeStatusState {
        system,
        disks: Disks::new_with_refreshed_list(),
        cpu_usage_ready: true,
        tak_container_usage,
        active_jobs: BTreeMap::new(),
    }))
}

impl NodeStatusState {
    pub(crate) fn register_job(&mut self, idempotency_key: String, job: ActiveJobMetadata) {
        self.active_jobs.insert(idempotency_key, job);
    }

    pub(crate) fn finish_job(&mut self, idempotency_key: &str) {
        self.active_jobs.remove(idempotency_key);
    }

    pub(crate) fn update_job_label(
        &mut self,
        idempotency_key: &str,
        task_label: &str,
        execution_label: Option<String>,
    ) {
        let Some(job) = self.active_jobs.get_mut(idempotency_key) else {
            return;
        };
        job.task_label = task_label.to_string();
        job.execution_label = execution_label;
    }

    pub(crate) fn active_job_keys(&self) -> Vec<String> {
        self.active_jobs.keys().cloned().collect()
    }

    pub(crate) fn snapshot(
        &mut self,
        node: &NodeInfo,
        execution_root_base: &std::path::Path,
        image_cache: Option<&RemoteImageCacheRuntimeConfig>,
        queued_jobs: Vec<ResourceRequest>,
    ) -> Result<NodeStatusResponse> {
        self.system.refresh_memory();
        self.system.refresh_cpu_usage();
        self.disks
            .refresh_specifics(false, DiskRefreshKind::everything());

        let mut active_jobs = self
            .active_jobs
            .values()
            .map(active_job_value)
            .collect::<Result<Vec<_>>>()?;
        active_jobs.sort_unstable_by(|left, right| {
            left.task_label
                .cmp(&right.task_label)
                .then(left.attempt.cmp(&right.attempt))
                .then(left.task_run_id.cmp(&right.task_run_id))
        });

        let tak_execution_bytes = active_jobs.iter().map(|job| job.execution_root_bytes).sum();
        let logical_cores = u32::try_from(self.system.cpus().len()).unwrap_or(u32::MAX);
        let tak_reserved_cores = tak_reserved_cores(&active_jobs);
        let tak_usage = self.tak_container_usage.latest();
        let host_cpu_used =
            host_cpu_cores_used(f64::from(self.system.global_cpu_usage()), logical_cores);
        let non_tak_cpu_used = non_tak_cpu_cores(host_cpu_used, tak_usage.cpu_cores);
        let cpu = CpuUsage {
            utilization_percent: if self.cpu_usage_ready {
                Some(f64::from(self.system.global_cpu_usage()))
            } else {
                None
            },
            logical_cores,
            non_tak_used_cores: if self.cpu_usage_ready {
                Some(non_tak_cpu_used)
            } else {
                None
            },
            tak_reserved_cores: Some(tak_reserved_cores),
            tak_admission_available_cores: if self.cpu_usage_ready {
                Some(cpu_admission_available(
                    logical_cores,
                    non_tak_cpu_used,
                    tak_reserved_cores,
                ))
            } else {
                None
            },
        };
        self.cpu_usage_ready = true;
        let memory_total_bytes = self.system.total_memory();
        let memory_available_bytes = self.system.available_memory();
        let host_used_bytes = memory_total_bytes.saturating_sub(memory_available_bytes);
        let non_tak_used_bytes = non_tak_memory_bytes(host_used_bytes, tak_usage.memory_bytes);
        let tak_reserved_bytes = tak_reserved_memory_bytes(&active_jobs);

        Ok(NodeStatusResponse {
            node: Some(node.clone()),
            sampled_at_ms: unix_epoch_ms(),
            cpu: Some(cpu),
            memory: Some(MemoryUsage {
                used_bytes: self.system.used_memory(),
                total_bytes: memory_total_bytes,
                available_bytes: Some(memory_available_bytes),
                non_tak_used_bytes: Some(non_tak_used_bytes),
                tak_reserved_bytes: Some(tak_reserved_bytes),
                tak_admission_available_bytes: Some(memory_admission_available(
                    memory_total_bytes,
                    non_tak_used_bytes,
                    tak_reserved_bytes,
                )),
            }),
            storage: Some(storage_usage(
                &self.disks,
                execution_root_base,
                tak_execution_bytes,
            )),
            allocated_needs: aggregate_need_usage(&active_jobs),
            active_jobs,
            image_cache: image_cache.and_then(|config| {
                tak_runner::image_cache_status(
                    &config.db_path,
                    config.budget_bytes,
                    config.low_disk_min_free_percent,
                    config.low_disk_min_free_bytes,
                )
                .ok()
            }),
            queued_jobs: queued_jobs
                .iter()
                .enumerate()
                .map(|(index, job)| super::status_state_helpers::queued_job_value(job, index + 1))
                .collect(),
        })
    }
}

fn tak_reserved_cores(active_jobs: &[tak_proto::ActiveJob]) -> f64 {
    active_jobs
        .iter()
        .filter_map(|job| job.resource_limits.as_ref())
        .map(|limits| limits.cpu_cores)
        .sum()
}

fn tak_reserved_memory_bytes(active_jobs: &[tak_proto::ActiveJob]) -> u64 {
    active_jobs
        .iter()
        .filter_map(|job| job.resource_limits.as_ref())
        .map(|limits| limits.memory_mb.saturating_mul(1024 * 1024))
        .sum()
}
