use anyhow::{Result, anyhow};
use rusqlite::{Connection, OptionalExtension, Row, params};
use tak_core::v2::ResolvedJob;
use tak_proto::local_daemon::v2::{
    RunDetails, RunEvent, RunJobSummary, RunLifecycleState, RunSummary,
};

use super::RunStore;

const ATTACH_EVENT_PAGE_SIZE: usize = 256;

impl RunStore {
    pub fn summary(&self, run_id: &str) -> Result<Option<RunSummary>> {
        let connection = self.open_connection()?;
        summary(&connection, run_id)
    }

    pub fn list_runs(&self) -> Result<Vec<RunSummary>> {
        let connection = self.open_connection()?;
        let mut statement =
            connection.prepare("SELECT run_id FROM runs ORDER BY created_at_ms, run_id")?;
        let run_ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        run_ids
            .iter()
            .map(|run_id| {
                summary(&connection, run_id)?
                    .ok_or_else(|| anyhow!("run disappeared while listing"))
            })
            .collect()
    }

    pub fn get_run(&self, run_id: &str) -> Result<Option<RunDetails>> {
        let connection = self.open_connection()?;
        let Some(summary) = summary(&connection, run_id)? else {
            return Ok(None);
        };
        let mut statement = connection.prepare(
            "SELECT job_id, state, node_id, attempt, definition_json FROM run_jobs \
             WHERE run_id = ?1 ORDER BY ordinal, job_id",
        )?;
        let jobs = statement
            .query_map([run_id], job_summary)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(Some(RunDetails { summary, jobs }))
    }

    pub fn events_after(&self, run_id: &str, after_event: u64) -> Result<Vec<RunEvent>> {
        let connection = self.open_connection()?;
        events_after(&connection, run_id, after_event)
    }

    pub fn attachment_snapshot(
        &self,
        run_id: &str,
        after_event: u64,
    ) -> Result<Option<(RunSummary, Vec<RunEvent>, bool)>> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction()?;
        let Some(summary) = summary(&transaction, run_id)? else {
            return Ok(None);
        };
        let (events, has_more) = event_page(&transaction, run_id, after_event)?;
        transaction.commit()?;
        Ok(Some((summary, events, has_more)))
    }
}

fn event_page(
    connection: &Connection,
    run_id: &str,
    after_event: u64,
) -> Result<(Vec<RunEvent>, bool)> {
    let mut statement = connection.prepare(
        "SELECT payload_json FROM run_events WHERE run_id = ?1 AND seq > ?2 ORDER BY seq LIMIT ?3",
    )?;
    let mut events = statement
        .query_map(
            params![
                run_id,
                sqlite_i64(after_event)?,
                i64::try_from(ATTACH_EVENT_PAGE_SIZE + 1).expect("page size fits SQLite")
            ],
            decode_event,
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let has_more = events.len() > ATTACH_EVENT_PAGE_SIZE;
    events.truncate(ATTACH_EVENT_PAGE_SIZE);
    Ok((events, has_more))
}

fn events_after(connection: &Connection, run_id: &str, after_event: u64) -> Result<Vec<RunEvent>> {
    let mut statement = connection.prepare(
        "SELECT payload_json FROM run_events WHERE run_id = ?1 AND seq > ?2 ORDER BY seq",
    )?;
    statement
        .query_map(params![run_id, sqlite_i64(after_event)?], decode_event)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn decode_event(row: &Row<'_>) -> rusqlite::Result<RunEvent> {
    let payload = row.get::<_, String>(0)?;
    serde_json::from_str(&payload).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

fn summary(connection: &Connection, run_id: &str) -> Result<Option<RunSummary>> {
    let stored = connection
        .query_row(
            "SELECT r.run_id, r.state, r.created_at_ms, r.updated_at_ms, r.targets_json, \
             COUNT(j.job_id), COALESCE(SUM(CASE WHEN j.state IN ('succeeded', 'failed', 'cancelled', 'skipped') THEN 1 ELSE 0 END), 0) \
             FROM runs r LEFT JOIN run_jobs j ON j.run_id = r.run_id \
             WHERE r.run_id = ?1 GROUP BY r.run_id",
            [run_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                ))
            },
        )
        .optional()?;
    let Some((run_id, state, created, updated, targets_json, total, terminal)) = stored else {
        return Ok(None);
    };
    Ok(Some(RunSummary {
        run_id,
        state: parse_state(&state)?,
        created_at_ms: unsigned(created, "created timestamp")?,
        updated_at_ms: unsigned(updated, "updated timestamp")?,
        targets: serde_json::from_str(&targets_json)
            .map_err(|error| anyhow!("stored run targets are invalid: {error}"))?,
        total_jobs: unsigned(total, "job count")?,
        terminal_jobs: unsigned(terminal, "terminal job count")?,
    }))
}

fn job_summary(row: &Row<'_>) -> rusqlite::Result<RunJobSummary> {
    let definition_json = row.get::<_, String>(4)?;
    let definition: ResolvedJob = serde_json::from_str(&definition_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(RunJobSummary {
        job_id: row.get(0)?,
        task_ids: definition.task_ids,
        state: row.get(1)?,
        node_id: row.get(2)?,
        attempt: row.get(3)?,
        cache: None,
    })
}

fn parse_state(value: &str) -> Result<RunLifecycleState> {
    match value {
        "awaiting_workspace" => Ok(RunLifecycleState::AwaitingWorkspace),
        "awaiting_commit" => Ok(RunLifecycleState::AwaitingCommit),
        "queued" => Ok(RunLifecycleState::Queued),
        "running" => Ok(RunLifecycleState::Running),
        "cancelling" => Ok(RunLifecycleState::Cancelling),
        "succeeded" => Ok(RunLifecycleState::Succeeded),
        "failed" => Ok(RunLifecycleState::Failed),
        "cancelled" => Ok(RunLifecycleState::Cancelled),
        _ => Err(anyhow!("stored run has unknown state `{value}`")),
    }
}

fn unsigned<T>(value: i64, name: &str) -> Result<T>
where
    T: TryFrom<i64>,
{
    value
        .try_into()
        .map_err(|_| anyhow!("stored {name} is invalid"))
}

fn sqlite_i64(value: u64) -> Result<i64> {
    value
        .try_into()
        .map_err(|_| anyhow!("event cursor exceeds SQLite range"))
}
