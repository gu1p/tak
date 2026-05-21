use anyhow::Result;
use sysinfo::{CpuRefreshKind, DiskRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

use super::daemon::{LocalDaemonStatus, local_daemon_status};
use crate::cli::task_history::TaskHistoryStore;

#[derive(Clone, Debug)]
pub(super) struct LocalStatusSnapshot {
    pub(super) resources: LocalResources,
    pub(super) daemon: LocalDaemonStatus,
    pub(super) history: LocalHistoryStatus,
    pub(super) active_tasks: Vec<LocalTask>,
}

#[derive(Clone, Debug)]
pub(super) enum LocalHistoryStatus {
    Ok,
    Unavailable { detail: String },
}

#[derive(Clone, Debug)]
pub(super) struct LocalResources {
    pub(super) cpu_percent: Option<f64>,
    pub(super) logical_cores: usize,
    pub(super) memory_used_bytes: u64,
    pub(super) memory_total_bytes: u64,
    pub(super) storage_used_bytes: u64,
    pub(super) storage_total_bytes: u64,
    pub(super) storage_available_bytes: u64,
}

#[derive(Clone, Debug)]
pub(super) struct LocalTask {
    pub(super) task_run_id: String,
    pub(super) task_label: String,
    pub(super) attempt: u32,
    pub(super) placement: String,
    pub(super) remote_node_id: String,
    pub(super) origin: String,
    pub(super) runtime: String,
    pub(super) runtime_source: String,
    pub(super) command: String,
    pub(super) started_at_ms: i64,
}

pub(super) async fn local_status_snapshot() -> Result<LocalStatusSnapshot> {
    let (history, active_tasks) = local_history_tasks();
    Ok(LocalStatusSnapshot {
        resources: sample_local_resources(),
        daemon: local_daemon_status().await,
        history,
        active_tasks,
    })
}

impl LocalStatusSnapshot {
    pub(super) fn container_tasks(&self) -> Vec<&LocalTask> {
        self.active_tasks
            .iter()
            .filter(|task| task.runtime == "containerized")
            .collect()
    }
}

fn local_history_tasks() -> (LocalHistoryStatus, Vec<LocalTask>) {
    match TaskHistoryStore::open_default().and_then(|store| store.active_local_runs()) {
        Ok(rows) => (
            LocalHistoryStatus::Ok,
            rows.into_iter().map(LocalTask::from).collect(),
        ),
        Err(err) => (
            LocalHistoryStatus::Unavailable {
                detail: single_line(&format!("{err:#}")),
            },
            Vec::new(),
        ),
    }
}

impl From<crate::cli::task_history::ActiveTaskRow> for LocalTask {
    fn from(row: crate::cli::task_history::ActiveTaskRow) -> Self {
        Self {
            task_run_id: row.task_run_id,
            task_label: row.task_label,
            attempt: row.attempts,
            placement: row.placement,
            remote_node_id: row.remote_node_id,
            origin: row.origin,
            runtime: row.runtime,
            runtime_source: row.runtime_source,
            command: row.command,
            started_at_ms: row.started_at_ms,
        }
    }
}

fn sample_local_resources() -> LocalResources {
    let mut system = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );
    system.refresh_memory();
    system.refresh_cpu_all();

    let storage = local_storage_usage();
    LocalResources {
        cpu_percent: Some(f64::from(system.global_cpu_usage())),
        logical_cores: system.cpus().len(),
        memory_used_bytes: system.used_memory(),
        memory_total_bytes: system.total_memory(),
        storage_used_bytes: storage.used_bytes,
        storage_total_bytes: storage.total_bytes,
        storage_available_bytes: storage.available_bytes,
    }
}

fn local_storage_usage() -> LocalStorageUsage {
    let path = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut disks = Disks::new_with_refreshed_list();
    disks.refresh_specifics(false, DiskRefreshKind::everything());
    let selected = disks
        .list()
        .iter()
        .filter(|disk| path.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().display().to_string().len());

    if let Some(disk) = selected {
        let total_bytes = disk.total_space();
        let available_bytes = disk.available_space();
        return LocalStorageUsage {
            total_bytes,
            available_bytes,
            used_bytes: total_bytes.saturating_sub(available_bytes),
        };
    }

    LocalStorageUsage {
        total_bytes: 0,
        available_bytes: 0,
        used_bytes: 0,
    }
}

fn single_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

struct LocalStorageUsage {
    total_bytes: u64,
    available_bytes: u64,
    used_bytes: u64,
}
