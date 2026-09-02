use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use rusqlite::{TransactionBehavior, params};
use sha2::{Digest, Sha256};
use tak_core::v2::{WorkspaceEntry, WorkspaceManifest};
use tak_proto::local_daemon::v2::OutputArtifact;
use tak_proto::worker_v2::WorkerOutputArtifact;

use crate::daemon::scheduler::{DispatchCommand, ResultAcceptance};

use super::{OutputArtifactChunk, RunOutputManifest, RunStore};

mod cas;
mod queries;
mod resolution;
mod validation;

pub(super) use resolution::{FinalPublication, dependency_overlays, publish_final};

pub(in crate::daemon) struct CapturedFile {
    pub(in crate::daemon) size: u64,
    pub(in crate::daemon) sha256: String,
    pub(in crate::daemon) executable: bool,
}

#[derive(Debug, Clone)]
pub(in crate::daemon) struct OutputOverlay {
    pub(in crate::daemon) entry: WorkspaceEntry,
    pub(in crate::daemon) blob_path: Option<PathBuf>,
}

impl RunStore {
    pub(in crate::daemon) fn capture_output_file(&self, source: &Path) -> Result<CapturedFile> {
        cas::capture(self, source)
    }

    pub(in crate::daemon) fn import_remote_output_blob(
        &self,
        entry: &WorkspaceEntry,
        bytes: &[u8],
    ) -> Result<()> {
        cas::publish_bytes(self, entry, bytes)
    }

    pub(in crate::daemon) fn prevalidate_remote_attempt_outputs(
        &self,
        command: &DispatchCommand,
        outputs: &[WorkerOutputArtifact],
    ) -> Result<ResultAcceptance> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(run) = validation::current_run(&transaction, command)? else {
            return Ok(ResultAcceptance::Stale);
        };
        for output in outputs {
            validation::declarations(
                &run,
                command,
                &output.producer_task_id,
                std::slice::from_ref(&output.entry),
            )?;
        }
        transaction.commit()?;
        Ok(ResultAcceptance::Applied)
    }

    pub(in crate::daemon) fn persist_attempt_task_outputs(
        &self,
        command: &DispatchCommand,
        producer_task_id: &str,
        outputs: &[WorkspaceEntry],
    ) -> Result<ResultAcceptance> {
        let manifest = WorkspaceManifest::new(outputs.to_vec())?;
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(run) = validation::current_run(&transaction, command)? else {
            return Ok(ResultAcceptance::Stale);
        };
        validation::declarations(&run, command, producer_task_id, &manifest.entries)?;
        for entry in &manifest.entries {
            cas::require_blob(self, entry)?;
        }
        let existing = queries::attempt_task_entries(
            &transaction,
            &command.run_id,
            &command.fencing_token,
            producer_task_id,
        )?;
        if !existing.is_empty() {
            if existing == manifest.entries {
                return Ok(ResultAcceptance::Duplicate);
            }
            bail!("attempt output replay differs from its persisted manifest");
        }
        for entry in manifest.entries {
            let artifact_id = artifact_id(command, producer_task_id, &entry)?;
            transaction.execute(
                "INSERT INTO run_attempt_outputs (run_id,fencing_token,producer_task_id,path,artifact_id,entry_json) VALUES (?1,?2,?3,?4,?5,?6)",
                params![command.run_id, command.fencing_token, producer_task_id,
                    entry.path, artifact_id, serde_json::to_string(&entry)?],
            )?;
        }
        transaction.commit()?;
        Ok(ResultAcceptance::Applied)
    }

    pub fn output_manifest(&self, run_id: &str) -> Result<Option<Vec<OutputArtifact>>> {
        Ok(self
            .output_manifest_status(run_id)?
            .map(|manifest| manifest.artifacts))
    }

    pub fn output_manifest_status(&self, run_id: &str) -> Result<Option<RunOutputManifest>> {
        queries::manifest_status(self, run_id)
    }

    pub fn output_chunk(
        &self,
        artifact_id: &str,
        offset: u64,
        max_bytes: u32,
    ) -> Result<Option<OutputArtifactChunk>> {
        queries::chunk(self, artifact_id, offset, max_bytes)
    }
}

fn artifact_id(
    command: &DispatchCommand,
    producer_task_id: &str,
    entry: &WorkspaceEntry,
) -> Result<String> {
    let identity = serde_json::to_vec(&(
        "tak-v2-output",
        &command.run_id,
        &command.fencing_token,
        producer_task_id,
        entry,
    ))?;
    Ok(format!("{:x}", Sha256::digest(identity)))
}
