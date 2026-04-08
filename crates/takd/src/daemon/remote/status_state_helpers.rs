use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use sysinfo::Disks;
use tak_proto::{ActiveJob, AggregatedNeedUsage, StorageUsage};

use super::status_state::ActiveJobMetadata;

pub(super) fn active_job_value(job: &ActiveJobMetadata) -> Result<ActiveJob> {
    Ok(ActiveJob {
        task_run_id: job.task_run_id.clone(),
        attempt: job.attempt,
        task_label: job.task_label.clone(),
        started_at_ms: job.started_at_ms,
        needs: job.needs.clone(),
        execution_root_bytes: dir_size_bytes(&job.execution_root)?,
        runtime: job.runtime.clone(),
    })
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

fn dir_size_bytes(path: &Path) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }

    let metadata = fs::metadata(path).with_context(|| format!("inspect {}", path.display()))?;
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    if !metadata.is_dir() {
        return Ok(0);
    }

    let mut total = 0_u64;
    for entry in fs::read_dir(path).with_context(|| format!("read {}", path.display()))? {
        let entry = entry.with_context(|| format!("iterate {}", path.display()))?;
        total = total.saturating_add(dir_size_bytes(&entry.path())?);
    }
    Ok(total)
}
