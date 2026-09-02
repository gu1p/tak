use std::collections::BTreeSet;
use std::fs;

use anyhow::{Result, ensure};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use tak_core::v2::{WorkspaceEntry, WorkspaceEntryType};
use tak_proto::local_daemon::v2::{RunEvent, RunEventKind};

use super::super::RunStore;
use super::shared_workspaces;

pub(super) fn expire_one(store: &RunStore, run_id: &str) -> Result<()> {
    let mut connection = store.open_connection()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let digests = expire_in(store, &transaction, run_id)?;
    transaction.commit()?;
    drop(connection);
    remove_unreferenced_outputs(store, digests)
}

pub(super) fn expire_due(store: &RunStore, now_ms: u64, ttl_ms: u64) -> Result<u64> {
    let cutoff = now_ms.saturating_sub(ttl_ms);
    let mut connection = store.open_connection()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let run_ids = {
        let mut statement = transaction.prepare(
            "SELECT run_id FROM runs WHERE state IN ('succeeded','failed','cancelled') \
             AND updated_at_ms<=?1 AND (logs_expired=0 OR outputs_expired=0) ORDER BY run_id",
        )?;
        statement
            .query_map([sqlite(cutoff)?], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    let mut digests = BTreeSet::new();
    for run_id in &run_ids {
        digests.extend(expire_in(store, &transaction, run_id)?);
    }
    transaction.commit()?;
    drop(connection);
    remove_unreferenced_outputs(store, digests)?;
    Ok(run_ids.len() as u64)
}

pub(super) fn purge_due(store: &RunStore, now_ms: u64, ttl_ms: u64) -> Result<u64> {
    let cutoff = now_ms.saturating_sub(ttl_ms);
    let mut connection = store.open_connection()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let run_ids = terminal_runs_before(&transaction, cutoff)?;
    for run_id in &run_ids {
        shared_workspaces::remove(store, &transaction, run_id)?;
        transaction.execute("DELETE FROM runs WHERE run_id=?1", [run_id])?;
    }
    transaction.commit()?;
    Ok(run_ids.len() as u64)
}

fn expire_in(
    store: &RunStore,
    transaction: &Transaction<'_>,
    run_id: &str,
) -> Result<BTreeSet<String>> {
    let state = transaction
        .query_row("SELECT state FROM runs WHERE run_id=?1", [run_id], |row| {
            row.get::<_, String>(0)
        })
        .optional()?;
    ensure!(
        state.is_some_and(|state| matches!(state.as_str(), "succeeded" | "failed" | "cancelled")),
        "run is missing or not terminal"
    );
    shared_workspaces::remove(store, transaction, run_id)?;
    let log_sequences = {
        let mut statement = transaction
            .prepare("SELECT seq,payload_json FROM run_events WHERE run_id=?1 ORDER BY seq")?;
        statement
            .query_map([run_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .filter_map(|(seq, payload)| {
                serde_json::from_str::<RunEvent>(&payload)
                    .ok()
                    .filter(|event| {
                        matches!(event.kind, RunEventKind::Stdout | RunEventKind::Stderr)
                    })
                    .map(|_| seq)
            })
            .collect::<Vec<_>>()
    };
    for seq in log_sequences {
        transaction.execute(
            "DELETE FROM run_events WHERE run_id=?1 AND seq=?2",
            params![run_id, seq],
        )?;
    }
    let digests = output_digests(transaction, Some(run_id))?;
    transaction.execute("DELETE FROM run_attempt_outputs WHERE run_id=?1", [run_id])?;
    transaction.execute(
        "UPDATE runs SET logs_expired=1,outputs_expired=1 WHERE run_id=?1",
        [run_id],
    )?;
    Ok(digests)
}

fn terminal_runs_before(transaction: &Transaction<'_>, cutoff: u64) -> Result<Vec<String>> {
    let mut statement = transaction.prepare(
        "SELECT run_id FROM runs WHERE state IN ('succeeded','failed','cancelled') \
         AND updated_at_ms<?1 ORDER BY run_id",
    )?;
    Ok(statement
        .query_map([sqlite(cutoff)?], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<_>>()?)
}

fn output_digests(transaction: &Transaction<'_>, run_id: Option<&str>) -> Result<BTreeSet<String>> {
    let sql = match run_id {
        Some(_) => "SELECT entry_json FROM run_attempt_outputs WHERE run_id=?1",
        None => "SELECT entry_json FROM run_attempt_outputs WHERE ?1 IS NULL",
    };
    let mut statement = transaction.prepare(sql)?;
    let entries = statement
        .query_map([run_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    entries
        .into_iter()
        .map(|entry| serde_json::from_str::<WorkspaceEntry>(&entry).map_err(Into::into))
        .filter_map(|entry: Result<WorkspaceEntry>| match entry {
            Ok(entry) if entry.entry_type == WorkspaceEntryType::File => {
                Some(Ok(entry.content_sha256))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

fn remove_unreferenced_outputs(store: &RunStore, candidates: BTreeSet<String>) -> Result<()> {
    if candidates.is_empty() {
        return Ok(());
    }
    let mut connection = store.open_connection()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let retained = output_digests(&transaction, None)?;
    for digest in candidates.difference(&retained) {
        let path = store.blob_root.join("outputs").join(digest);
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    transaction.commit()?;
    Ok(())
}

fn sqlite(value: u64) -> Result<i64> {
    value
        .try_into()
        .map_err(|_| anyhow::anyhow!("maintenance timestamp exceeds SQLite range"))
}
