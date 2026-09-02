use anyhow::{Result, bail};
use tak_proto::worker_v2::{
    ObserveAttemptResponse, WorkerAttemptIdentity, WorkerAttemptState, WorkerOutputArtifact,
    WorkerTerminal, WorkerTerminalOutcome, encode_observe_response,
};

pub(super) fn terminal_outputs(
    transaction: &rusqlite::Transaction<'_>,
    fence: &str,
) -> Result<Vec<WorkerOutputArtifact>> {
    let mut statement = transaction.prepare(
        "SELECT artifact_id,producer_task_id,entry_json FROM worker_v2_outputs \
         WHERE fencing_token=?1 ORDER BY producer_task_id,path",
    )?;
    statement
        .query_map([fence], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .map(|row| {
            let (artifact_id, producer_task_id, entry) = row?;
            Ok(WorkerOutputArtifact {
                artifact_id,
                producer_task_id,
                entry: serde_json::from_str(&entry)?,
            })
        })
        .collect()
}

pub(super) fn event_watermark(transaction: &rusqlite::Transaction<'_>, fence: &str) -> Result<u64> {
    let watermark: i64 = transaction.query_row(
        "SELECT COALESCE(MAX(seq),0) FROM worker_v2_events WHERE fencing_token=?1",
        [fence],
        |row| row.get(0),
    )?;
    Ok(u64::try_from(watermark)?)
}

pub(super) fn validate_terminal(
    identity: &WorkerAttemptIdentity,
    terminal: &WorkerTerminal,
) -> Result<()> {
    encode_observe_response(&ObserveAttemptResponse {
        protocol_version: 2,
        fencing_token: identity.fencing_token.clone(),
        state: WorkerAttemptState::Completed,
        events: vec![],
        next_event: terminal.event_watermark,
        terminal: Some(terminal.clone()),
    })
    .map(|_| ())
}

pub(super) fn terminal_state(outcome: WorkerTerminalOutcome) -> &'static str {
    match outcome {
        WorkerTerminalOutcome::Succeeded => "succeeded",
        WorkerTerminalOutcome::Failed => "failed",
        WorkerTerminalOutcome::Cancelled => "cancelled",
    }
}

pub(super) fn existing_terminal(
    transaction: &rusqlite::Transaction<'_>,
    identity: &WorkerAttemptIdentity,
    outcome: WorkerTerminalOutcome,
    digest: &str,
    exit_code: Option<i32>,
) -> Result<WorkerTerminal> {
    let encoded: String = transaction.query_row(
        "SELECT terminal_json FROM worker_v2_attempts WHERE fencing_token=?1",
        [&identity.fencing_token],
        |row| row.get(0),
    )?;
    let terminal: WorkerTerminal = serde_json::from_str(&encoded)?;
    if terminal.outcome == outcome
        && terminal.terminal_digest == digest
        && terminal.exit_code == exit_code
    {
        return Ok(terminal);
    }
    bail!("conflicting worker terminal result")
}
