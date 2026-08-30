use anyhow::Result;
use rusqlite::{Transaction, params};
use tak_core::v2::{
    OutputMergeError, ProducedOutput, ResolvedRun, WorkspaceEntry, WorkspaceEntryType,
    resolve_dependency_outputs, resolve_final_outputs,
};

use super::{OutputOverlay, RunStore, cas};

pub(in crate::daemon::run_store) enum FinalPublication {
    Published,
    Conflict(OutputMergeError),
}

pub(in crate::daemon::run_store) fn dependency_overlays(
    store: &RunStore,
    transaction: &Transaction<'_>,
    run_id: &str,
    run: &ResolvedRun,
    consumer_task_id: &str,
) -> Result<Vec<OutputOverlay>> {
    let merged =
        resolve_dependency_outputs(run, consumer_task_id, accepted(transaction, run_id, None)?)?;
    merged
        .outputs
        .into_iter()
        .map(|output| {
            let blob_path = (output.entry.entry_type == WorkspaceEntryType::File)
                .then(|| cas::blob_path(store, &output.entry.content_sha256));
            if blob_path.is_some() {
                cas::require_blob(store, &output.entry)?;
            }
            Ok(OutputOverlay {
                entry: output.entry,
                blob_path,
            })
        })
        .collect()
}

pub(in crate::daemon::run_store) fn publish_final(
    transaction: &Transaction<'_>,
    run_id: &str,
    current_fence: Option<&str>,
) -> Result<FinalPublication> {
    let encoded = transaction.query_row(
        "SELECT resolved_json FROM runs WHERE run_id=?1",
        [run_id],
        |row| row.get::<_, String>(0),
    )?;
    let run: ResolvedRun = serde_json::from_str(&encoded)?;
    let merged = match resolve_final_outputs(&run, accepted(transaction, run_id, current_fence)?) {
        Ok(merged) => merged,
        Err(error) => return Ok(FinalPublication::Conflict(error)),
    };
    transaction.execute("DELETE FROM run_final_outputs WHERE run_id=?1", [run_id])?;
    for output in merged.outputs {
        transaction.execute(
            "INSERT INTO run_final_outputs (run_id,path,artifact_id,producer_task_id) VALUES (?1,?2,?3,?4)",
            params![run_id, output.entry.path, output.payload, output.producers[0]],
        )?;
    }
    Ok(FinalPublication::Published)
}

fn accepted(
    transaction: &Transaction<'_>,
    run_id: &str,
    current_fence: Option<&str>,
) -> Result<Vec<ProducedOutput<String>>> {
    let mut statement = transaction.prepare(
        "SELECT output.producer_task_id,output.entry_json,output.artifact_id \
         FROM run_attempt_outputs output JOIN run_attempts attempt \
         ON attempt.run_id=output.run_id AND attempt.fencing_token=output.fencing_token \
         WHERE output.run_id=?1 AND (attempt.outcome='succeeded' OR \
         (?2 IS NOT NULL AND attempt.fencing_token=?2 AND attempt.released_at_ms IS NULL \
         AND attempt.state IN ('transferring','running','output_committing'))) \
         ORDER BY output.producer_task_id,output.path",
    )?;
    let rows = statement
        .query_map(params![run_id, current_fence], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(|(producer_task_id, encoded, artifact_id)| {
            Ok(ProducedOutput {
                producer_task_id,
                entry: serde_json::from_str::<WorkspaceEntry>(&encoded)?,
                payload: artifact_id,
            })
        })
        .collect()
}
