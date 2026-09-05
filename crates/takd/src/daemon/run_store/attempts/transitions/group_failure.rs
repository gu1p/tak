use anyhow::Result;
use rusqlite::{Transaction, params};
use tak_core::v2::{Affinity, ResolvedJob, ResolvedRun};
use tak_proto::local_daemon::v2::RunEventKind;

use crate::daemon::run_store::events::{JobEventDetails, append_job_event, now_ms, sqlite_i64};

pub(in crate::daemon::run_store::attempts) fn fail_affinity_group(
    transaction: &Transaction<'_>,
    run_id: &str,
    group: &str,
    node_id: &str,
    run: &ResolvedRun,
) -> Result<()> {
    let now = sqlite_i64(now_ms()?, "timestamp")?;
    let mut failed = Vec::new();
    for job in run.jobs.iter().filter(|job| belongs_to(job, group)) {
        let changed = transaction.execute(
            "UPDATE run_jobs SET state = 'failed', current_fencing_token = NULL \
             WHERE run_id = ?1 AND job_id = ?2 \
             AND state NOT IN ('succeeded', 'failed', 'cancelled', 'skipped')",
            params![run_id, job.job_id],
        )?;
        if changed == 1 {
            release_attempts(transaction, run_id, &job.job_id, now)?;
            failed.push(job);
        }
    }
    for job in &failed {
        append_job_event(
            transaction,
            run_id,
            RunEventKind::Failed,
            &job.job_id,
            &job.task_ids,
            node_id,
            JobEventDetails::new("hard-affinity home was lost"),
        )?;
    }
    for job in &failed {
        super::skip_after_failure(transaction, run_id, &job.job_id)?;
    }
    if !failed.is_empty() {
        super::rearm_scheduler(transaction, run_id)?;
        super::refresh_run_state(transaction, run_id)?;
    }
    Ok(())
}

fn belongs_to(job: &ResolvedJob, group: &str) -> bool {
    matches!(
        &job.affinity,
        Some(Affinity::PreferSameNode { group: member })
            | Some(Affinity::RequireSameNode { group: member }) if member == group
    )
}

fn release_attempts(
    transaction: &Transaction<'_>,
    run_id: &str,
    job_id: &str,
    now: i64,
) -> Result<()> {
    transaction.execute(
        "UPDATE run_attempts SET state = 'unknown', outcome = 'unknown', \
         finished_at_ms = ?3, released_at_ms = ?3 \
         WHERE run_id = ?1 AND job_id = ?2 AND released_at_ms IS NULL",
        params![run_id, job_id, now],
    )?;
    transaction.execute(
        "UPDATE run_dispatch_outbox SET delivered_at_ms = COALESCE(delivered_at_ms, ?3) \
         WHERE run_id = ?1 AND job_id = ?2",
        params![run_id, job_id, now],
    )?;
    transaction.execute(
        "UPDATE run_cancel_outbox SET delivered_at_ms = COALESCE(delivered_at_ms, ?3) \
         WHERE run_id = ?1 AND job_id = ?2",
        params![run_id, job_id, now],
    )?;
    Ok(())
}
