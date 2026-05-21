use anyhow::Result;
use rusqlite::{Connection, params};
use tak_exec::{OutputStream, TaskFinishedEvent, TaskOutputChunk, TaskStartedEvent};

use super::{TaskHistoryWriter, unix_epoch_ms};

struct StartedRunRecord<'a> {
    task_run_id: &'a str,
    task_label: &'a str,
    attempt: u32,
    origin: &'a str,
    runtime: &'a str,
    runtime_source: &'a str,
    command: &'a str,
    placement: &'a str,
    remote_node_id: &'a str,
    update_placement: bool,
}

impl TaskHistoryWriter {
    pub(in crate::cli::task_history) fn record_started(
        &mut self,
        task_run_id: &str,
        task_label: &str,
        attempt: u32,
    ) -> Result<()> {
        self.record_started_metadata(StartedRunRecord {
            task_run_id,
            task_label,
            attempt,
            origin: "task",
            runtime: "",
            runtime_source: "",
            command: "",
            placement: "local",
            remote_node_id: "",
            update_placement: false,
        })
    }

    pub(in crate::cli::task_history) fn record_started_event(
        &mut self,
        event: &TaskStartedEvent,
        task_label: &str,
    ) -> Result<()> {
        self.record_started_metadata(StartedRunRecord {
            task_run_id: &event.task_run_id,
            task_label,
            attempt: 1,
            origin: event.origin.as_deref().unwrap_or("task"),
            runtime: event.runtime.as_deref().unwrap_or(""),
            runtime_source: event.runtime_source.as_deref().unwrap_or(""),
            command: event.command.as_deref().unwrap_or(""),
            placement: event.placement_mode.as_str(),
            remote_node_id: event.remote_node_id.as_deref().unwrap_or(""),
            update_placement: true,
        })
    }

    fn record_started_metadata(&mut self, record: StartedRunRecord<'_>) -> Result<()> {
        self.conn.execute(
            "
            INSERT INTO task_runs (
                task_run_id, task_label, attempts, state, placement, remote_node_id,
                origin, runtime, runtime_source, command, started_at_ms, finished_at_ms
            )
            VALUES (?1, ?2, ?3, 'active', ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL)
            ON CONFLICT(task_run_id) DO UPDATE SET
                task_label = excluded.task_label,
                attempts = MAX(task_runs.attempts, excluded.attempts),
                state = 'active',
                placement = CASE
                    WHEN ?11 != 0 THEN excluded.placement
                    ELSE task_runs.placement
                END,
                remote_node_id = CASE
                    WHEN ?11 != 0 THEN excluded.remote_node_id
                    ELSE task_runs.remote_node_id
                END,
                origin = CASE
                    WHEN excluded.origin != 'task' THEN excluded.origin
                    ELSE task_runs.origin
                END,
                runtime = CASE
                    WHEN excluded.runtime != '' THEN excluded.runtime
                    ELSE task_runs.runtime
                END,
                runtime_source = CASE
                    WHEN excluded.runtime_source != '' THEN excluded.runtime_source
                    ELSE task_runs.runtime_source
                END,
                command = CASE
                    WHEN excluded.command != '' THEN excluded.command
                    ELSE task_runs.command
                END,
                started_at_ms = COALESCE(task_runs.started_at_ms, excluded.started_at_ms)
            ",
            params![
                record.task_run_id,
                record.task_label,
                i64::from(record.attempt),
                record.placement,
                record.remote_node_id,
                record.origin,
                record.runtime,
                record.runtime_source,
                record.command,
                unix_epoch_ms(),
                if record.update_placement {
                    1_i64
                } else {
                    0_i64
                },
            ],
        )?;
        Ok(())
    }

    pub(in crate::cli::task_history) fn append_output(
        &mut self,
        chunk: &TaskOutputChunk,
    ) -> Result<()> {
        let seq = next_output_seq(&self.conn, &chunk.task_run_id)?;
        self.conn.execute(
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
        &mut self,
        event: &TaskFinishedEvent,
    ) -> Result<()> {
        self.conn.execute(
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
