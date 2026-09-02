use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use rusqlite::{OpenFlags, params};
use tak_exec::ProcessSqliteConnection;

mod active;

pub(in crate::cli) use active::ActiveTaskRow;

#[derive(Clone)]
pub(in crate::cli) struct TaskHistoryStore {
    db_path: PathBuf,
}

pub(super) struct TaskHistoryRow {
    pub(super) task_run_id: String,
    pub(super) task_label: String,
    pub(super) attempts: u32,
    pub(super) state: String,
    pub(super) placement: String,
    pub(super) remote_node_id: String,
}

pub(super) struct TaskOutputRow {
    pub(super) stream: String,
    pub(super) bytes: Vec<u8>,
}

impl TaskHistoryStore {
    pub(in crate::cli) fn open_default() -> Result<Self> {
        Ok(Self {
            db_path: default_db_path()?,
        })
    }

    pub(super) fn list_runs(&self, limit: usize) -> Result<Vec<TaskHistoryRow>> {
        let Some(conn) = self.open_read_connection()? else {
            return Ok(Vec::new());
        };
        let mut stmt = conn.prepare(
            "
            SELECT task_run_id, task_label, attempts, state, placement, remote_node_id
            FROM task_runs
            ORDER BY started_at_ms DESC, task_run_id ASC
            LIMIT ?1
            ",
        )?;
        let rows = stmt.query_map(params![i64::try_from(limit).unwrap_or(i64::MAX)], |row| {
            let attempts = row.get::<_, i64>(2)?;
            Ok(TaskHistoryRow {
                task_run_id: row.get(0)?,
                task_label: row.get(1)?,
                attempts: u32::try_from(attempts).unwrap_or(u32::MAX),
                state: row.get(3)?,
                placement: row.get(4)?,
                remote_node_id: row.get(5)?,
            })
        })?;
        collect_rows(rows)
    }

    pub(super) fn output_rows(&self, task_run_id: &str) -> Result<Vec<TaskOutputRow>> {
        if !self.run_exists(task_run_id)? {
            bail!("task_run_id {task_run_id} not found in local task history");
        }
        let Some(conn) = self.open_read_connection()? else {
            bail!("task_run_id {task_run_id} not found in local task history");
        };
        let mut stmt = conn.prepare(
            "
            SELECT stream, bytes
            FROM task_outputs
            WHERE task_run_id = ?1
            ORDER BY seq ASC
            ",
        )?;
        let rows = stmt.query_map(params![task_run_id.trim()], |row| {
            Ok(TaskOutputRow {
                stream: row.get(0)?,
                bytes: row.get(1)?,
            })
        })?;
        collect_rows(rows)
    }

    fn open_read_connection(&self) -> Result<Option<ProcessSqliteConnection>> {
        if !self.db_path.exists() {
            return Ok(None);
        }
        let conn = ProcessSqliteConnection::open_with_flags(
            &self.db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .with_context(|| format!("open task history db {}", self.db_path.display()))?;
        conn.busy_timeout(Duration::from_secs(5))
            .context("configure task history sqlite busy timeout")?;
        Ok(Some(conn))
    }

    fn run_exists(&self, task_run_id: &str) -> Result<bool> {
        let Some(conn) = self.open_read_connection()? else {
            return Ok(false);
        };
        let mut stmt = conn.prepare("SELECT 1 FROM task_runs WHERE task_run_id = ?1 LIMIT 1")?;
        let mut rows = stmt.query(params![task_run_id.trim()])?;
        Ok(rows.next()?.is_some())
    }
}

fn collect_rows<T>(rows: impl Iterator<Item = rusqlite::Result<T>>) -> Result<Vec<T>> {
    let mut output = Vec::new();
    for row in rows {
        output.push(row?);
    }
    Ok(output)
}

fn state_home() -> Result<PathBuf> {
    std::env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .map_err(|_| anyhow!("failed to resolve xdg_state_home"))
}

fn default_db_path() -> Result<PathBuf> {
    Ok(state_home()?.join("tak").join("tasks.sqlite"))
}
