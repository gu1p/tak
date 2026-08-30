use anyhow::{Result, bail};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use tak_core::v2::ResolvedJob;
use tak_proto::local_daemon::v2::RunEventKind;

use crate::daemon::scheduler::{AttemptOutputStream, DispatchCommand, ResultAcceptance};

use super::RunStore;
use super::events::append_output_event;

const MAX_OUTPUT_EVENT_BYTES: usize = 64 * 1024;

impl RunStore {
    pub fn append_attempt_output(
        &self,
        command: &DispatchCommand,
        task_id: &str,
        stream: AttemptOutputStream,
        bytes: &[u8],
    ) -> Result<ResultAcceptance> {
        if bytes.len() > MAX_OUTPUT_EVENT_BYTES {
            bail!("attempt output chunk exceeds the durable event limit");
        }
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let definition = transaction
            .query_row(
                "SELECT job.definition_json FROM run_attempts attempt \
                 JOIN run_jobs job USING (run_id, job_id) WHERE attempt.run_id=?1 \
                 AND attempt.job_id=?2 AND attempt.authored_attempt=?3 \
                 AND attempt.dispatch_generation=?4 AND attempt.fencing_token=?5 \
                 AND attempt.node_id=?6 AND attempt.state IN ('transferring','running') \
                 AND attempt.released_at_ms IS NULL \
                 AND job.current_fencing_token=attempt.fencing_token",
                params![
                    command.run_id,
                    command.job_id,
                    command.authored_attempt,
                    command.dispatch_generation,
                    command.fencing_token,
                    command.node_id
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(definition) = definition else {
            return Ok(ResultAcceptance::Stale);
        };
        let job: ResolvedJob = serde_json::from_str(&definition)?;
        if !job.task_ids.iter().any(|candidate| candidate == task_id) {
            bail!("attempt output task does not belong to the fenced job");
        }
        if bytes.is_empty() {
            return Ok(ResultAcceptance::Applied);
        }
        append_output_event(
            &transaction,
            &command.run_id,
            match stream {
                AttemptOutputStream::Stdout => RunEventKind::Stdout,
                AttemptOutputStream::Stderr => RunEventKind::Stderr,
            },
            &command.job_id,
            &[task_id.to_owned()],
            &command.node_id,
            bytes,
        )?;
        transaction.commit()?;
        Ok(ResultAcceptance::Applied)
    }
}
