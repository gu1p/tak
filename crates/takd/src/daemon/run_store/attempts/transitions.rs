use anyhow::{Result, bail};
use rusqlite::{Transaction, params};
use tak_core::v2::ResolvedJob;
use tak_proto::local_daemon::v2::RunEventKind;

use crate::daemon::scheduler::DispatchCommand;

use super::super::events::{
    append_event, append_job_event, append_skipped_event, now_ms, sqlite_i64,
};

mod group_failure;
mod retry;

pub(super) use group_failure::fail_affinity_group;

pub(super) fn schedule_retry(
    transaction: &Transaction<'_>,
    command: &DispatchCommand,
    job: &ResolvedJob,
    message: &str,
) -> Result<()> {
    let now = now_ms()?;
    let eligible = now
        .saturating_add(retry::delay(&job.retry, command.authored_attempt))
        .min(i64::MAX as u64);
    let updated = transaction.execute(
        "UPDATE run_jobs SET state = 'retrying', node_id = NULL, current_fencing_token = NULL, next_eligible_at_ms = ?4, ready_order = (SELECT COALESCE(MAX(other.ready_order), 0) + 1 FROM run_jobs other WHERE other.run_id = ?1) WHERE run_id = ?1 AND job_id = ?2 AND current_fencing_token = ?3",
        params![command.run_id, command.job_id, command.fencing_token,
            sqlite_i64(eligible, "retry timestamp")?],
    )?;
    if updated != 1 {
        bail!("attempt fence is no longer current");
    }
    append_job_event(
        transaction,
        &command.run_id,
        RunEventKind::Retrying,
        &command.job_id,
        &job.task_ids,
        &command.node_id,
        message,
    )?;
    rearm_scheduler(transaction, &command.run_id)?;
    refresh_run_state(transaction, &command.run_id)
}

pub(super) fn finish_job(
    transaction: &Transaction<'_>,
    command: &DispatchCommand,
    job: &ResolvedJob,
    succeeded: bool,
) -> Result<()> {
    let state = if succeeded { "succeeded" } else { "failed" };
    let updated = transaction.execute(
        "UPDATE run_jobs SET state = ?4, current_fencing_token = NULL WHERE run_id = ?1 AND job_id = ?2 AND current_fencing_token = ?3",
        params![command.run_id, command.job_id, command.fencing_token, state],
    )?;
    if updated != 1 {
        bail!("attempt fence is no longer current");
    }
    let kind = if succeeded {
        RunEventKind::Succeeded
    } else {
        RunEventKind::Failed
    };
    append_job_event(
        transaction,
        &command.run_id,
        kind,
        &command.job_id,
        &job.task_ids,
        &command.node_id,
        if succeeded {
            "job succeeded"
        } else {
            "job failed"
        },
    )?;
    if succeeded {
        promote_dependencies(transaction, &command.run_id)?;
    } else {
        skip_after_failure(transaction, &command.run_id, &command.job_id)?;
    }
    rearm_scheduler(transaction, &command.run_id)?;
    refresh_run_state(transaction, &command.run_id)
}

fn promote_dependencies(transaction: &Transaction<'_>, run_id: &str) -> Result<()> {
    let now = sqlite_i64(now_ms()?, "timestamp")?;
    transaction.execute(
        "UPDATE run_jobs AS job SET state = 'ready', next_eligible_at_ms = ?2, \
         ready_order = (SELECT COALESCE(MAX(other.ready_order), 0) + 1 + job.ordinal FROM run_jobs other WHERE other.run_id = ?1) \
         WHERE job.run_id = ?1 AND job.state = 'blocked' \
         AND NOT EXISTS (SELECT 1 FROM run_dependencies dependency \
             JOIN run_jobs prerequisite ON prerequisite.run_id = dependency.run_id \
             AND prerequisite.job_id = dependency.dependency_job_id \
             WHERE dependency.run_id = job.run_id \
             AND dependency.dependent_job_id = job.job_id \
             AND prerequisite.state != 'succeeded')",
        params![run_id, now],
    )?;
    Ok(())
}

