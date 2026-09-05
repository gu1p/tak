use anyhow::Result;
use rusqlite::{TransactionBehavior, params};
use tak_proto::local_daemon::v2::RunEventKind;

use crate::daemon::scheduler::{DispatchCommand, ResultAcceptance};

use super::super::RunStore;
use super::super::events::{JobEventDetails, append_job_event, now_ms, sqlite_i64};
use super::{load_attempt, load_job};

impl RunStore {
    pub fn begin_output_commit(&self, command: &DispatchCommand) -> Result<ResultAcceptance> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(attempt) = load_attempt(&transaction, command)? else {
            return Ok(ResultAcceptance::Stale);
        };
        if !attempt.matches(command) {
            return Ok(ResultAcceptance::Stale);
        }
        if attempt.state == "output_committing" {
            return Ok(ResultAcceptance::Duplicate);
        }
        if !matches!(attempt.state.as_str(), "transferring" | "running") {
            return Ok(ResultAcceptance::Stale);
        }
        let job = load_job(&transaction, command)?;
        let now = sqlite_i64(now_ms()?, "timestamp")?;
        if attempt.state == "transferring" {
            append_job_event(
                &transaction,
                &command.run_id,
                RunEventKind::Running,
                &command.job_id,
                &job.task_ids,
                &command.node_id,
                JobEventDetails::new("worker accepted job"),
            )?;
        }
        transaction.execute(
            "UPDATE run_attempts SET state='output_committing', dispatch_started_at_ms=COALESCE(dispatch_started_at_ms,?4), accepted_at_ms=COALESCE(accepted_at_ms,?4) WHERE run_id=?1 AND fencing_token=?2 AND released_at_ms IS NULL AND state IN ('transferring','running') AND job_id=?3",
            params![command.run_id, command.fencing_token, command.job_id, now],
        )?;
        transaction.execute(
            "UPDATE run_jobs SET state='output_committing' WHERE run_id=?1 AND job_id=?2 AND current_fencing_token=?3",
            params![command.run_id, command.job_id, command.fencing_token],
        )?;
        transaction.execute(
            "UPDATE run_dispatch_outbox SET delivered_at_ms=COALESCE(delivered_at_ms,?3) WHERE run_id=?1 AND fencing_token=?2",
            params![command.run_id, command.fencing_token, now],
        )?;
        append_job_event(
            &transaction,
            &command.run_id,
            RunEventKind::OutputCommitting,
            &command.job_id,
            &job.task_ids,
            &command.node_id,
            JobEventDetails::new("worker is committing declared outputs"),
        )?;
        transaction.commit()?;
        Ok(ResultAcceptance::Applied)
    }
}
