use super::*;

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

    pub(super) fn open_connection(&self) -> Result<Connection> {
        if let Some(parent) = self.db_path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create sqlite parent directory {:?}", parent)
            })?;
        }
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("failed to open sqlite db at {:?}", self.db_path))?;
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
