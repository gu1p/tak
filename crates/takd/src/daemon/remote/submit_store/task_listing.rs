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

    pub fn task_attempt_summaries(
        &self,
        active_only: bool,
        limit: usize,
    ) -> Result<Vec<SubmitAttemptSummaryRecord>> {
        let conn = self.open_connection()?;
        let where_clause = if active_only {
            "WHERE results.idempotency_key IS NULL"
        } else {
            ""
        };
        let sql = format!(
            "
            SELECT attempts.task_run_id,
                   attempts.attempt,
                   attempts.task_label,
                   attempts.selected_node_id,
                   attempts.created_at_ms,
                   results.payload_json
            FROM submit_attempts attempts
            LEFT JOIN submit_results results
              ON attempts.idempotency_key = results.idempotency_key
            {where_clause}
            ORDER BY attempts.created_at_ms DESC, attempts.task_run_id ASC, attempts.attempt ASC
            LIMIT ?1
            "
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![i64::try_from(limit).unwrap_or(i64::MAX)], |row| {
            task_summary_from_row(row)
        })?;
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

fn task_summary_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SubmitAttemptSummaryRecord> {
    let attempt = row.get::<_, i64>(1)?;
    let result_payload = row.get::<_, Option<String>>(5)?;
    let finished_at_ms = result_payload.as_deref().and_then(result_finished_at_ms);
    Ok(SubmitAttemptSummaryRecord {
        task_run_id: row.get(0)?,
        attempt: u32::try_from(attempt).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(
                1,
                rusqlite::types::Type::Integer,
                Box::new(err),
            )
        })?,
        task_label: row.get(2)?,
        selected_node_id: row.get(3)?,
        state: if result_payload.is_some() {
            "completed".to_string()
        } else {
            "active".to_string()
        },
        created_at_ms: row.get(4)?,
        finished_at_ms,
    })
}

fn result_finished_at_ms(payload: &str) -> Option<i64> {
    serde_json::from_str::<serde_json::Value>(payload)
        .ok()
        .and_then(|value| {
            value
                .get("finished_at")
                .or_else(|| value.get("finished_at_ms"))
                .and_then(serde_json::Value::as_i64)
        })
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
