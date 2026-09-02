use anyhow::{Result, bail};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use tak_core::v2::{Affinity, EnvironmentValue, ResolvedRun, SessionReuse};
use tak_proto::worker_v2::WorkerWorkspaceReuse;

use super::*;
use crate::daemon::run_store::blob::verified_workspace_blob;

impl RunStore {
    pub(in crate::daemon) fn remote_execution_snapshot(
        &self,
        command: &DispatchCommand,
    ) -> Result<RemoteExecutionSnapshot> {
        if command.transport.is_none() {
            bail!("remote attempt has no persisted transport");
        }
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let stored = transaction
            .query_row(
                "SELECT run.resolved_json,run.workspace_fingerprint FROM run_attempts attempt \
             JOIN run_jobs job USING (run_id,job_id) JOIN runs run USING (run_id) \
             WHERE attempt.run_id=?1 AND attempt.job_id=?2 AND attempt.authored_attempt=?3 \
             AND attempt.dispatch_generation=?4 AND attempt.fencing_token=?5 \
             AND attempt.node_id=?6 AND attempt.transport IS ?7 \
             AND attempt.state IN ('transferring','running') AND attempt.released_at_ms IS NULL \
             AND job.current_fencing_token=attempt.fencing_token",
                params![
                    command.run_id,
                    command.job_id,
                    command.authored_attempt,
                    command.dispatch_generation,
                    command.fencing_token,
                    command.node_id,
                    command.transport
                ],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        let Some((resolved, fingerprint)) = stored else {
            bail!("remote attempt fence is no longer current");
        };
        let run: ResolvedRun = serde_json::from_str(&resolved)?;
        let job = run
            .jobs
            .iter()
            .find(|job| job.job_id == command.job_id)
            .ok_or_else(|| anyhow::anyhow!("resolved remote job is missing"))?;
        let tasks = job
            .task_ids
            .iter()
            .map(|task_id| {
                run.tasks
                    .iter()
                    .find(|task| task.task_id == *task_id && task.job_id == job.job_id)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("resolved remote task `{task_id}` is missing"))
            })
            .collect::<Result<Vec<_>>>()?;
        let environment_values = environment_values(&transaction, &command.run_id)?;
        let workspace_blob = verified_workspace_blob(self, &transaction, &fingerprint)?
            .ok_or_else(|| anyhow::anyhow!("verified workspace blob is missing"))?;
        let consumer = job
            .task_ids
            .first()
            .ok_or_else(|| anyhow::anyhow!("resolved remote job has no tasks"))?;
        let overlays = dependency_overlays(self, &transaction, &command.run_id, &run, consumer)?;
        let workspace_reuse = workspace_reuse(job)?;
        let mut descriptor = run.workspace;
        descriptor.archive_sha256 = workspace_blob.archive_sha256;
        descriptor.archive_size = workspace_blob.archive_size;
        let snapshot = RemoteExecutionSnapshot {
            archive_path: workspace_blob.path,
            descriptor,
            tasks,
            environment_values,
            resources: job.resources,
            context_manifest: job.context_manifest.clone(),
            workspace_reuse,
            overlays,
        };
        transaction.commit()?;
        Ok(snapshot)
    }
}

fn workspace_reuse(job: &tak_core::v2::ResolvedJob) -> Result<WorkerWorkspaceReuse> {
    let Some(session) = job.session.as_ref() else {
        return Ok(WorkerWorkspaceReuse::Private);
    };
    if let SessionReuse::Paths { paths } = &session.reuse {
        return Ok(WorkerWorkspaceReuse::Paths {
            session_id: session.id.clone(),
            paths: paths.clone(),
        });
    }
    if !matches!(session.reuse, SessionReuse::SharedWorkspace { .. }) {
        return Ok(WorkerWorkspaceReuse::Private);
    }
    let (
        Some(Affinity::RequireSameNode {
            group: session_group,
        }),
        Some(Affinity::RequireSameNode { group: job_group }),
    ) = (&session.affinity, &job.affinity)
    else {
        bail!("remote shared workspace requires hard same-node affinity");
    };
    if session_group != job_group {
        bail!("remote shared workspace affinity does not match its job");
    }
    Ok(WorkerWorkspaceReuse::Shared {
        session_id: session.id.clone(),
        affinity_group: session_group.clone(),
    })
}

fn environment_values(
    transaction: &rusqlite::Transaction<'_>,
    run_id: &str,
) -> Result<Vec<EnvironmentValue>> {
    let mut statement = transaction
        .prepare("SELECT name,value FROM run_environment WHERE run_id=?1 ORDER BY name")?;
    statement
        .query_map([run_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .map(|row| {
            let (name, value) = row?;
            EnvironmentValue::new(name, value).map_err(anyhow::Error::from)
        })
        .collect()
}
