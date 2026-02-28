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

    /// Registers a submit attempt by `(task_run_id, attempt)` and returns whether it was created or attached.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on local sqlite availability and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn register_submit(
        &self,
        task_run_id: &str,
        attempt: Option<u32>,
        selected_node_id: &str,
    ) -> Result<SubmitRegistration> {
        let selected_node_id = selected_node_id.trim();
        if selected_node_id.is_empty() {
            bail!("selected_node_id is required");
        }

        let idempotency_key = build_submit_idempotency_key(task_run_id, attempt)?;
        let conn = self.open_connection()?;
        let inserted = conn.execute(
            "
            INSERT INTO submit_attempts (
                idempotency_key, task_run_id, attempt, selected_node_id, created_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            params![
                idempotency_key,
                task_run_id.trim(),
                attempt.expect("validated by build_submit_idempotency_key"),
                selected_node_id,
                unix_epoch_ms(),
            ],
        );

        match inserted {
            Ok(_) => Ok(SubmitRegistration::Created { idempotency_key }),
            Err(err) if is_submit_unique_violation(&err) => {
                if !self.has_submit_attempt(&conn, &idempotency_key)? {
                    bail!(
                        "submit idempotency key {} reported duplicate but no row was found",
                        idempotency_key
                    );
                }
                Ok(SubmitRegistration::Attached { idempotency_key })
            }
            Err(err) => Err(err.into()),
        }
    }

    /// Persists one idempotent event for an existing submit attempt.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on local sqlite availability and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn append_event(&self, idempotency_key: &str, seq: u64, payload_json: &str) -> Result<()> {
        let key = idempotency_key.trim();
        if key.is_empty() {
            bail!("idempotency_key is required");
        }
        if payload_json.trim().is_empty() {
            bail!("event payload_json is required");
        }
        let seq =
            i64::try_from(seq).with_context(|| format!("event seq {seq} exceeds sqlite range"))?;
        let conn = self.open_connection()?;
        self.ensure_submit_attempt_exists(&conn, key)?;
        conn.execute(
            "
            INSERT OR IGNORE INTO submit_events (idempotency_key, seq, payload_json)
            VALUES (?1, ?2, ?3)
            ",
            params![key, seq, payload_json],
        )?;
        Ok(())
    }

    /// Persists terminal result payload for an existing submit attempt.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on local sqlite availability and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn set_result_payload(&self, idempotency_key: &str, payload_json: &str) -> Result<()> {
        let key = idempotency_key.trim();
        if key.is_empty() {
            bail!("idempotency_key is required");
        }
        if payload_json.trim().is_empty() {
            bail!("result payload_json is required");
        }
        let conn = self.open_connection()?;
        self.ensure_submit_attempt_exists(&conn, key)?;
        conn.execute(
            "
            INSERT INTO submit_results (idempotency_key, payload_json)
            VALUES (?1, ?2)
            ON CONFLICT(idempotency_key) DO UPDATE SET payload_json=excluded.payload_json
            ",
            params![key, payload_json],
        )?;
        Ok(())
    }
}
