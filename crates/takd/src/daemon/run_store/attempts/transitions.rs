use anyhow::{Result, bail};
use rusqlite::{Transaction, params};
use tak_core::v2::ResolvedJob;
use tak_proto::local_daemon::v2::RunEventKind;

use crate::daemon::scheduler::DispatchCommand;

use super::super::events::{append_job_event, now_ms, sqlite_i64};

mod group_failure;
mod retry;
mod settlement;

pub(super) use group_failure::fail_affinity_group;
pub(super) use settlement::{fail_job, finish_job};
use settlement::{rearm_scheduler, refresh_run_state, skip_after_failure};

pub(super) fn schedule_retry(
    transaction: &Transaction<'_>,
    command: &DispatchCommand,
    job: &ResolvedJob,
    message: &str,
) -> Result<()> {
    let now = now_ms()?;
    let eligible = now
        .saturating_add(retry::delay(
            &job.retry,
            command.authored_attempt,
            &command.fencing_token,
        ))
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
    settlement::refresh_run_state(transaction, &command.run_id)
}
