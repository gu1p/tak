use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Result, bail};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use tak_core::v2::{ResolvedRun, ResolvedTaskUnit, SessionReuse};

use crate::daemon::scheduler::DispatchCommand;

use super::RunStore;
use super::blob::verified_blob;
use super::output_artifacts::{OutputOverlay, dependency_overlays};

mod remote;

pub(in crate::daemon) struct RemoteExecutionSnapshot {
    pub(in crate::daemon) archive_path: PathBuf,
    pub(in crate::daemon) descriptor: tak_core::v2::WorkspaceDescriptor,
    pub(in crate::daemon) tasks: Vec<ResolvedTaskUnit>,
    pub(in crate::daemon) environment_values: Vec<tak_core::v2::EnvironmentValue>,
    pub(in crate::daemon) resources: tak_core::v2::ResourceRequest,
    pub(in crate::daemon) context_manifest: tak_core::v2::JobContextManifest,
    pub(in crate::daemon) workspace_reuse: tak_proto::worker_v2::WorkerWorkspaceReuse,
    pub(in crate::daemon) overlays: Vec<OutputOverlay>,
}

pub(in crate::daemon) struct LocalExecutionSnapshot {
    pub(in crate::daemon) archive_path: PathBuf,
    pub(in crate::daemon) attempt_root: PathBuf,
    pub(in crate::daemon) tasks: Vec<ResolvedTaskUnit>,
    pub(in crate::daemon) environment: BTreeMap<String, String>,
    pub(in crate::daemon) workspace: LocalWorkspace,
    pub(in crate::daemon) overlays: Vec<OutputOverlay>,
    pub(in crate::daemon) context_manifest: tak_core::v2::JobContextManifest,
}

pub(in crate::daemon) enum LocalWorkspace {
    Private,
    Paths(super::super::path_cache::PathCache),
    Shared(PathBuf),
}

impl RunStore {
    pub(in crate::daemon) fn local_attempt_is_current(
        &self,
        command: &DispatchCommand,
    ) -> Result<bool> {
        let connection = self.open_connection()?;
        let count = connection.query_row(
            "SELECT COUNT(*) FROM run_attempts attempt JOIN run_jobs job USING (run_id,job_id) \
             WHERE attempt.run_id=?1 AND attempt.job_id=?2 AND attempt.authored_attempt=?3 \
             AND attempt.dispatch_generation=?4 AND attempt.fencing_token=?5 \
             AND attempt.node_id=?6 AND attempt.state IN ('transferring','running','output_committing') \
             AND attempt.released_at_ms IS NULL \
             AND job.current_fencing_token=attempt.fencing_token",
            rusqlite::params![
                command.run_id,
                command.job_id,
                command.authored_attempt,
                command.dispatch_generation,
                command.fencing_token,
                command.node_id
            ],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(count == 1)
    }

    pub(in crate::daemon) fn local_execution_snapshot(
        &self,
        command: &DispatchCommand,
    ) -> Result<LocalExecutionSnapshot> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let stored = transaction
            .query_row(
                "SELECT run.resolved_json, run.workspace_fingerprint \
                 FROM run_attempts attempt JOIN run_jobs job USING (run_id,job_id) \
                 JOIN runs run USING (run_id) WHERE attempt.run_id=?1 AND attempt.job_id=?2 \
                 AND attempt.authored_attempt=?3 AND attempt.dispatch_generation=?4 \
                 AND attempt.fencing_token=?5 AND attempt.node_id=?6 \
                 AND attempt.state IN ('transferring','running') \
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
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        let Some((resolved, fingerprint)) = stored else {
            bail!("local attempt fence is no longer current");
        };
        let run: ResolvedRun = serde_json::from_str(&resolved)?;
        let job = run
            .jobs
            .iter()
            .find(|job| job.job_id == command.job_id)
            .ok_or_else(|| anyhow::anyhow!("resolved local job is missing"))?;
        let mut tasks = Vec::with_capacity(job.task_ids.len());
        for task_id in &job.task_ids {
            let task = run
                .tasks
                .iter()
                .find(|task| task.task_id == *task_id && task.job_id == job.job_id)
                .ok_or_else(|| anyhow::anyhow!("resolved local task `{task_id}` is missing"))?;
            tasks.push(task.clone());
        }
        let mut statement = transaction
            .prepare("SELECT name,value FROM run_environment WHERE run_id=?1 ORDER BY name")?;
        let environment = statement
            .query_map([&command.run_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<BTreeMap<_, _>>>()?;
        drop(statement);
        let archive_path = verified_blob(self, &transaction, &fingerprint)?
            .ok_or_else(|| anyhow::anyhow!("verified workspace blob is missing"))?;
        let consumer = job
            .task_ids
            .first()
            .ok_or_else(|| anyhow::anyhow!("resolved local job has no tasks"))?;
        let overlays = dependency_overlays(self, &transaction, &command.run_id, &run, consumer)?;
        transaction.commit()?;
        let workspace = match job.session.as_ref() {
            Some(session) if matches!(session.reuse, SessionReuse::SharedWorkspace { .. }) => {
                let identity =
                    serde_json::to_vec(&(&command.run_id, &session.id, &command.node_id))
                        .expect("shared workspace identity serializes");
                LocalWorkspace::Shared(
                    self.blob_root
                        .join("shared-workspaces")
                        .join(format!("{:x}", Sha256::digest(identity))),
                )
            }
            Some(session) if matches!(session.reuse, SessionReuse::Paths { .. }) => {
                let SessionReuse::Paths { paths } = &session.reuse else {
                    unreachable!();
                };
                let identity =
                    serde_json::to_vec(&(&command.run_id, &session.id, &command.node_id))?;
                let root = self
                    .blob_root
                    .join("path-caches")
                    .join(format!("{:x}", Sha256::digest(identity)));
                LocalWorkspace::Paths(super::super::path_cache::PathCache::new(
                    root,
                    paths.clone(),
                )?)
            }
            _ => LocalWorkspace::Private,
        };
        Ok(LocalExecutionSnapshot {
            archive_path,
            attempt_root: self.attempt_root(command),
            tasks,
            environment,
            workspace,
            overlays,
            context_manifest: job.context_manifest.clone(),
        })
    }

    pub(in crate::daemon) fn attempt_root(&self, command: &DispatchCommand) -> PathBuf {
        let identity = serde_json::to_vec(command).expect("dispatch identity serializes");
        self.blob_root
            .join("attempts")
            .join(format!("{:x}", Sha256::digest(identity)))
    }
}
