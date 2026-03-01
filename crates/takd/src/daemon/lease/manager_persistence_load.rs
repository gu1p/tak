use super::*;

impl LeaseManager {
    /// Ensures SQLite schema exists for active leases and lease history.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub(super) fn ensure_schema(&self) -> Result<()> {
        let Some(mut conn) = self.open_connection()? else {
            return Ok(());
        };

        let tx = conn.transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS active_leases (
                lease_id TEXT PRIMARY KEY,
                request_id TEXT NOT NULL,
                task_label TEXT NOT NULL,
                user_name TEXT NOT NULL,
                pid INTEGER NOT NULL,
                needs_json TEXT NOT NULL,
                ttl_ms INTEGER NOT NULL,
                expires_at_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS lease_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ts_ms INTEGER NOT NULL,
                event TEXT NOT NULL,
                lease_id TEXT NOT NULL,
                request_id TEXT NOT NULL,
                task_label TEXT NOT NULL,
                user_name TEXT NOT NULL,
                pid INTEGER NOT NULL,
                payload_json TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_lease_history_lease_id ON lease_history(lease_id);
            CREATE INDEX IF NOT EXISTS idx_lease_history_event ON lease_history(event);
            ",
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Restores non-expired active leases from SQLite into in-memory state.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub(super) fn restore_active_leases(&mut self) -> Result<()> {
        let Some(conn) = self.open_connection()? else {
            return Ok(());
        };

        let now_ms = unix_epoch_ms();
        let mut stmt = conn.prepare(
            "SELECT lease_id, request_id, task_label, user_name, pid, needs_json, ttl_ms, expires_at_ms FROM active_leases",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(StoredLeaseRow {
                lease_id: row.get::<_, String>(0)?,
                request_id: row.get::<_, String>(1)?,
                task_label: row.get::<_, String>(2)?,
                user_name: row.get::<_, String>(3)?,
                pid: row.get::<_, u32>(4)?,
                needs_json: row.get::<_, String>(5)?,
                ttl_ms: row.get::<_, i64>(6)?,
                expires_at_ms: row.get::<_, i64>(7)?,
            })
        })?;

        let mut expired_ids = Vec::new();

        for row in rows {
            let row = row?;
            if row.expires_at_ms <= now_ms {
                expired_ids.push(row.lease_id);
                continue;
            }

            let ttl_ms = u64::try_from(row.ttl_ms).with_context(|| {
                format!(
                    "invalid persisted ttl_ms {} for lease {}",
                    row.ttl_ms, row.lease_id
                )
            })?;
            let remaining_ms = (row.expires_at_ms - now_ms) as u64;
            let needs: Vec<NeedRequest> =
                serde_json::from_str(&row.needs_json).with_context(|| {
                    format!("failed to parse needs_json for lease {}", row.lease_id)
                })?;

            self.allocate(&needs);
            self.leases.insert(
                row.lease_id,
                LeaseRecord {
                    needs,
                    expires_at: Instant::now() + Duration::from_millis(remaining_ms),
                    ttl_ms,
                    request_id: row.request_id,
                    task_label: row.task_label,
                    user_name: row.user_name,
                    pid: row.pid,
                },
            );
        }

        if !expired_ids.is_empty() {
            let mut conn = self
                .open_connection()?
                .ok_or_else(|| anyhow!("sqlite connection missing during cleanup"))?;
            let tx = conn.transaction()?;
            for lease_id in expired_ids {
                tx.execute("DELETE FROM active_leases WHERE lease_id = ?1", [lease_id])?;
            }
            tx.commit()?;
        }

        Ok(())
    }
}
