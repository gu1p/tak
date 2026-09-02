use anyhow::{Result, bail};
use base64::Engine;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use tak_core::v2::ResolvedJob;
use tak_proto::local_daemon::v2::RunEventKind;
use tak_proto::worker_v2::{WorkerAttemptEvent, WorkerOutputStream};

use crate::daemon::scheduler::{DispatchCommand, ResultAcceptance};

use super::super::RunStore;
use super::super::events::append_output_event;

const MAX_EVENT_BYTES: usize = 64 * 1024;

impl RunStore {
    pub(in crate::daemon) fn worker_event_cursor(
        &self,
        command: &DispatchCommand,
    ) -> Result<Option<u64>> {
        let connection = self.open_connection()?;
        let value = connection
            .query_row(
                "SELECT attempt.worker_event_cursor FROM run_attempts attempt JOIN run_jobs job \
                 USING (run_id,job_id) WHERE attempt.run_id=?1 AND attempt.job_id=?2 AND \
                 attempt.authored_attempt=?3 AND attempt.dispatch_generation=?4 AND \
                 attempt.fencing_token=?5 AND attempt.node_id=?6 AND attempt.transport IS ?7 AND \
                 attempt.state IN ('transferring','running','output_committing') AND \
                 attempt.released_at_ms IS NULL AND job.current_fencing_token=?5",
                params![
                    command.run_id,
                    command.job_id,
                    command.authored_attempt,
                    command.dispatch_generation,
                    command.fencing_token,
                    command.node_id,
                    command.transport
                ],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        value.map(u64::try_from).transpose().map_err(Into::into)
    }

    pub(in crate::daemon) fn ingest_worker_events(
        &self,
        command: &DispatchCommand,
        after_event: u64,
        events: &[WorkerAttemptEvent],
        next_event: u64,
    ) -> Result<ResultAcceptance> {
        validate_sequence(after_event, events, next_event)?;
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let stored = transaction
            .query_row(
                "SELECT attempt.worker_event_cursor,job.definition_json FROM run_attempts attempt \
             JOIN run_jobs job USING (run_id,job_id) WHERE attempt.run_id=?1 AND attempt.job_id=?2 \
             AND attempt.authored_attempt=?3 AND attempt.dispatch_generation=?4 \
             AND attempt.fencing_token=?5 AND attempt.node_id=?6 AND attempt.transport IS ?7 AND \
             attempt.state IN ('transferring','running','output_committing') AND \
             attempt.released_at_ms IS NULL AND job.current_fencing_token=?5",
                params![
                    command.run_id,
                    command.job_id,
                    command.authored_attempt,
                    command.dispatch_generation,
                    command.fencing_token,
                    command.node_id,
                    command.transport
                ],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        let Some((cursor, definition)) = stored else {
            return Ok(ResultAcceptance::Stale);
        };
        let cursor = u64::try_from(cursor)?;
        if cursor == next_event {
            return Ok(ResultAcceptance::Duplicate);
        }
        if cursor != after_event {
            return Ok(ResultAcceptance::Stale);
        }
        let job: ResolvedJob = serde_json::from_str(&definition)?;
        for event in events {
            if !job.task_ids.contains(&event.task_id) {
                bail!("worker event task is outside the job")
            }
            let bytes = decode_event(event)?;
            append_output_event(
                &transaction,
                &command.run_id,
                match event.stream {
                    WorkerOutputStream::Stdout => RunEventKind::Stdout,
                    WorkerOutputStream::Stderr => RunEventKind::Stderr,
                },
                &command.job_id,
                std::slice::from_ref(&event.task_id),
                &command.node_id,
                &bytes,
            )?;
        }
        transaction.execute(
            "UPDATE run_attempts SET worker_event_cursor=?6 WHERE run_id=?1 AND job_id=?2 \
             AND authored_attempt=?3 AND dispatch_generation=?4 AND fencing_token=?5",
            params![
                command.run_id,
                command.job_id,
                command.authored_attempt,
                command.dispatch_generation,
                command.fencing_token,
                i64::try_from(next_event)?
            ],
        )?;
        transaction.commit()?;
        Ok(ResultAcceptance::Applied)
    }
}

fn validate_sequence(after: u64, events: &[WorkerAttemptEvent], next: u64) -> Result<()> {
    let mut expected = after;
    for event in events {
        expected = expected
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("worker event cursor overflow"))?;
        if event.seq != expected {
            bail!("worker event sequence contains a gap")
        }
    }
    if expected != next {
        bail!("worker event cursor does not match its events")
    }
    Ok(())
}

fn decode_event(event: &WorkerAttemptEvent) -> Result<Vec<u8>> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(&event.chunk_base64)?;
    if bytes.len() > MAX_EVENT_BYTES
        || format!("{:x}", Sha256::digest(&bytes)) != event.chunk_sha256
    {
        bail!("worker event chunk is invalid");
    }
    Ok(bytes)
}
