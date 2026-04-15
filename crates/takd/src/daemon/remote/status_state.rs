use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use sysinfo::{CpuRefreshKind, DiskRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};
use tak_proto::{CpuUsage, MemoryUsage, NodeInfo, NodeStatusResponse, SubmittedNeed};

use super::execution_root::remote_execution_root_base;
use super::query_helpers::unix_epoch_ms;
use super::status_state_helpers::{active_job_value, aggregate_need_usage, storage_usage};

#[derive(Clone)]
pub(crate) struct ActiveJobMetadata {
    pub(crate) task_run_id: String,
    pub(crate) attempt: u32,
    pub(crate) task_label: String,
    pub(crate) started_at_ms: i64,
    pub(crate) needs: Vec<SubmittedNeed>,
    pub(crate) runtime: Option<String>,
    pub(crate) execution_root: PathBuf,
}

impl ActiveJobMetadata {
    pub(crate) fn new(
        task_run_id: &str,
        attempt: u32,
        task_label: &str,
        needs: &[SubmittedNeed],
        runtime: Option<String>,
        execution_root: PathBuf,
    ) -> Self {
        Self {
            task_run_id: task_run_id.to_string(),
            attempt,
            task_label: task_label.to_string(),
            started_at_ms: unix_epoch_ms(),
            needs: needs.to_vec(),
            runtime,
            execution_root,
        }
    }
}

pub(crate) type SharedNodeStatusState = Arc<Mutex<NodeStatusState>>;

pub(crate) struct NodeStatusState {
    system: System,
    disks: Disks,
    cpu_usage_ready: bool,
    active_jobs: BTreeMap<String, ActiveJobMetadata>,
}

pub(crate) fn new_shared_node_status_state() -> SharedNodeStatusState {
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
        cpu_usage_ready: false,
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

    pub(crate) fn active_job_keys(&self) -> Vec<String> {
        self.active_jobs.keys().cloned().collect()
    }

    pub(crate) fn snapshot(&mut self, node: &NodeInfo) -> Result<NodeStatusResponse> {
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
        let cpu = CpuUsage {
            utilization_percent: if self.cpu_usage_ready {
                Some(f64::from(self.system.global_cpu_usage()))
            } else {
                None
            },
            logical_cores: u32::try_from(self.system.cpus().len()).unwrap_or(u32::MAX),
        };
        self.cpu_usage_ready = true;

        Ok(NodeStatusResponse {
            node: Some(node.clone()),
            sampled_at_ms: unix_epoch_ms(),
            cpu: Some(cpu),
            memory: Some(MemoryUsage {
                used_bytes: self.system.used_memory(),
                total_bytes: self.system.total_memory(),
            }),
            storage: Some(storage_usage(
                &self.disks,
                &remote_execution_root_base(),
                tak_execution_bytes,
            )),
            allocated_needs: aggregate_need_usage(&active_jobs),
            active_jobs,
        })
    }
}
