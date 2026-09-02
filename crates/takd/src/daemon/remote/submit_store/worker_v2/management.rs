use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, params};
use tak_proto::worker_v2::{DispatchAttemptRequest, ObserveAttemptResponse, WorkerAttemptIdentity};

use crate::daemon::remote::SubmitAttemptSummaryRecord;

use super::SubmitAttemptStore;

impl SubmitAttemptStore {
    pub(crate) fn worker_v2_task_attempt_summaries(
        &self,
        active_only: bool,
        limit: usize,
    ) -> Result<Vec<SubmitAttemptSummaryRecord>> {
        let connection = self.open_connection()?;
        let where_clause = if active_only {
            "WHERE state IN ('accepted','running','cancelling')"
        } else {
            ""
        };
        let sql = format!(
            "SELECT fencing_token,authored_attempt,node_id,state,created_at_ms,updated_at_ms,\
             request_json FROM worker_v2_attempts {where_clause} ORDER BY created_at_ms DESC,\
             run_id ASC,job_id ASC,authored_attempt DESC LIMIT ?1"
        );
        let mut statement = connection.prepare(&sql)?;
        let rows = statement.query_map([i64::try_from(limit).unwrap_or(i64::MAX)], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        rows.map(|row| {
            let (fence, attempt, node, state, created_at, updated_at, request_json) = row?;
            let request: DispatchAttemptRequest = serde_json::from_str(&request_json)?;
            let task_label = request
                .payload
                .tasks
                .iter()
                .map(|task| task.task_id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            Ok(SubmitAttemptSummaryRecord {
                task_run_id: fence,
                attempt: u32::try_from(attempt).context("invalid worker v2 authored attempt")?,
                task_label,
                execution_label: None,
                selected_node_id: node,
                finished_at_ms: is_terminal(&state).then_some(updated_at),
                state,
                created_at_ms: created_at,
            })
        })
        .collect()
    }

    pub(crate) fn observe_worker_v2_task(
        &self,
        fencing_token: &str,
        authored_attempt: Option<u32>,
        after_event: u64,
    ) -> Result<Option<ObserveAttemptResponse>> {
        let connection = self.open_connection()?;
        let attempt = authored_attempt.map(i64::from);
        let identity = connection
            .query_row(
                "SELECT run_id,job_id,node_id,authored_attempt,dispatch_generation,fencing_token \
                 FROM worker_v2_attempts WHERE fencing_token=?1 AND \
                 (?2 IS NULL OR authored_attempt=?2)",
                params![fencing_token, attempt],
                |row| {
                    Ok(WorkerAttemptIdentity {
                        run_id: row.get(0)?,
                        job_id: row.get(1)?,
                        node_id: row.get(2)?,
                        authored_attempt: row.get(3)?,
                        dispatch_generation: row.get(4)?,
                        fencing_token: row.get(5)?,
                    })
                },
            )
            .optional()?;
        drop(connection);
        identity
            .map(|identity| self.observe_worker_v2_attempt(&identity, after_event))
            .transpose()
    }
}

fn is_terminal(state: &str) -> bool {
    matches!(state, "succeeded" | "failed" | "cancelled" | "missing")
}
