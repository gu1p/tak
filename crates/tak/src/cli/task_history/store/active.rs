use anyhow::Result;

use super::{TaskHistoryStore, collect_rows};

pub(in crate::cli) struct ActiveContainerRow {
    pub(in crate::cli) task_run_id: String,
    pub(in crate::cli) task_label: String,
    pub(in crate::cli) attempts: u32,
    pub(in crate::cli) origin: String,
    pub(in crate::cli) runtime: String,
    pub(in crate::cli) runtime_source: String,
    pub(in crate::cli) command: String,
    pub(in crate::cli) started_at_ms: i64,
}

pub(in crate::cli) struct ActiveTaskRow {
    pub(in crate::cli) task_run_id: String,
    pub(in crate::cli) task_label: String,
    pub(in crate::cli) attempts: u32,
    pub(in crate::cli) placement: String,
    pub(in crate::cli) remote_node_id: String,
    pub(in crate::cli) origin: String,
    pub(in crate::cli) runtime: String,
    pub(in crate::cli) runtime_source: String,
    pub(in crate::cli) command: String,
    pub(in crate::cli) started_at_ms: i64,
}

impl TaskHistoryStore {
    pub(in crate::cli) fn active_container_runs(&self) -> Result<Vec<ActiveContainerRow>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "
            SELECT task_run_id, task_label, attempts, origin, runtime, runtime_source, command, started_at_ms
            FROM task_runs
            WHERE state = 'active'
              AND placement = 'local'
              AND runtime = 'containerized'
            ORDER BY started_at_ms DESC, task_run_id ASC
            ",
        )?;
        let rows = stmt.query_map([], |row| {
            let attempts = row.get::<_, i64>(2)?;
            Ok(ActiveContainerRow {
                task_run_id: row.get(0)?,
                task_label: row.get(1)?,
                attempts: u32::try_from(attempts).unwrap_or(u32::MAX),
                origin: row.get(3)?,
                runtime: row.get(4)?,
                runtime_source: row.get(5)?,
                command: row.get(6)?,
                started_at_ms: row.get(7)?,
            })
        })?;
        collect_rows(rows)
    }

    pub(in crate::cli) fn active_local_runs(&self) -> Result<Vec<ActiveTaskRow>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "
            SELECT task_run_id, task_label, attempts, placement, remote_node_id,
                   origin, runtime, runtime_source, command, started_at_ms
            FROM task_runs
            WHERE state = 'active'
              AND placement = 'local'
            ORDER BY started_at_ms DESC, task_run_id ASC
            ",
        )?;
        let rows = stmt.query_map([], |row| {
            let attempts = row.get::<_, i64>(2)?;
            Ok(ActiveTaskRow {
                task_run_id: row.get(0)?,
                task_label: row.get(1)?,
                attempts: u32::try_from(attempts).unwrap_or(u32::MAX),
                placement: row.get(3)?,
                remote_node_id: row.get(4)?,
                origin: row.get(5)?,
                runtime: row.get(6)?,
                runtime_source: row.get(7)?,
                command: row.get(8)?,
                started_at_ms: row.get(9)?,
            })
        })?;
        collect_rows(rows)
    }
}
