use anyhow::Result;
use rusqlite::{Connection, params};
use tak_exec::{OutputStream, TaskFinishedEvent, TaskOutputChunk};

use super::{TaskHistoryStore, unix_epoch_ms};

impl TaskHistoryStore {
    pub(in crate::cli::task_history) fn record_started(
        &self,
        task_run_id: &str,
        task_label: &str,
        attempt: u32,
    ) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute(
            "
            INSERT INTO task_runs (
                task_run_id, task_label, attempts, state, placement, remote_node_id,
                started_at_ms, finished_at_ms
            )
            VALUES (?1, ?2, ?3, 'active', 'local', '', ?4, NULL)
            ON CONFLICT(task_run_id) DO UPDATE SET
                task_label = excluded.task_label,
                attempts = MAX(task_runs.attempts, excluded.attempts),
                state = 'active',
                placement = 'local',
                started_at_ms = COALESCE(task_runs.started_at_ms, excluded.started_at_ms)
            ",
            params![task_run_id, task_label, i64::from(attempt), unix_epoch_ms()],
        )?;
        Ok(())
    }

    pub(in crate::cli::task_history) fn append_output(
        &self,
        chunk: &TaskOutputChunk,
    ) -> Result<()> {
        let conn = self.open_connection()?;
        let seq = next_output_seq(&conn, &chunk.task_run_id)?;
        conn.execute(
            "
            INSERT INTO task_outputs (task_run_id, seq, attempt, stream, bytes)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            params![
                chunk.task_run_id,
                seq,
                i64::from(chunk.attempt),
                output_stream_name(chunk.stream),
                chunk.bytes
            ],
        )?;
        Ok(())
    }

    pub(in crate::cli::task_history) fn record_finished(
        &self,
        event: &TaskFinishedEvent,
    ) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute(
            "
            INSERT INTO task_runs (
                task_run_id, task_label, attempts, state, placement, remote_node_id,
                started_at_ms, finished_at_ms
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(task_run_id) DO UPDATE SET
                task_label = excluded.task_label,
                attempts = excluded.attempts,
                state = excluded.state,
                placement = excluded.placement,
                remote_node_id = excluded.remote_node_id,
                finished_at_ms = excluded.finished_at_ms
            ",
            params![
                event.task_run_id,
                task_label_from_finished(event),
                i64::from(event.attempts),
                if event.success { "success" } else { "failed" },
                event.placement_mode.as_str(),
                event.remote_node_id.clone().unwrap_or_default(),
                unix_epoch_ms(),
                unix_epoch_ms()
            ],
        )?;
        Ok(())
    }
}

fn next_output_seq(conn: &Connection, task_run_id: &str) -> Result<i64> {
    conn.query_row(
        "
        SELECT COALESCE(MAX(seq), 0) + 1
        FROM task_outputs
        WHERE task_run_id = ?1
        ",
        params![task_run_id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

fn output_stream_name(stream: OutputStream) -> &'static str {
    match stream {
        OutputStream::Stdout => "stdout",
        OutputStream::Stderr => "stderr",
    }
}

fn task_label_from_finished(event: &TaskFinishedEvent) -> String {
    if event.task_label.package == "//" {
        format!("//:{}", event.task_label.name)
    } else {
        format!("{}:{}", event.task_label.package, event.task_label.name)
    }
}
