use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use anyhow::{Result, bail, ensure};
use rusqlite::{OptionalExtension, Transaction, params};
use tak_core::v2::{WorkspaceEntry, WorkspaceEntryType};
use tak_proto::local_daemon::v2::{MAX_WORKSPACE_CHUNK_BYTES, OutputArtifact};

use super::cas;
use crate::daemon::run_store::{OutputArtifactChunk, RunOutputManifest, RunStore};

pub(super) fn attempt_task_entries(
    transaction: &Transaction<'_>,
    run_id: &str,
    fence: &str,
    producer: &str,
) -> Result<Vec<WorkspaceEntry>> {
    let mut statement = transaction.prepare(
        "SELECT entry_json FROM run_attempt_outputs WHERE run_id=?1 AND fencing_token=?2 \
         AND producer_task_id=?3 ORDER BY path",
    )?;
    let encoded = statement
        .query_map(params![run_id, fence, producer], |row| {
            row.get::<_, String>(0)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    encoded
        .into_iter()
        .map(|entry| serde_json::from_str(&entry).map_err(Into::into))
        .collect()
}

pub(super) fn manifest_status(store: &RunStore, run_id: &str) -> Result<Option<RunOutputManifest>> {
    let connection = store.open_connection()?;
    let state = connection
        .query_row(
            "SELECT state,outputs_expired,output_error FROM runs WHERE run_id=?1",
            [run_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, bool>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()?;
    let Some((state, expired, output_error)) = state else {
        return Ok(None);
    };
    ensure!(
        matches!(state.as_str(), "succeeded" | "failed" | "cancelled"),
        "run outputs are not available before terminal state"
    );
    if let Some(error) = output_error {
        bail!("run output manifest conflict: {error}");
    }
    if expired {
        return Ok(Some(RunOutputManifest {
            expired,
            artifacts: Vec::new(),
        }));
    }
    let mut statement = connection.prepare(
        "SELECT output.artifact_id, attempt.entry_json FROM run_final_outputs output \
         JOIN run_attempt_outputs attempt ON attempt.artifact_id=output.artifact_id \
         WHERE output.run_id=?1 ORDER BY output.path",
    )?;
    let rows = statement
        .query_map([run_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let artifacts = rows
        .into_iter()
        .map(|(artifact_id, encoded)| {
            let entry: WorkspaceEntry = serde_json::from_str(&encoded)?;
            Ok(protocol_artifact(entry, artifact_id))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Some(RunOutputManifest { expired, artifacts }))
}

pub(super) fn chunk(
    store: &RunStore,
    artifact_id: &str,
    offset: u64,
    max_bytes: u32,
) -> Result<Option<OutputArtifactChunk>> {
    ensure!(
        max_bytes > 0 && max_bytes as usize <= MAX_WORKSPACE_CHUNK_BYTES,
        "output chunk size is invalid"
    );
    let connection = store.open_connection()?;
    let encoded = connection
        .query_row(
            "SELECT attempt.entry_json FROM run_final_outputs output JOIN run_attempt_outputs \
             attempt ON attempt.artifact_id=output.artifact_id JOIN runs run ON \
             run.run_id=output.run_id WHERE output.artifact_id=?1 AND run.outputs_expired=0",
            [artifact_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(encoded) = encoded else {
        return Ok(None);
    };
    let entry: WorkspaceEntry = serde_json::from_str(&encoded)?;
    if entry.entry_type != WorkspaceEntryType::File {
        bail!("output artifact has no downloadable body");
    }
    cas::require_blob(store, &entry)?;
    ensure!(offset <= entry.size, "output chunk offset is invalid");
    let mut file = File::open(cas::blob_path(store, &entry.content_sha256))?;
    file.seek(SeekFrom::Start(offset))?;
    let remaining = entry.size - offset;
    let length = remaining.min(u64::from(max_bytes)) as usize;
    let mut bytes = vec![0; length];
    file.read_exact(&mut bytes)?;
    Ok(Some(OutputArtifactChunk {
        bytes,
        complete: offset + length as u64 == entry.size,
    }))
}

fn protocol_artifact(entry: WorkspaceEntry, artifact_id: String) -> OutputArtifact {
    let entry_type = match entry.entry_type {
        WorkspaceEntryType::File => "file",
        WorkspaceEntryType::Directory => "directory",
        WorkspaceEntryType::Symlink => "symlink",
    };
    OutputArtifact {
        path: entry.path,
        entry_type: entry_type.into(),
        executable: entry.executable,
        symlink_target: entry.symlink_target,
        size: entry.size,
        sha256: entry.content_sha256,
        artifact_id,
    }
}
