use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use rusqlite::{Connection, params};

mod write;

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
        let store = Self {
            db_path: state_home()?.join("tak").join("tasks.sqlite"),
        };
        store.ensure_schema()?;
        Ok(store)
    }

    pub(super) fn list_runs(&self, limit: usize) -> Result<Vec<TaskHistoryRow>> {
        let conn = self.open_connection()?;
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
        let conn = self.open_connection()?;
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

    fn open_connection(&self) -> Result<Connection> {
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        Connection::open(&self.db_path)
            .with_context(|| format!("open task history db {}", self.db_path.display()))
    }

    fn ensure_schema(&self) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute_batch(include_str!("schema.sql"))?;
        Ok(())
    }

    fn run_exists(&self, task_run_id: &str) -> Result<bool> {
        let conn = self.open_connection()?;
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

fn unix_epoch_ms() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis() as i64,
        Err(_) => 0,
    }
}
