use anyhow::Result;
use rusqlite::{Transaction, params};
use tak_core::v2::ResolvedJob;
use tak_proto::local_daemon::v2::RunEventKind;

use crate::daemon::scheduler::{DispatchCommand, SchedulerNode};

use super::super::events::{append_job_event, now_ms, sqlite_i64};

pub(super) fn reserve(
    transaction: &Transaction<'_>,
    run_id: &str,
    job_id: &str,
    job: &ResolvedJob,
    node: &SchedulerNode,
) -> Result<DispatchCommand> {
    let authored_attempt = 1_u32;
    let dispatch_generation = 1_u32;
    let fencing_token = uuid::Uuid::new_v4().to_string();
    let command = DispatchCommand {
        run_id: run_id.to_owned(),
        job_id: job_id.to_owned(),
        node_id: node.node_id.clone(),
        authored_attempt,
        dispatch_generation,
        fencing_token,
    };
    let now = sqlite_i64(now_ms()?, "timestamp")?;
    transaction.execute(
        "INSERT INTO run_attempts (run_id, job_id, authored_attempt, dispatch_generation, fencing_token, node_id, state, cpu_millis, memory_bytes, execution_slots, reserved_at_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'transferring', ?7, ?8, ?9, ?10)",
        params![run_id, job_id, authored_attempt, dispatch_generation, command.fencing_token,
            node.node_id, sqlite_i64(job.resources.cpu_millis, "CPU reservation")?,
            sqlite_i64(job.resources.memory_bytes, "memory reservation")?,
            i64::from(job.resources.execution_slots.get()), now],
    )?;
    transaction.execute(
        "UPDATE run_jobs SET state = 'transferring', node_id = ?3, attempt = ?4 WHERE run_id = ?1 AND job_id = ?2 AND state = 'ready'",
        params![run_id, job_id, node.node_id, authored_attempt],
    )?;
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
        "job reserved and transferring",
    )?;
    transaction.execute(
        "INSERT INTO run_dispatch_outbox (run_id, job_id, authored_attempt, dispatch_generation, fencing_token, payload_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![run_id, job_id, authored_attempt, dispatch_generation, command.fencing_token,
            serde_json::to_string(&command)?],
    )?;
    Ok(command)
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
