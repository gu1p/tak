use anyhow::Result;
use rusqlite::{Transaction, params};
use tak_core::v2::RunSubmission;
use tak_proto::local_daemon::v2::WorkspaceDisposition;

use super::super::events::{now_ms, sqlite_i64};

pub(super) fn insert_run(
    transaction: &Transaction<'_>,
    run_id: &str,
    submission: &RunSubmission,
    submitter_id: &str,
    workspace: &WorkspaceDisposition,
) -> Result<()> {
    let now = sqlite_i64(now_ms()?, "timestamp")?;
    let state = match workspace {
        WorkspaceDisposition::Present => "awaiting_commit",
        WorkspaceDisposition::UploadRequired { .. } => "awaiting_workspace",
    };
    let descriptor = &submission.run.workspace;
    let upload_offset = match workspace {
        WorkspaceDisposition::Present => descriptor.archive_size,
        WorkspaceDisposition::UploadRequired { next_offset } => *next_offset,
    };
    transaction.execute(
        "INSERT INTO runs (run_id, submitter_id, idempotency_key, request_digest, state, project_id, targets_json, resolved_json, max_parallel_jobs, workspace_fingerprint, archive_sha256, archive_size, upload_offset, created_at_ms, updated_at_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
        params![
            run_id,
            submitter_id,
            submission.idempotency_key,
            submission.request_digest(),
            state,
            submission.run.project_id,
            serde_json::to_string(&submission.run.targets)?,
            serde_json::to_string(&submission.run)?,
            i64::from(submission.run.options.max_parallel_jobs.get()),
            descriptor.manifest.fingerprint,
            descriptor.archive_sha256,
            sqlite_i64(descriptor.archive_size, "workspace archive size")?,
            sqlite_i64(upload_offset, "workspace upload offset")?,
            now
        ],
    )?;
    Ok(())
}

pub(super) fn insert_environment(
    transaction: &Transaction<'_>,
    run_id: &str,
    submission: &RunSubmission,
) -> Result<()> {
    for entry in &submission.environment_values {
        transaction.execute(
            "INSERT INTO run_environment (run_id, name, value) VALUES (?1, ?2, ?3)",
            params![run_id, entry.name, entry.value],
        )?;
    }
    Ok(())
}

pub(super) fn insert_jobs(
    transaction: &Transaction<'_>,
    run_id: &str,
    submission: &RunSubmission,
) -> Result<()> {
    for (ordinal, job) in submission.run.jobs.iter().enumerate() {
        transaction.execute(
            "INSERT INTO run_jobs (run_id, job_id, ordinal, state, definition_json) VALUES (?1, ?2, ?3, 'staged', ?4)",
            params![
                run_id,
                job.job_id,
                sqlite_i64(ordinal as u64, "job ordinal")?,
                serde_json::to_string(job)?
            ],
        )?;
    }
    Ok(())
}

pub(super) fn insert_edges(
    transaction: &Transaction<'_>,
    run_id: &str,
    submission: &RunSubmission,
) -> Result<()> {
    for edge in &submission.run.job_edges {
        transaction.execute(
            "INSERT INTO run_dependencies (run_id, dependency_job_id, dependent_job_id) VALUES (?1, ?2, ?3)",
            params![run_id, edge.dependency_job_id, edge.dependent_job_id],
        )?;
    }
    Ok(())
}
