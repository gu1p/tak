use anyhow::Result;
use rusqlite::{OptionalExtension, Transaction, params};
use tak_core::v2::WorkspaceDescriptor;

use super::SharedUpload;
use crate::daemon::run_store::events::{now_ms, sqlite_i64};

pub(in crate::daemon::run_store) fn load_or_claim(
    transaction: &Transaction<'_>,
    run_id: &str,
    descriptor: &WorkspaceDescriptor,
    offset: u64,
) -> Result<SharedUpload> {
    load(transaction, &descriptor.manifest.fingerprint)?
        .map_or_else(|| claim(transaction, run_id, descriptor, offset), Ok)
}

pub(in crate::daemon::run_store) fn record_progress(
    transaction: &Transaction<'_>,
    fingerprint: &str,
    next_offset: u64,
    complete: bool,
) -> Result<Vec<String>> {
    let now = sqlite_i64(now_ms()?, "timestamp")?;
    if !complete {
        transaction.execute(
            "UPDATE workspace_uploads SET upload_offset = ?2, updated_at_ms = ?3 WHERE fingerprint = ?1",
            params![fingerprint, sqlite_i64(next_offset, "upload offset")?, now],
        )?;
        sync_waiters(transaction, fingerprint, next_offset)?;
        return Ok(Vec::new());
    }
    let run_ids = waiting_run_ids(transaction, fingerprint)?;
    transaction.execute(
        "UPDATE runs SET upload_offset = archive_size, state = 'awaiting_commit', updated_at_ms = ?2 WHERE workspace_fingerprint = ?1 AND state = 'awaiting_workspace'",
        params![fingerprint, now],
    )?;
    transaction.execute(
        "DELETE FROM workspace_uploads WHERE fingerprint = ?1",
        [fingerprint],
    )?;
    Ok(run_ids)
}

pub(in crate::daemon::run_store) fn reset(
    transaction: &Transaction<'_>,
    fingerprint: &str,
) -> Result<()> {
    let now = sqlite_i64(now_ms()?, "timestamp")?;
    transaction.execute(
        "UPDATE workspace_uploads SET upload_offset = 0, updated_at_ms = ?2 WHERE fingerprint = ?1",
        params![fingerprint, now],
    )?;
    transaction.execute(
        "UPDATE runs SET upload_offset = 0, state = 'awaiting_workspace', updated_at_ms = ?2 WHERE workspace_fingerprint = ?1 AND state IN ('awaiting_workspace', 'awaiting_commit')",
        params![fingerprint, now],
    )?;
    Ok(())
}

pub(in crate::daemon::run_store) fn sync_waiters(
    transaction: &Transaction<'_>,
    fingerprint: &str,
    offset: u64,
) -> Result<()> {
    transaction.execute(
        "UPDATE runs SET upload_offset = ?2 WHERE workspace_fingerprint = ?1 AND state = 'awaiting_workspace'",
        params![fingerprint, sqlite_i64(offset, "upload offset")?],
    )?;
    Ok(())
}

pub(in crate::daemon::run_store) fn release_if_unused(
    transaction: &Transaction<'_>,
    run_id: &str,
) -> Result<Option<String>> {
    let fingerprint = transaction
        .query_row(
            "SELECT workspace_fingerprint FROM runs WHERE run_id = ?1",
            [run_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(fingerprint) = fingerprint else {
        return Ok(None);
    };
    let Some(upload) = load(transaction, &fingerprint)? else {
        return Ok(None);
    };
    let waiters = transaction.query_row(
        "SELECT COUNT(*) FROM runs WHERE workspace_fingerprint = ?1 AND state = 'awaiting_workspace'",
        [&fingerprint],
        |row| row.get::<_, i64>(0),
    )?;
    if waiters != 0 {
        return Ok(None);
    }
    transaction.execute(
        "DELETE FROM workspace_uploads WHERE fingerprint = ?1",
        [&fingerprint],
    )?;
    Ok(Some(upload.owner_run_id))
}

pub(in crate::daemon::run_store) fn discard(
    transaction: &Transaction<'_>,
    fingerprint: &str,
) -> Result<Option<String>> {
    let owner = load(transaction, fingerprint)?.map(|upload| upload.owner_run_id);
    transaction.execute(
        "DELETE FROM workspace_uploads WHERE fingerprint = ?1",
        [fingerprint],
    )?;
    Ok(owner)
}

fn load(transaction: &Transaction<'_>, fingerprint: &str) -> Result<Option<SharedUpload>> {
    transaction
        .query_row(
            "SELECT owner_run_id, archive_sha256, archive_size, upload_offset FROM workspace_uploads WHERE fingerprint = ?1",
            [fingerprint],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()?
        .map(decode_upload)
        .transpose()
}

fn claim(
    transaction: &Transaction<'_>,
    run_id: &str,
    descriptor: &WorkspaceDescriptor,
    offset: u64,
) -> Result<SharedUpload> {
    transaction.execute(
        "INSERT INTO workspace_uploads (fingerprint, owner_run_id, archive_sha256, archive_size, upload_offset, updated_at_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            descriptor.manifest.fingerprint,
            run_id,
            descriptor.archive_sha256,
            sqlite_i64(descriptor.archive_size, "workspace archive size")?,
            sqlite_i64(offset, "upload offset")?,
            sqlite_i64(now_ms()?, "timestamp")?,
        ],
    )?;
    Ok(SharedUpload {
        owner_run_id: run_id.to_owned(),
        archive_sha256: descriptor.archive_sha256.clone(),
        archive_size: descriptor.archive_size,
        next_offset: offset,
    })
}

fn decode_upload(row: (String, String, i64, i64)) -> Result<SharedUpload> {
    Ok(SharedUpload {
        owner_run_id: row.0,
        archive_sha256: row.1,
        archive_size: u64::try_from(row.2)
            .map_err(|_| anyhow::anyhow!("stored archive size is invalid"))?,
        next_offset: u64::try_from(row.3)
            .map_err(|_| anyhow::anyhow!("stored upload offset is invalid"))?,
    })
}

fn waiting_run_ids(transaction: &Transaction<'_>, fingerprint: &str) -> Result<Vec<String>> {
    let mut statement = transaction.prepare(
        "SELECT run_id FROM runs WHERE workspace_fingerprint = ?1 AND state = 'awaiting_workspace' ORDER BY created_at_ms, run_id",
    )?;
    Ok(statement
        .query_map([fingerprint], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<_>>()?)
}
