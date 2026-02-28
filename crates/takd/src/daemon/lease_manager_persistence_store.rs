impl LeaseManager {
    /// Upserts one active lease row in SQLite storage.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn persist_active_lease(&self, lease_id: &str, record: &LeaseRecord) -> Result<()> {
        let Some(conn) = self.open_connection()? else {
            return Ok(());
        };

        let needs_json = serde_json::to_string(&record.needs)?;
        let ttl_ms = i64::try_from(record.ttl_ms)
            .with_context(|| format!("ttl_ms {} exceeds sqlite range", record.ttl_ms))?;
        let expires_at_ms = unix_epoch_ms().checked_add(ttl_ms).ok_or_else(|| {
            anyhow!(
                "ttl_ms overflow while computing expires_at_ms for lease {}",
                lease_id
            )
        })?;
        conn.execute(
            "
            INSERT INTO active_leases (
                lease_id, request_id, task_label, user_name, pid, needs_json, ttl_ms, expires_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(lease_id) DO UPDATE SET
                request_id = excluded.request_id,
                task_label = excluded.task_label,
                user_name = excluded.user_name,
                pid = excluded.pid,
                needs_json = excluded.needs_json,
                ttl_ms = excluded.ttl_ms,
                expires_at_ms = excluded.expires_at_ms
            ",
            params![
                lease_id,
                record.request_id,
                record.task_label,
                record.user_name,
                record.pid,
                needs_json,
                ttl_ms,
                expires_at_ms
            ],
        )?;

        Ok(())
    }

    /// Removes one active lease row from SQLite storage.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn delete_active_lease(&self, lease_id: &str) -> Result<()> {
        let Some(conn) = self.open_connection()? else {
            return Ok(());
        };

        conn.execute("DELETE FROM active_leases WHERE lease_id = ?1", [lease_id])?;
        Ok(())
    }

    /// Appends one lease lifecycle event row to SQLite history.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn append_history(&self, event: &str, lease_id: &str, record: &LeaseRecord) -> Result<()> {
        let Some(conn) = self.open_connection()? else {
            return Ok(());
        };

        let payload_json = serde_json::json!({
            "needs": record.needs,
            "ttl_ms": record.ttl_ms,
        })
        .to_string();

        conn.execute(
            "
            INSERT INTO lease_history (
                ts_ms, event, lease_id, request_id, task_label, user_name, pid, payload_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ",
            params![
                unix_epoch_ms(),
                event,
                lease_id,
                record.request_id,
                record.task_label,
                record.user_name,
                record.pid,
                payload_json
            ],
        )?;

        Ok(())
    }

    /// Opens the configured SQLite connection if persistence is enabled.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn open_connection(&self) -> Result<Option<Connection>> {
        let Some(db_path) = &self.db_path else {
            return Ok(None);
        };

        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create sqlite directory {}", parent.display())
            })?;
        }

        let conn = Connection::open(db_path)
            .with_context(|| format!("failed to open sqlite db {}", db_path.display()))?;
        Ok(Some(conn))
    }
}
