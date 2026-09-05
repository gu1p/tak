use anyhow::{Result, bail};
use rusqlite::{Transaction, params};
use tak_core::v2::ResolvedJob;
use tak_proto::local_daemon::v2::RunEventKind;

use crate::daemon::scheduler::{DispatchCommand, SchedulerNode};

use super::super::events::{JobEventDetails, append_job_event, now_ms, sqlite_i64};

pub(super) fn reserve(
    transaction: &Transaction<'_>,
    run_id: &str,
    job_id: &str,
    job: &ResolvedJob,
    node: &SchedulerNode,
) -> Result<DispatchCommand> {
    let authored_attempt = next_authored_attempt(transaction, run_id, job_id)?;
    let dispatch_generation = 1_u32;
    let fencing_token = uuid::Uuid::new_v4().to_string();
    let transport = job
        .placement_candidates
        .iter()
        .find(|candidate| candidate.node_id == node.node_id)
        .ok_or_else(|| anyhow::anyhow!("selected node is not a placement candidate"))?
        .transport
        .clone();
    let command = DispatchCommand {
        run_id: run_id.to_owned(),
        job_id: job_id.to_owned(),
        node_id: node.node_id.clone(),
        authored_attempt,
        dispatch_generation,
        fencing_token,
        transport,
    };
    let now = sqlite_i64(now_ms()?, "timestamp")?;
    transaction.execute(
        "INSERT INTO run_attempts (run_id, job_id, authored_attempt, dispatch_generation, fencing_token, node_id, transport, state, cpu_millis, memory_bytes, execution_slots, reserved_at_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'transferring', ?8, ?9, ?10, ?11)",
        params![run_id, job_id, authored_attempt, dispatch_generation, command.fencing_token,
            node.node_id, command.transport, sqlite_i64(job.resources.cpu_millis, "CPU reservation")?,
            sqlite_i64(job.resources.memory_bytes, "memory reservation")?,
            i64::from(job.resources.execution_slots.get()), now],
    )?;
    super::constraints::bind_affinity_home(transaction, run_id, job, &node.node_id, now)?;
    let updated = transaction.execute(
        "UPDATE run_jobs SET state = 'transferring', node_id = ?3, attempt = ?4, cache = NULL, dispatch_generation = ?5, current_fencing_token = ?6, next_eligible_at_ms = 0 WHERE run_id = ?1 AND job_id = ?2 AND state IN ('ready', 'retrying')",
        params![run_id, job_id, node.node_id, authored_attempt, dispatch_generation,
            command.fencing_token],
    )?;
    if updated != 1 {
        bail!("ready job changed during reservation");
    }
    transaction.execute(
        "UPDATE runs SET state = 'running', updated_at_ms = ?2 WHERE run_id = ?1",
        params![run_id, now],
    )?;
    append_job_event(
        transaction,
        run_id,
        RunEventKind::Transferring,
        job_id,
        &job.task_ids,
        &node.node_id,
        JobEventDetails::for_attempt("job reserved and transferring", authored_attempt),
    )?;
    transaction.execute(
        "INSERT INTO run_dispatch_outbox (run_id, job_id, authored_attempt, dispatch_generation, fencing_token, payload_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![run_id, job_id, authored_attempt, dispatch_generation, command.fencing_token,
            serde_json::to_string(&command)?],
    )?;
    Ok(command)
}

fn next_authored_attempt(transaction: &Transaction<'_>, run_id: &str, job_id: &str) -> Result<u32> {
    let next = transaction.query_row(
        "SELECT COALESCE(MAX(authored_attempt), 0) + 1 FROM run_attempts WHERE run_id = ?1 AND job_id = ?2",
        params![run_id, job_id],
        |row| row.get::<_, i64>(0),
    )?;
    u32::try_from(next).map_err(|_| anyhow::anyhow!("authored attempt overflow"))
}

pub(super) fn save_cursor(
    transaction: &Transaction<'_>,
    run_id: &str,
    policy_id: &str,
    next: u64,
) -> Result<()> {
    transaction.execute(
        "INSERT INTO run_policy_cursors (run_id, policy_id, next_assignment) VALUES (?1, ?2, ?3) \
         ON CONFLICT(run_id, policy_id) DO UPDATE SET next_assignment = excluded.next_assignment",
        params![run_id, policy_id, sqlite_i64(next, "round-robin cursor")?],
    )?;
    Ok(())
}

pub(super) fn advance_fairness(
    transaction: &Transaction<'_>,
    run_id: &str,
    submitter_id: &str,
) -> Result<()> {
    let turn = transaction.query_row(
        "SELECT next_turn FROM scheduler_state WHERE singleton = 1",
        [],
        |row| row.get::<_, i64>(0),
    )?;
    let next = turn
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("scheduler fairness turn overflow"))?;
    transaction.execute(
        "UPDATE scheduler_state SET next_turn = ?1 WHERE singleton = 1",
        [next],
    )?;
    transaction.execute(
        "INSERT INTO scheduler_submitters (submitter_id, last_scheduled_turn) VALUES (?1, ?2) \
         ON CONFLICT(submitter_id) DO UPDATE SET last_scheduled_turn = excluded.last_scheduled_turn",
        params![submitter_id, turn],
    )?;
    transaction.execute(
        "UPDATE runs SET last_scheduled_turn = ?2 WHERE run_id = ?1",
        params![run_id, turn],
    )?;
    Ok(())
}
