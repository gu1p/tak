use anyhow::Result;
use rusqlite::TransactionBehavior;
use tak_core::v2::ResolvedJob;

use crate::daemon::scheduler::{DispatchCommand, SchedulerNode};

use super::RunStore;

mod queries;
mod reservation;
mod selection;

use queries::{active_run_attempts, policy_cursor, ready_jobs, validate_nodes};
use reservation::{reserve, save_cursor};
use selection::select_node;

impl RunStore {
    pub fn reserve_next(&self, nodes: &[SchedulerNode]) -> Result<Option<DispatchCommand>> {
        validate_nodes(nodes)?;
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let ready = ready_jobs(&transaction)?;
        for (run_id, job_id, definition, max_parallel) in ready {
            if active_run_attempts(&transaction, &run_id)? >= max_parallel {
                continue;
            }
            let job: ResolvedJob = serde_json::from_str(&definition)?;
            let cursor = policy_cursor(&transaction, &run_id, &job)?;
            let Some((node, next_cursor)) = select_node(&transaction, nodes, &job, cursor)? else {
                continue;
            };
            let command = reserve(&transaction, &run_id, &job_id, &job, node)?;
            if let Some(next_cursor) = next_cursor {
                save_cursor(
                    &transaction,
                    &run_id,
                    &job.placement_policy.policy_id,
                    next_cursor,
                )?;
            }
            transaction.commit()?;
            return Ok(Some(command));
        }
        transaction.commit()?;
        Ok(None)
    }

    pub fn pending_dispatches(&self) -> Result<Vec<DispatchCommand>> {
        let connection = self.open_connection()?;
        let mut statement = connection.prepare(
            "SELECT payload_json FROM run_dispatch_outbox WHERE delivered_at_ms IS NULL \
             ORDER BY rowid",
        )?;
        statement
            .query_map([], |row| row.get::<_, String>(0))?
            .map(|payload| Ok(serde_json::from_str(&payload?)?))
            .collect()
    }
}