fn skip_after_failure(transaction: &Transaction<'_>, run_id: &str, job_id: &str) -> Result<()> {
    let keep_going = transaction.query_row(
        "SELECT keep_going FROM runs WHERE run_id = ?1",
        [run_id],
        |row| row.get::<_, bool>(0),
    )?;
    if !keep_going {
        transaction.execute(
            "UPDATE runs SET dispatch_stopped = 1 WHERE run_id = ?1",
            [run_id],
        )?;
    }
    let sql = if keep_going {
        "WITH RECURSIVE descendants(job_id) AS (\
            SELECT dependent_job_id FROM run_dependencies WHERE run_id = ?1 AND dependency_job_id = ?2 \
            UNION SELECT edge.dependent_job_id FROM run_dependencies edge JOIN descendants parent \
                ON edge.dependency_job_id = parent.job_id WHERE edge.run_id = ?1) \
         SELECT job.job_id, job.definition_json FROM descendants \
         JOIN run_jobs job ON job.run_id = ?1 AND job.job_id = descendants.job_id \
         WHERE job.state IN ('ready', 'blocked', 'retrying') ORDER BY job.ordinal"
    } else {
        "SELECT job_id, definition_json FROM run_jobs WHERE run_id = ?1 \
         AND job_id != ?2 AND state IN ('ready', 'blocked', 'retrying') ORDER BY ordinal"
    };
    let skipped = {
        let mut statement = transaction.prepare(sql)?;
        statement
            .query_map(params![run_id, job_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (skipped_id, definition) in skipped {
        let job: ResolvedJob = serde_json::from_str(&definition)?;
        transaction.execute(
            "UPDATE run_jobs SET state = 'skipped' WHERE run_id = ?1 AND job_id = ?2",
            params![run_id, skipped_id],
        )?;
        append_skipped_event(transaction, run_id, &skipped_id, &job.task_ids)?;
    }
    Ok(())
}

fn rearm_scheduler(transaction: &Transaction<'_>, run_id: &str) -> Result<()> {
    transaction.execute(
        "INSERT INTO run_outbox (run_id, kind, payload_json) VALUES (?1, 'scheduler_wakeup', '{}') \
         ON CONFLICT(run_id, kind) DO UPDATE SET delivered_at_ms = NULL",
        [run_id],
    )?;
    Ok(())
}

fn refresh_run_state(transaction: &Transaction<'_>, run_id: &str) -> Result<()> {
    let (nonterminal, failed, active) = transaction.query_row(
        "SELECT SUM(CASE WHEN state NOT IN ('succeeded','failed','cancelled','skipped') THEN 1 ELSE 0 END), \
         SUM(CASE WHEN state IN ('failed','skipped') THEN 1 ELSE 0 END), \
         SUM(CASE WHEN state IN ('transferring','running','output_committing','cancelling') THEN 1 ELSE 0 END) \
         FROM run_jobs WHERE run_id = ?1",
        [run_id],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)),
    )?;
    let state = if nonterminal == 0 {
        if failed == 0 { "succeeded" } else { "failed" }
    } else if active > 0 {
        "running"
    } else {
        "queued"
    };
    let previous = transaction.query_row(
        "SELECT state FROM runs WHERE run_id = ?1",
        [run_id],
        |row| row.get::<_, String>(0),
    )?;
    transaction.execute(
        "UPDATE runs SET state = ?2, updated_at_ms = ?3 WHERE run_id = ?1",
        params![run_id, state, sqlite_i64(now_ms()?, "timestamp")?],
    )?;
    if previous != state && matches!(state, "succeeded" | "failed") {
        append_event(
            transaction,
            run_id,
            if state == "succeeded" {
                RunEventKind::Succeeded
            } else {
                RunEventKind::Failed
            },
            if state == "succeeded" {
                "run succeeded"
            } else {
                "run failed"
            },
        )?;
    }
    Ok(())
}
