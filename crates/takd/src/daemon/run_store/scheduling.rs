use anyhow::Result;
use rusqlite::TransactionBehavior;
use tak_core::v2::ResolvedJob;

use crate::daemon::scheduler::{DispatchCommand, SchedulerNode};

use super::RunStore;
use super::events::now_ms;

mod queries;
mod reservation;
mod selection;

use queries::{active_run_attempts, policy_cursor, ready_jobs, validate_nodes};
use reservation::{advance_fairness, reserve, save_cursor};
use selection::select_node;

impl RunStore {
    pub fn reserve_next(&self, nodes: &[SchedulerNode]) -> Result<Option<DispatchCommand>> {
        validate_nodes(nodes)?;
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let ready = ready_jobs(&transaction, now_ms()?)?;
        for ready_job in ready {
            if active_run_attempts(&transaction, &ready_job.run_id)? >= ready_job.max_parallel {
                continue;
            }
            let job: ResolvedJob = serde_json::from_str(&ready_job.definition)?;
            let cursor = policy_cursor(&transaction, &ready_job.run_id, &job)?;
            let Some((node, next_cursor)) = select_node(
                &transaction,
                nodes,
                &job,
                cursor,
                &ready_job.workspace_fingerprint,
            )?
            else {
                continue;
            };
            let command = reserve(
                &transaction,
                &ready_job.run_id,
                &ready_job.job_id,
                &job,
                node,
            )?;
            if let Some(next_cursor) = next_cursor {
                save_cursor(
                    &transaction,
                    &ready_job.run_id,
                    &job.placement_policy.policy_id,
                    next_cursor,
                )?;
            }
            advance_fairness(&transaction, &ready_job.run_id, &ready_job.submitter_id)?;
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
