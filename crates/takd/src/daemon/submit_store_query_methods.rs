impl SubmitAttemptStore {
    /// Loads persisted submit events in ascending sequence order.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on local sqlite availability and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn events(&self, idempotency_key: &str) -> Result<Vec<SubmitEventRecord>> {
        let key = idempotency_key.trim();
        if key.is_empty() {
            bail!("idempotency_key is required");
        }
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "
            SELECT seq, payload_json
            FROM submit_events
            WHERE idempotency_key = ?1
            ORDER BY seq ASC
            ",
        )?;
        let rows = stmt.query_map(params![key], |row| {
            let seq = row.get::<_, i64>(0)?;
            let payload_json = row.get::<_, String>(1)?;
            Ok((seq, payload_json))
        })?;
        let mut events = Vec::new();
        for row in rows {
            let (seq, payload_json) = row?;
            events.push(SubmitEventRecord {
                seq: u64::try_from(seq)
                    .with_context(|| format!("invalid persisted submit event seq {seq}"))?,
                payload_json,
            });
        }
        Ok(events)
    }

    /// Loads the persisted terminal result payload, if any.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on local sqlite availability and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn result_payload(&self, idempotency_key: &str) -> Result<Option<String>> {
        let key = idempotency_key.trim();
        if key.is_empty() {
            bail!("idempotency_key is required");
        }
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "
            SELECT payload_json
            FROM submit_results
            WHERE idempotency_key = ?1
            ",
        )?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            let payload = row.get::<_, String>(0)?;
            Ok(Some(payload))
        } else {
            Ok(None)
        }
    }

    fn latest_submit_idempotency_key_for_task_run(
        &self,
        task_run_id: &str,
    ) -> Result<Option<String>> {
        let run_id = task_run_id.trim();
        if run_id.is_empty() {
            bail!("task_run_id is required");
        }
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "
            SELECT idempotency_key
            FROM submit_attempts
            WHERE task_run_id = ?1
            ORDER BY attempt DESC
            LIMIT 1
            ",
        )?;
        let mut rows = stmt.query(params![run_id])?;
        if let Some(row) = rows.next()? {
            let key = row.get::<_, String>(0)?;
            Ok(Some(key))
        } else {
            Ok(None)
        }
    }

    fn selected_node_id_for_submit(&self, idempotency_key: &str) -> Result<Option<String>> {
        let key = idempotency_key.trim();
        if key.is_empty() {
            bail!("idempotency_key is required");
        }
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "
            SELECT selected_node_id
            FROM submit_attempts
            WHERE idempotency_key = ?1
            LIMIT 1
            ",
        )?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            let node_id = row.get::<_, String>(0)?;
            Ok(Some(node_id))
        } else {
            Ok(None)
        }
    }

    fn open_connection(&self) -> Result<Connection> {
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

    fn ensure_schema(&self) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS submit_attempts (
                idempotency_key TEXT PRIMARY KEY,
                task_run_id TEXT NOT NULL,
                attempt INTEGER NOT NULL,
                selected_node_id TEXT NOT NULL,
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
        Ok(())
    }

    fn has_submit_attempt(&self, conn: &Connection, idempotency_key: &str) -> Result<bool> {
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

    fn ensure_submit_attempt_exists(&self, conn: &Connection, idempotency_key: &str) -> Result<()> {
        if self.has_submit_attempt(conn, idempotency_key)? {
            return Ok(());
        }
        bail!("submit attempt {idempotency_key} does not exist")
    }
}
