use anyhow::{Result, bail};
use rusqlite::{TransactionBehavior, params};
use tak_core::v2::ResolvedJob;
use tak_proto::local_daemon::v2::RunEventKind;

use super::RunStore;
use super::events::{JobEventDetails, append_job_event};
use crate::daemon::scheduler::DispatchCommand;

impl RunStore {
    pub(in crate::daemon) fn record_worker_cache(
        &self,
        command: &DispatchCommand,
        cache: &str,
    ) -> Result<()> {
        if !matches!(cache, "hit" | "miss") {
            bail!("worker cache state is invalid");
        }
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (definition, previous) = transaction.query_row(
            "SELECT definition_json,cache FROM run_jobs WHERE run_id=?1 AND job_id=?2 \
             AND current_fencing_token=?3",
            params![command.run_id, command.job_id, command.fencing_token],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )?;
        if previous.as_deref() == Some(cache) {
            transaction.commit()?;
            return Ok(());
        }
        let job: ResolvedJob = serde_json::from_str(&definition)?;
        let updated = transaction.execute(
            "UPDATE run_jobs SET cache=?4 WHERE run_id=?1 AND job_id=?2 \
             AND current_fencing_token=?3",
            params![command.run_id, command.job_id, command.fencing_token, cache],
        )?;
        if updated != 1 {
            bail!("worker cache state fence is no longer current");
        }
        append_job_event(
            &transaction,
            &command.run_id,
            RunEventKind::Transferring,
            &command.job_id,
            &job.task_ids,
            &command.node_id,
            JobEventDetails::new(&format!("workspace cache {cache}")),
        )?;
        transaction.commit()?;
        Ok(())
    }
}
