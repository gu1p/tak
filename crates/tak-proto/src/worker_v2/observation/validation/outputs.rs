use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use tak_core::v2::WorkspaceManifest;

use super::{valid_digest, valid_identifier};
use crate::worker_v2::{WorkerTerminal, WorkerTerminalOutcome};

pub(super) fn validate_terminal(terminal: &WorkerTerminal, next_event: u64) -> Result<()> {
    if !valid_digest(&terminal.terminal_digest)
        || terminal.event_watermark != next_event
        || (terminal.outcome != WorkerTerminalOutcome::Succeeded && !terminal.outputs.is_empty())
        || !valid_exit_code(terminal)
        || !valid_runtime(terminal)
    {
        bail!("worker terminal record is invalid");
    }
    let mut producers = BTreeMap::<&str, Vec<_>>::new();
    let mut artifact_ids = BTreeSet::new();
    for output in &terminal.outputs {
        if !valid_identifier(&output.artifact_id)
            || !valid_identifier(&output.producer_task_id)
            || !artifact_ids.insert(output.artifact_id.as_str())
        {
            bail!("worker output manifest is invalid");
        }
        producers
            .entry(&output.producer_task_id)
            .or_default()
            .push(output.entry.clone());
    }
    if !terminal.outputs.windows(2).all(|pair| {
        (&pair[0].producer_task_id, &pair[0].entry.path)
            < (&pair[1].producer_task_id, &pair[1].entry.path)
    }) {
        bail!("worker output manifest is not canonical");
    }
    for entries in producers.into_values() {
        if WorkspaceManifest::new(entries.clone())?.entries != entries {
            bail!("worker producer output manifest is not canonical");
        }
    }
    Ok(())
}

fn valid_runtime(terminal: &WorkerTerminal) -> bool {
    match (&terminal.runtime_kind, &terminal.runtime_engine) {
        (None, None) => true,
        (Some(kind), Some(engine)) => valid_identifier(kind) && valid_identifier(engine),
        _ => false,
    }
}

fn valid_exit_code(terminal: &WorkerTerminal) -> bool {
    match terminal.outcome {
        WorkerTerminalOutcome::Succeeded => terminal.exit_code.is_none_or(|code| code == 0),
        WorkerTerminalOutcome::Failed => terminal
            .exit_code
            .is_none_or(|code| (1..=u8::MAX.into()).contains(&code)),
        WorkerTerminalOutcome::Cancelled => terminal.exit_code.is_none(),
    }
}
