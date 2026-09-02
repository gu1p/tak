use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use base64::Engine as _;
use rusqlite::{Transaction, params};
use tak_proto::local_daemon::v2::{RunEvent, RunEventKind};

pub(super) fn append_event(
    transaction: &Transaction<'_>,
    run_id: &str,
    kind: RunEventKind,
    message: &str,
) -> Result<u64> {
    append_context_event(
        transaction,
        run_id,
        kind,
        None,
        &[],
        None,
        message,
        None,
        None,
    )
}

pub(super) fn append_terminal_event(
    transaction: &Transaction<'_>,
    run_id: &str,
    kind: RunEventKind,
    message: &str,
    exit_code: Option<i32>,
) -> Result<u64> {
    append_context_event(
        transaction,
        run_id,
        kind,
        None,
        &[],
        None,
        message,
        None,
        exit_code,
    )
}

pub(super) fn append_job_event(
    transaction: &Transaction<'_>,
    run_id: &str,
    kind: RunEventKind,
    job_id: &str,
    task_ids: &[String],
    node_id: &str,
    message: &str,
) -> Result<u64> {
    append_context_event(
        transaction,
        run_id,
        kind,
        Some(job_id),
        task_ids,
        Some(node_id),
        message,
        None,
        None,
    )
}

pub(super) struct TerminalDetails<'a> {
    pub message: &'a str,
    pub exit_code: Option<i32>,
}

pub(super) fn append_job_terminal_event(
    transaction: &Transaction<'_>,
    run_id: &str,
    kind: RunEventKind,
    job_id: &str,
    task_ids: &[String],
    node_id: &str,
    terminal: TerminalDetails<'_>,
) -> Result<u64> {
    append_context_event(
        transaction,
        run_id,
        kind,
        Some(job_id),
        task_ids,
        Some(node_id),
        terminal.message,
        None,
        terminal.exit_code,
    )
}

pub(super) fn append_output_event(
    transaction: &Transaction<'_>,
    run_id: &str,
    kind: RunEventKind,
    job_id: &str,
    task_ids: &[String],
    node_id: &str,
    bytes: &[u8],
) -> Result<u64> {
    append_context_event(
        transaction,
        run_id,
        kind,
        Some(job_id),
        task_ids,
        Some(node_id),
        "",
        Some(base64::engine::general_purpose::STANDARD.encode(bytes)),
        None,
    )
}

pub(super) fn append_skipped_event(
    transaction: &Transaction<'_>,
    run_id: &str,
    job_id: &str,
    task_ids: &[String],
) -> Result<u64> {
    append_context_event(
        transaction,
        run_id,
        RunEventKind::Skipped,
        Some(job_id),
        task_ids,
        None,
        "job skipped after dependency failure",
        None,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn append_context_event(
    transaction: &Transaction<'_>,
    run_id: &str,
    kind: RunEventKind,
    job_id: Option<&str>,
    task_ids: &[String],
    node_id: Option<&str>,
    message: &str,
    chunk_base64: Option<String>,
    exit_code: Option<i32>,
) -> Result<u64> {
    let next: i64 = transaction.query_row(
        "SELECT COALESCE(MAX(seq), 0) + 1 FROM run_events WHERE run_id = ?1",
        [run_id],
        |row| row.get(0),
    )?;
    let event = RunEvent {
        seq: u64::try_from(next).map_err(|_| anyhow!("event sequence overflow"))?,
        kind,
        job_id: job_id.map(str::to_owned),
        task_ids: task_ids.to_vec(),
        node_id: node_id.map(str::to_owned),
        message: message.to_owned(),
        chunk_base64,
        exit_code,
    };
    transaction.execute(
        "INSERT INTO run_events (run_id, seq, payload_json, created_at_ms) VALUES (?1, ?2, ?3, ?4)",
        params![
            run_id,
            next,
            serde_json::to_string(&event)?,
            sqlite_i64(now_ms()?, "event timestamp")?
        ],
    )?;
    Ok(event.seq)
}

pub(super) fn now_ms() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| anyhow!("system clock precedes Unix epoch"))?
        .as_millis()
        .try_into()
        .map_err(|_| anyhow!("system clock exceeds SQLite range"))
}

pub(super) fn sqlite_i64(value: u64, name: &str) -> Result<i64> {
    value
        .try_into()
        .map_err(|_| anyhow!("{name} exceeds SQLite range"))
}
