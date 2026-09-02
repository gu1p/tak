use anyhow::{Result, bail};
use rusqlite::{Row, Transaction, TransactionBehavior, params};
use tak_core::v2::{Affinity, ResolvedRun};

use crate::daemon::scheduler::{DispatchCommand, NodeLossResolution};

use super::super::RunStore;
use super::super::cancellation::settle_cancellation;
use super::super::events::{now_ms, sqlite_i64};
use super::{transitions, unknown};

impl RunStore {
    pub fn declare_node_lost(&self, node_id: &str) -> Result<NodeLossResolution> {
        if node_id.trim().is_empty() {
            bail!("lost node ID must not be empty");
        }
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let inserted = transaction.execute(
            "INSERT OR IGNORE INTO scheduler_node_losses (node_id, declared_at_ms) \
             VALUES (?1, ?2)",
            params![node_id, sqlite_i64(now_ms()?, "timestamp")?],
        )?;
        if inserted == 0 {
            transaction.commit()?;
            return Ok(NodeLossResolution::Duplicate);
        }
        for command in active_commands(&transaction, node_id, ActiveStates::Cancelling)? {
            settle_cancellation(&transaction, &command)?;
        }
        for binding in bindings_on_node(&transaction, node_id)? {
            let run: ResolvedRun = serde_json::from_str(&binding.resolved_run)?;
            if is_hard_group(&run, &binding.group)
                && !matches!(binding.run_state.as_str(), "cancelling" | "cancelled")
            {
                transitions::fail_affinity_group(
                    &transaction,
                    &binding.run_id,
                    &binding.group,
                    node_id,
                    &run,
                )?;
            } else if !is_hard_group(&run, &binding.group) {
                transaction.execute(
                    "DELETE FROM run_affinity_bindings \
                     WHERE run_id = ?1 AND affinity_group = ?2",
                    params![binding.run_id, binding.group],
                )?;
            }
        }
        for command in active_commands(&transaction, node_id, ActiveStates::Ambiguous)? {
            unknown::resolve_unknown_in_transaction(
                &transaction,
                &command,
                "node lost; outcome unknown; retrying",
            )?;
        }
        transaction.commit()?;
        Ok(NodeLossResolution::Applied)
    }
}

struct StoredBinding {
    run_id: String,
    group: String,
    resolved_run: String,
    run_state: String,
}

fn bindings_on_node(transaction: &Transaction<'_>, node_id: &str) -> Result<Vec<StoredBinding>> {
    let mut statement = transaction.prepare(
        "SELECT binding.run_id, binding.affinity_group, run.resolved_json, run.state \
         FROM run_affinity_bindings binding JOIN runs run USING (run_id) \
         WHERE binding.node_id = ?1 ORDER BY binding.run_id, binding.affinity_group",
    )?;
    statement
        .query_map([node_id], |row| {
            Ok(StoredBinding {
                run_id: row.get(0)?,
                group: row.get(1)?,
                resolved_run: row.get(2)?,
                run_state: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()
        .map_err(Into::into)
}

fn is_hard_group(run: &ResolvedRun, group: &str) -> bool {
    run.jobs.iter().any(|job| {
        matches!(
            &job.affinity,
            Some(Affinity::RequireSameNode { group: member }) if member == group
        )
    })
}

fn active_commands(
    transaction: &Transaction<'_>,
    node_id: &str,
    states: ActiveStates,
) -> Result<Vec<DispatchCommand>> {
    let state_filter = match states {
        ActiveStates::Cancelling => "state = 'cancelling'",
        ActiveStates::Ambiguous => "state IN ('transferring', 'running', 'output_committing')",
    };
    let mut statement = transaction.prepare(&format!(
        "SELECT run_id, job_id, node_id, transport, authored_attempt, dispatch_generation, fencing_token \
         FROM run_attempts WHERE node_id = ?1 AND released_at_ms IS NULL AND {state_filter} \
         ORDER BY run_id, job_id, authored_attempt, dispatch_generation"
    ))?;
    statement
        .query_map([node_id], command_from_row)?
        .collect::<rusqlite::Result<_>>()
        .map_err(Into::into)
}

enum ActiveStates {
    Cancelling,
    Ambiguous,
}

fn command_from_row(row: &Row<'_>) -> rusqlite::Result<DispatchCommand> {
    let attempt = row.get::<_, i64>(4)?;
    let generation = row.get::<_, i64>(5)?;
    Ok(DispatchCommand {
        run_id: row.get(0)?,
        job_id: row.get(1)?,
        node_id: row.get(2)?,
        transport: row.get(3)?,
        authored_attempt: u32::try_from(attempt)
            .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(4, attempt))?,
        dispatch_generation: u32::try_from(generation)
            .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(5, generation))?,
        fencing_token: row.get(6)?,
    })
}
