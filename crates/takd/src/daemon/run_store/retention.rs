use anyhow::{Result, anyhow};
use rusqlite::{Connection, OptionalExtension, Row, params};
use tak_proto::local_daemon::v2::{RunEvent, RunEventKind};

use super::{RunAttachmentSnapshot, RunStore, queries};

mod cleanup;
mod shared_workspaces;

const ATTACH_EVENT_PAGE_SIZE: usize = 256;

impl RunStore {
    pub fn expire_run_payloads(&self, run_id: &str) -> Result<()> {
        cleanup::expire_one(self, run_id)
    }

    pub fn events_after(&self, run_id: &str, after_event: u64) -> Result<Vec<RunEvent>> {
        let connection = self.open_connection()?;
        let mut events = load_events(&connection, run_id, after_event, None)?;
        hide_expired_logs(
            &mut events,
            expiration(&connection, run_id)?.is_some_and(|x| x.0),
        );
        Ok(events)
    }

    pub fn attachment_snapshot(
        &self,
        run_id: &str,
        after_event: u64,
    ) -> Result<Option<RunAttachmentSnapshot>> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction()?;
        let Some(summary) = queries::summary(&transaction, run_id)? else {
            return Ok(None);
        };
        let (logs_expired, _) = expiration(&transaction, run_id)?.unwrap_or_default();
        let mut events = load_events(
            &transaction,
            run_id,
            after_event,
            Some(ATTACH_EVENT_PAGE_SIZE + 1),
        )?;
        let has_more = events.len() > ATTACH_EVENT_PAGE_SIZE;
        events.truncate(ATTACH_EVENT_PAGE_SIZE);
        let next_event = events.last().map_or(after_event, |event| event.seq);
        hide_expired_logs(&mut events, logs_expired);
        transaction.commit()?;
        Ok(Some(RunAttachmentSnapshot {
            summary,
            events,
            next_event,
            has_more,
            logs_expired,
        }))
    }
}

pub(super) fn expire_due_payloads(store: &RunStore, now_ms: u64, ttl_ms: u64) -> Result<u64> {
    cleanup::expire_due(store, now_ms, ttl_ms)
}

pub(super) fn purge_due_metadata(store: &RunStore, now_ms: u64, ttl_ms: u64) -> Result<u64> {
    cleanup::purge_due(store, now_ms, ttl_ms)
}

pub(super) fn expiration(connection: &Connection, run_id: &str) -> Result<Option<(bool, bool)>> {
    connection
        .query_row(
            "SELECT logs_expired,outputs_expired FROM runs WHERE run_id=?1",
            [run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(Into::into)
}

fn load_events(
    connection: &Connection,
    run_id: &str,
    after_event: u64,
    limit: Option<usize>,
) -> Result<Vec<RunEvent>> {
    let sql = match limit {
        Some(_) => {
            "SELECT payload_json FROM run_events WHERE run_id=?1 AND seq>?2 ORDER BY seq LIMIT ?3"
        }
        None => "SELECT payload_json FROM run_events WHERE run_id=?1 AND seq>?2 ORDER BY seq",
    };
    let mut statement = connection.prepare(sql)?;
    let mut rows = match limit {
        Some(limit) => statement.query(params![run_id, sqlite_i64(after_event)?, limit as i64])?,
        None => statement.query(params![run_id, sqlite_i64(after_event)?])?,
    };
    let mut events = Vec::new();
    while let Some(row) = rows.next()? {
        events.push(decode_event(row)?);
    }
    Ok(events)
}

fn hide_expired_logs(events: &mut Vec<RunEvent>, expired: bool) {
    if expired {
        events.retain(|event| {
            !matches!(event.kind, RunEventKind::Stdout | RunEventKind::Stderr)
                && event.chunk_base64.is_none()
        });
    }
}

fn decode_event(row: &Row<'_>) -> rusqlite::Result<RunEvent> {
    let payload = row.get::<_, String>(0)?;
    serde_json::from_str(&payload).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

fn sqlite_i64(value: u64) -> Result<i64> {
    value
        .try_into()
        .map_err(|_| anyhow!("event cursor exceeds SQLite range"))
}
