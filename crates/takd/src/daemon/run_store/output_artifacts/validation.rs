use std::path::Path;

use anyhow::{Result, bail};
use ignore::gitignore::GitignoreBuilder;
use rusqlite::{OptionalExtension, Transaction, params};
use tak_core::v2::{OutputSelector, ResolvedRun, WorkspaceEntry, WorkspaceEntryType};

use crate::daemon::scheduler::DispatchCommand;

pub(super) fn current_run(
    transaction: &Transaction<'_>,
    command: &DispatchCommand,
) -> Result<Option<ResolvedRun>> {
    let encoded = transaction
        .query_row(
            "SELECT run.resolved_json FROM run_attempts attempt JOIN run_jobs job \
             USING (run_id,job_id) JOIN runs run USING (run_id) WHERE attempt.run_id=?1 \
             AND attempt.job_id=?2 AND attempt.authored_attempt=?3 \
             AND attempt.dispatch_generation=?4 AND attempt.fencing_token=?5 \
             AND attempt.node_id=?6 AND attempt.transport IS ?7 \
             AND attempt.state IN ('transferring','running','output_committing') \
             AND attempt.released_at_ms IS NULL AND job.current_fencing_token=?5",
            params![
                command.run_id,
                command.job_id,
                command.authored_attempt,
                command.dispatch_generation,
                command.fencing_token,
                command.node_id,
                command.transport
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    encoded
        .map(|value| serde_json::from_str(&value).map_err(Into::into))
        .transpose()
}

pub(super) fn declarations(
    run: &ResolvedRun,
    command: &DispatchCommand,
    producer: &str,
    entries: &[WorkspaceEntry],
) -> Result<()> {
    let task = run
        .tasks
        .iter()
        .find(|task| task.task_id == producer && task.job_id == command.job_id)
        .ok_or_else(|| anyhow::anyhow!("remote output producer is outside the dispatched job"))?;
    let job = run
        .jobs
        .iter()
        .find(|job| job.job_id == command.job_id)
        .ok_or_else(|| anyhow::anyhow!("resolved remote job is missing"))?;
    if !job.task_ids.contains(&task.task_id) {
        bail!("remote output producer is outside the dispatched job");
    }
    for entry in entries {
        if !declared(&task.outputs, entry)? {
            bail!("remote output path is not declared by its producer task");
        }
    }
    Ok(())
}

fn declared(selectors: &[OutputSelector], entry: &WorkspaceEntry) -> Result<bool> {
    for selector in selectors {
        if matches(selector, entry)? {
            return Ok(true);
        }
    }
    Ok(false)
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
