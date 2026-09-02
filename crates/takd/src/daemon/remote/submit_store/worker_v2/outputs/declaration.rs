use std::path::Path;

use anyhow::{Result, bail};
use ignore::gitignore::GitignoreBuilder;
use rusqlite::params;
use tak_core::v2::{OutputSelector, WorkspaceEntry, WorkspaceEntryType};
use tak_proto::worker_v2::{WorkerAttemptIdentity, decode_dispatch_request};

pub(super) fn validate(
    transaction: &rusqlite::Transaction<'_>,
    identity: &WorkerAttemptIdentity,
    producer: &str,
    entry: &WorkspaceEntry,
) -> Result<()> {
    let request: String = transaction.query_row(
        "SELECT request_json FROM worker_v2_attempts WHERE run_id=?1 AND job_id=?2 AND \
         authored_attempt=?3 AND dispatch_generation=?4 AND fencing_token=?5 AND node_id=?6",
        params![
            identity.run_id,
            identity.job_id,
            identity.authored_attempt,
            identity.dispatch_generation,
            identity.fencing_token,
            identity.node_id
        ],
        |row| row.get(0),
    )?;
    let request = decode_dispatch_request(request.as_bytes())?;
    let task = request
        .payload
        .tasks
        .iter()
        .find(|task| task.task_id == producer)
        .ok_or_else(|| anyhow::anyhow!("worker output producer is outside the dispatched job"))?;
    for selector in &task.outputs {
        if matches(selector, entry)? {
            return Ok(());
        }
    }
    bail!("worker output path is not declared by its producer task")
}

fn matches(selector: &OutputSelector, entry: &WorkspaceEntry) -> Result<bool> {
    match selector {
        OutputSelector::Path { value } => Ok(entry.path == *value
            || entry
                .path
                .strip_prefix(value)
                .is_some_and(|suffix| suffix.starts_with('/'))),
        OutputSelector::Glob { value } => {
            let mut builder = GitignoreBuilder::new(".");
            builder.add_line(None, value)?;
            let matcher = builder.build()?;
            Ok(matcher
                .matched(
                    Path::new(&entry.path),
                    entry.entry_type == WorkspaceEntryType::Directory,
                )
                .is_ignore())
        }
    }
}
