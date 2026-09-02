use std::collections::BTreeMap;
use std::path::Path;

use sysinfo::Disks;
use tak_proto::{ActiveJob, AggregatedNeedUsage, StorageUsage};

use super::resource_admission::{ResourceRequest, proto_resource_limits};

pub(super) fn active_job_value(job: &ResourceRequest) -> ActiveJob {
    ActiveJob {
        task_run_id: job.task_run_id.clone(),
        attempt: job.attempt,
        task_label: job.task_label.clone(),
        started_at_ms: job.queued_at_ms,
        needs: Vec::new(),
        execution_root_bytes: 0,
        runtime: job.runtime.clone(),
        origin: job.origin.clone(),
        runtime_source: job.runtime_source.clone(),
        command: job.command.clone(),
        resource_limits: proto_resource_limits(&job.resource_limits),
        execution_label: job.execution_label.clone(),
    }
}

pub(super) fn storage_usage(
    disks: &Disks,
    execution_root_base: &Path,
    tak_execution_bytes: u64,
) -> StorageUsage {
    let selected = disks
        .list()
        .iter()
        .filter(|disk| execution_root_base.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().display().to_string().len());

    if let Some(disk) = selected {
        let total_bytes = disk.total_space();
        let available_bytes = disk.available_space();
        return StorageUsage {
            path: disk.mount_point().display().to_string(),
            total_bytes,
            available_bytes,
            used_bytes: total_bytes.saturating_sub(available_bytes),
            tak_execution_bytes,
        };
    }

    StorageUsage {
        path: execution_root_base.display().to_string(),
        total_bytes: 0,
        available_bytes: 0,
        used_bytes: 0,
        tak_execution_bytes,
    }
}

pub(super) fn aggregate_need_usage(active_jobs: &[ActiveJob]) -> Vec<AggregatedNeedUsage> {
    let mut totals = BTreeMap::<(String, String, Option<String>), f64>::new();
    for job in active_jobs {
        for need in &job.needs {
            let key = (
                need.name.clone(),
                need.scope.clone(),
                need.scope_key.clone(),
            );
            *totals.entry(key).or_insert(0.0) += need.slots;
        }
    }

    totals
        .into_iter()
        .map(|((name, scope, scope_key), slots)| AggregatedNeedUsage {
            name,
            scope,
            scope_key,
            slots,
        })
        .collect()
}
