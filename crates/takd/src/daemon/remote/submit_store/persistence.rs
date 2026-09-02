use super::*;
use std::time::Duration;

use tak_runner::ProcessSqliteConnection;

impl SubmitAttemptStore {
    /// Creates a SQLite-backed submit idempotency store and ensures schema is present.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on local sqlite availability and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn with_db_path(db_path: PathBuf) -> Result<Self> {
        let store = Self { db_path };
        store.ensure_schema()?;
        Ok(store)
    }

    pub(super) fn open_connection(&self) -> Result<ProcessSqliteConnection> {
        if let Some(parent) = self.db_path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create sqlite parent directory {:?}", parent)
            })?;
            secure_path(parent, 0o700)?;
        }
        let conn = ProcessSqliteConnection::open(&self.db_path)
            .with_context(|| format!("failed to open sqlite db at {:?}", self.db_path))?;
        secure_path(&self.db_path, 0o600)?;
        conn.busy_timeout(Duration::from_secs(5))
            .context("configure sqlite busy timeout")?;
        conn.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
        )?;
        secure_sqlite_sidecars(&self.db_path)?;
        Ok(conn)
    }

    pub(super) fn ensure_schema(&self) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS submit_attempts (
                idempotency_key TEXT PRIMARY KEY,
                task_run_id TEXT NOT NULL,
                attempt INTEGER NOT NULL,
                task_label TEXT NOT NULL DEFAULT '',
                execution_label TEXT NOT NULL DEFAULT '',
                selected_node_id TEXT NOT NULL,
                execution_root_base TEXT NOT NULL DEFAULT '',
                created_at_ms INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_submit_attempts_run_attempt
            ON submit_attempts(task_run_id, attempt);

            CREATE TABLE IF NOT EXISTS submit_events (
                idempotency_key TEXT NOT NULL,
                seq INTEGER NOT NULL,
                payload_json TEXT NOT NULL,
                PRIMARY KEY (idempotency_key, seq),
                FOREIGN KEY (idempotency_key) REFERENCES submit_attempts(idempotency_key)
            );

            CREATE TABLE IF NOT EXISTS submit_results (
                idempotency_key TEXT PRIMARY KEY,
                payload_json TEXT NOT NULL,
                FOREIGN KEY (idempotency_key) REFERENCES submit_attempts(idempotency_key)
            );
            ",
        )?;
        if !self.table_has_column(&conn, "submit_attempts", "execution_root_base")? {
            conn.execute_batch(
                "
                ALTER TABLE submit_attempts
                ADD COLUMN execution_root_base TEXT NOT NULL DEFAULT '';
                ",
            )?;
        }
        if !self.table_has_column(&conn, "submit_attempts", "task_label")? {
            conn.execute_batch(
                "
                ALTER TABLE submit_attempts
                ADD COLUMN task_label TEXT NOT NULL DEFAULT '';
                ",
            )?;
        }
        if !self.table_has_column(&conn, "submit_attempts", "execution_label")? {
            conn.execute_batch(
                "
                ALTER TABLE submit_attempts
                ADD COLUMN execution_label TEXT NOT NULL DEFAULT '';
                ",
            )?;
        }
        super::worker_v2::ensure_schema(&conn)?;
        Ok(())
    }

    pub(super) fn has_submit_attempt(
        &self,
        conn: &Connection,
        idempotency_key: &str,
    ) -> Result<bool> {
        let mut stmt = conn.prepare(
            "
            SELECT 1
            FROM submit_attempts
            WHERE idempotency_key = ?1
            LIMIT 1
            ",
        )?;
        let mut rows = stmt.query(params![idempotency_key])?;
        Ok(rows.next()?.is_some())
    }

    pub(super) fn ensure_submit_attempt_exists(
        &self,
        conn: &Connection,
        idempotency_key: &str,
    ) -> Result<()> {
        if self.has_submit_attempt(conn, idempotency_key)? {
            return Ok(());
        }
        bail!("submit attempt {idempotency_key} does not exist")
    }

    fn table_has_column(&self, conn: &Connection, table: &str, column: &str) -> Result<bool> {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
        for row in rows {
            if row?.trim() == column {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(unix)]
fn secure_path(path: &std::path::Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .with_context(|| format!("secure sqlite path {}", path.display()))
}

#[cfg(not(unix))]
fn secure_path(_path: &std::path::Path, _mode: u32) -> Result<()> {
    Ok(())
}

fn secure_sqlite_sidecars(db_path: &std::path::Path) -> Result<()> {
    for suffix in ["-wal", "-shm"] {
        let sidecar = std::path::PathBuf::from(format!("{}{suffix}", db_path.display()));
        if sidecar.exists() {
            secure_path(&sidecar, 0o600)?;
        }
    }
    Ok(())
}
