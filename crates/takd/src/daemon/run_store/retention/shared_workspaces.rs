use std::fs;

use anyhow::{Result, ensure};
use rusqlite::{OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use tak_core::v2::{ResolvedRun, SessionReuse};

use super::super::RunStore;

pub(super) fn remove(store: &RunStore, transaction: &Transaction<'_>, run_id: &str) -> Result<()> {
    if has_active_attempt(transaction, run_id)? {
        return Ok(());
    }
    let Some(encoded) = transaction
        .query_row(
            "SELECT resolved_json FROM runs WHERE run_id=?1",
            [run_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
    else {
        return Ok(());
    };
    let run: ResolvedRun = serde_json::from_str(&encoded)?;
    let mut statement = transaction.prepare(
        "SELECT DISTINCT job_id,node_id FROM run_attempts WHERE run_id=?1 \
         AND node_id='local' AND transport IS NULL ORDER BY job_id,node_id",
    )?;
    let placements = statement
        .query_map([run_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (job_id, node_id) in placements {
        let Some(session) = run
            .jobs
            .iter()
            .find(|job| job.job_id == job_id)
            .and_then(|job| job.session.as_ref())
            .filter(|session| matches!(session.reuse, SessionReuse::SharedWorkspace { .. }))
        else {
            continue;
        };
        session.validate()?;
        let identity = serde_json::to_vec(&(run_id, &session.id, &node_id))?;
        remove_root(
            &store.blob_root.join("shared-workspaces"),
            &format!("{:x}", Sha256::digest(identity)),
        )?;
    }
    Ok(())
}

fn has_active_attempt(transaction: &Transaction<'_>, run_id: &str) -> Result<bool> {
    transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM run_attempts WHERE run_id=?1 \
             AND released_at_ms IS NULL)",
            [run_id],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn remove_root(parent: &std::path::Path, key: &str) -> Result<()> {
    ensure!(
        key.len() == 64 && key.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "shared workspace key is invalid"
    );
    let root = parent.join(key);
    ensure!(
        root.parent() == Some(parent),
        "shared workspace escapes blob root"
    );
    let metadata = match fs::symlink_metadata(&root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    ensure!(
        metadata.file_type().is_dir() && !metadata.file_type().is_symlink(),
        "shared workspace is not a directory"
    );
    let tombstone = parent.join(format!(".gc-shared-{}.tmp", uuid::Uuid::new_v4()));
    fs::rename(root, &tombstone)?;
    fs::remove_dir_all(tombstone)?;
    Ok(())
}
