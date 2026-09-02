use anyhow::{Result, anyhow};
use rusqlite::{Connection, OptionalExtension, Row};
use tak_core::v2::ResolvedJob;
use tak_proto::local_daemon::v2::{RunDetails, RunJobSummary, RunLifecycleState, RunSummary};

use super::{RunStore, retention};

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
            "SELECT job_id, state, node_id, attempt, definition_json, cache FROM run_jobs \
             WHERE run_id = ?1 ORDER BY ordinal, job_id",
        )?;
        let jobs = statement
            .query_map([run_id], job_summary)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let (logs_expired, outputs_expired) =
            retention::expiration(&connection, run_id)?.unwrap_or_default();
        Ok(Some(RunDetails {
            summary,
            jobs,
            logs_expired,
            outputs_expired,
        }))
    }
}

pub(super) fn summary(connection: &Connection, run_id: &str) -> Result<Option<RunSummary>> {
    let stored = connection
        .query_row(
            "SELECT r.run_id, r.state, r.created_at_ms, r.updated_at_ms, r.targets_json, \
             COUNT(j.job_id), COALESCE(SUM(CASE WHEN j.state IN ('succeeded', 'failed', 'cancelled', 'skipped') THEN 1 ELSE 0 END), 0), r.exit_code \
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
                    row.get::<_, Option<i32>>(7)?,
                ))
            },
        )
        .optional()?;
    let Some((run_id, state, created, updated, targets_json, total, terminal, exit_code)) = stored
    else {
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
        exit_code,
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
        cache: row.get(5)?,
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
