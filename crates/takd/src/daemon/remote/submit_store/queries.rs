use super::*;

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

    pub(in crate::daemon::remote) fn latest_submit_idempotency_key_for_task_run(
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

    pub(in crate::daemon::remote) fn selected_node_id_for_submit(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<String>> {
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

    pub(in crate::daemon::remote) fn execution_root_base_for_submit(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<PathBuf>> {
        let key = idempotency_key.trim();
        if key.is_empty() {
            bail!("idempotency_key is required");
        }
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "
            SELECT execution_root_base
            FROM submit_attempts
            WHERE idempotency_key = ?1
            LIMIT 1
            ",
        )?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            let root = row.get::<_, String>(0)?;
            let root = root.trim();
            if root.is_empty() {
                return Ok(None);
            }
            Ok(Some(PathBuf::from(root)))
        } else {
            Ok(None)
        }
    }

    pub(in crate::daemon::remote) fn known_execution_root_bases(&self) -> Result<Vec<PathBuf>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "
            SELECT DISTINCT execution_root_base
            FROM submit_attempts
            WHERE execution_root_base != ''
            ",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut roots = Vec::new();
        for row in rows {
            let root = row?;
            let root = root.trim();
            if root.is_empty() {
                continue;
            }
            let path = PathBuf::from(root);
            if !roots.contains(&path) {
                roots.push(path);
            }
        }
        Ok(roots)
    }
}
