use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use rusqlite::{Transaction, params};
use tak_proto::local_daemon::v2::{RunEvent, RunEventKind};

pub(super) fn append_event(
    transaction: &Transaction<'_>,
    run_id: &str,
    kind: RunEventKind,
    message: &str,
) -> Result<u64> {
    append_context_event(transaction, run_id, kind, None, &[], None, message)
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
    )
}

fn append_context_event(
    transaction: &Transaction<'_>,
    run_id: &str,
    kind: RunEventKind,
    job_id: Option<&str>,
    task_ids: &[String],
    node_id: Option<&str>,
    message: &str,
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
