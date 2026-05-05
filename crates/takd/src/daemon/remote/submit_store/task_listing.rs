use super::*;

impl SubmitAttemptStore {
    /// Lists submit attempts that do not have a terminal result yet.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on local sqlite availability and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn active_attempts(&self) -> Result<Vec<ActiveSubmitAttempt>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "
            SELECT attempts.idempotency_key,
                   attempts.task_run_id,
                   attempts.attempt,
                   attempts.task_label,
                   attempts.selected_node_id,
                   attempts.created_at_ms
            FROM submit_attempts attempts
            LEFT JOIN submit_results results
              ON attempts.idempotency_key = results.idempotency_key
            WHERE results.idempotency_key IS NULL
            ORDER BY attempts.created_at_ms ASC, attempts.task_run_id ASC, attempts.attempt ASC
            ",
        )?;
        let rows = stmt.query_map([], active_attempt_from_row)?;
        let mut attempts = Vec::new();
        for row in rows {
            attempts.push(row?);
        }
        Ok(attempts)
    }

    pub(in crate::daemon::remote::submit_store) fn next_event_seq(
        &self,
        idempotency_key: &str,
    ) -> Result<u64> {
        let key = idempotency_key.trim();
        if key.is_empty() {
            bail!("idempotency_key is required");
        }
        let conn = self.open_connection()?;
        self.ensure_submit_attempt_exists(&conn, key)?;
        let next = conn.query_row(
            "
            SELECT COALESCE(MAX(seq), 0) + 1
            FROM submit_events
            WHERE idempotency_key = ?1
            ",
            params![key],
            |row| row.get::<_, i64>(0),
        )?;
        u64::try_from(next).with_context(|| format!("invalid next submit event seq {next}"))
    }
}

fn active_attempt_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ActiveSubmitAttempt> {
    let attempt = row.get::<_, i64>(2)?;
    Ok(ActiveSubmitAttempt {
        idempotency_key: row.get(0)?,
        task_run_id: row.get(1)?,
        attempt: u32::try_from(attempt).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Integer,
                Box::new(err),
            )
        })?,
        task_label: row.get(3)?,
        selected_node_id: row.get(4)?,
        created_at_ms: row.get(5)?,
    })
}
