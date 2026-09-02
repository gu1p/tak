use anyhow::Result;
use rusqlite::{TransactionBehavior, params};

use crate::daemon::scheduler::{DispatchCommand, ResultAcceptance};

use super::super::RunStore;
use super::super::events::{now_ms, sqlite_i64};
use super::load_attempt;

impl RunStore {
    pub fn mark_dispatch_started(&self, command: &DispatchCommand) -> Result<ResultAcceptance> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(attempt) = load_attempt(&transaction, command)? else {
            return Ok(ResultAcceptance::Stale);
        };
        if !attempt.matches(command) || attempt.state != "transferring" {
            return Ok(ResultAcceptance::Stale);
        }
        if attempt.dispatch_started_at_ms.is_some() {
            return Ok(ResultAcceptance::Duplicate);
        }
        let updated = transaction.execute(
            "UPDATE run_attempts SET dispatch_started_at_ms=?6 WHERE run_id=?1 AND job_id=?2 AND authored_attempt=?3 AND dispatch_generation=?4 AND fencing_token=?5 AND state='transferring' AND dispatch_started_at_ms IS NULL",
            params![command.run_id, command.job_id, command.authored_attempt,
                command.dispatch_generation, command.fencing_token,
                sqlite_i64(now_ms()?, "timestamp")?],
        )?;
        if updated != 1 {
            return Ok(ResultAcceptance::Stale);
        }
        transaction.commit()?;
        Ok(ResultAcceptance::Applied)
    }
}
