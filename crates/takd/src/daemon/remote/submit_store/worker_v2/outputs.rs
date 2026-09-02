use anyhow::{Result, bail};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use tak_core::v2::{WorkspaceEntry, WorkspaceEntryType, WorkspaceManifest};
use tak_proto::worker_v2::{WorkerAttemptIdentity, WorkerOutputArtifact};

use super::{SubmitAttemptStore, current_state};

mod declaration;

impl SubmitAttemptStore {
    pub fn publish_worker_v2_output(
        &self,
        identity: &WorkerAttemptIdentity,
        producer_task_id: &str,
        entry: WorkspaceEntry,
        content: &[u8],
    ) -> Result<WorkerOutputArtifact> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_running(&current_state(&transaction, identity)?)?;
        validate_content(&entry, content)?;
        declaration::validate(&transaction, identity, producer_task_id, &entry)?;
        let artifact = WorkerOutputArtifact {
            artifact_id: artifact_id(identity, producer_task_id, &entry)?,
            producer_task_id: producer_task_id.to_owned(),
            entry,
        };
        let inserted = transaction.execute(
            "INSERT OR IGNORE INTO worker_v2_outputs \
             (fencing_token,artifact_id,producer_task_id,path,entry_json,content) \
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                identity.fencing_token,
                artifact.artifact_id,
                artifact.producer_task_id,
                artifact.entry.path,
                serde_json::to_string(&artifact.entry)?,
                content
            ],
        )?;
        if inserted == 0 && existing(&transaction, identity, &artifact, content)?.is_none() {
            bail!("conflicting worker output publication");
        }
        transaction.commit()?;
        Ok(artifact)
    }

    pub fn worker_v2_output_chunk(
        &self,
        identity: &WorkerAttemptIdentity,
        artifact_id: &str,
        offset: u64,
        max_bytes: usize,
    ) -> Result<Vec<u8>> {
        Ok(self
            .worker_v2_output_chunk_with_eof(identity, artifact_id, offset, max_bytes)?
            .0)
    }

    pub fn worker_v2_output_chunk_with_eof(
        &self,
        identity: &WorkerAttemptIdentity,
        artifact_id: &str,
        offset: u64,
        max_bytes: usize,
    ) -> Result<(Vec<u8>, bool)> {
        let connection = self.open_connection()?;
        let content = connection
            .query_row(
                "SELECT output.content FROM worker_v2_outputs output JOIN worker_v2_attempts attempt \
                 ON attempt.fencing_token=output.fencing_token WHERE output.fencing_token=?1 AND \
                 output.artifact_id=?2 AND attempt.run_id=?3 AND attempt.job_id=?4 AND \
                 attempt.authored_attempt=?5 AND attempt.dispatch_generation=?6 AND attempt.node_id=?7",
                params![identity.fencing_token, artifact_id, identity.run_id, identity.job_id,
                    identity.authored_attempt, identity.dispatch_generation, identity.node_id],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()?
            .ok_or_else(|| anyhow::anyhow!("worker output artifact is unknown"))?;
        let start = usize::try_from(offset)?;
        if start > content.len() {
            bail!("worker output offset exceeds the artifact size");
        }
        let end = start.saturating_add(max_bytes).min(content.len());
        Ok((content[start..end].to_vec(), end == content.len()))
    }

    pub(crate) fn discard_worker_v2_outputs(&self, identity: &WorkerAttemptIdentity) -> Result<()> {
        let connection = self.open_connection()?;
        connection.execute(
            "DELETE FROM worker_v2_outputs WHERE fencing_token=?1",
            [&identity.fencing_token],
        )?;
        Ok(())
    }
}

fn require_running(state: &str) -> Result<()> {
    if state == "running" {
        return Ok(());
    }
    bail!("worker attempt is not active")
}

fn validate_content(entry: &WorkspaceEntry, content: &[u8]) -> Result<()> {
    let _ = WorkspaceManifest::new([entry.clone()])?;
    match entry.entry_type {
        WorkspaceEntryType::File
            if entry.size == content.len() as u64
                && entry.content_sha256 == format!("{:x}", Sha256::digest(content)) =>
        {
            Ok(())
        }
        WorkspaceEntryType::Directory | WorkspaceEntryType::Symlink if content.is_empty() => Ok(()),
        _ => bail!("worker output content does not match its manifest entry"),
    }
}

fn artifact_id(
    identity: &WorkerAttemptIdentity,
    producer_task_id: &str,
    entry: &WorkspaceEntry,
) -> Result<String> {
    let encoded = serde_json::to_vec(&(&identity.fencing_token, producer_task_id, entry))?;
    Ok(format!("{:x}", Sha256::digest(encoded)))
}

fn existing(
    connection: &rusqlite::Connection,
    identity: &WorkerAttemptIdentity,
    artifact: &WorkerOutputArtifact,
    content: &[u8],
) -> Result<Option<()>> {
    let stored = connection
        .query_row(
            "SELECT artifact_id,entry_json,content FROM worker_v2_outputs WHERE \
             fencing_token=?1 AND producer_task_id=?2 AND path=?3",
            params![
                identity.fencing_token,
                artifact.producer_task_id,
                artifact.entry.path
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            },
        )
        .optional()?;
    Ok(stored
        .filter(|(id, entry, bytes)| {
            id == &artifact.artifact_id
                && entry == &serde_json::to_string(&artifact.entry).unwrap_or_default()
                && bytes == content
        })
        .map(|_| ()))
}
