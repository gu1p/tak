use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use rusqlite::{OptionalExtension, Transaction, params};
use sha2::{Digest, Sha256};

use super::RunStore;
use super::events::{now_ms, sqlite_i64};

mod archive;

pub(super) fn verify_archive_manifest(
    path: &Path,
    expected: &tak_core::v2::WorkspaceManifest,
) -> Result<()> {
    archive::verify_archive_manifest(path, expected)
}

pub(super) fn verify_file(path: &Path, expected_digest: &str, expected_size: u64) -> Result<()> {
    if !file_matches(path, expected_digest, expected_size)? {
        bail!("workspace archive digest mismatch");
    }
    Ok(())
}

fn file_matches(path: &Path, expected_digest: &str, expected_size: u64) -> Result<bool> {
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut size = 0_u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        size += read as u64;
        hasher.update(&buffer[..read]);
    }
    if size != expected_size || format!("{:x}", hasher.finalize()) != expected_digest {
        return Ok(false);
    }
    Ok(true)
}

pub(super) fn verified_blob(
    store: &RunStore,
    transaction: &Transaction<'_>,
    fingerprint: &str,
) -> Result<Option<PathBuf>> {
    let stored = transaction
        .query_row(
            "SELECT archive_sha256, archive_size, path FROM workspace_blobs WHERE fingerprint = ?1",
            [fingerprint],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?;
    let Some((digest, size, path)) = stored else {
        return Ok(None);
    };
    let size = u64::try_from(size).map_err(|_| anyhow::anyhow!("stored blob size is invalid"))?;
    let path = PathBuf::from(path);
    if path != store.blob_path(fingerprint) || !file_matches(&path, &digest, size)? {
        return Ok(None);
    }
    transaction.execute(
        "UPDATE workspace_blobs SET last_accessed_ms = ?2 WHERE fingerprint = ?1",
        params![
            fingerprint,
            sqlite_i64(now_ms()?, "workspace access timestamp")?
        ],
    )?;
    Ok(Some(path))
}

pub(super) fn publish_blob(
    store: &RunStore,
    transaction: &Transaction<'_>,
    upload_path: &Path,
    fingerprint: &str,
    digest: &str,
    size: u64,
) -> Result<()> {
    let blob_path = store.blob_path(fingerprint);
    ensure_private_parent(&blob_path)?;
    if verified_blob(store, transaction, fingerprint)?.is_none() {
        let temporary = blob_path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        fs::copy(upload_path, &temporary).context("copy workspace blob")?;
        fs::File::open(&temporary)?.sync_all()?;
        verify_file(&temporary, digest, size)?;
        fs::rename(&temporary, &blob_path).context("publish workspace blob")?;
        fs::File::open(blob_path.parent().expect("blob has parent"))?.sync_all()?;
        transaction.execute(
            "INSERT INTO workspace_blobs (fingerprint, archive_sha256, archive_size, path, last_accessed_ms) VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(fingerprint) DO UPDATE SET archive_sha256 = excluded.archive_sha256, archive_size = excluded.archive_size, path = excluded.path, last_accessed_ms = excluded.last_accessed_ms",
            params![
                fingerprint,
                digest,
                sqlite_i64(size, "workspace archive size")?,
                blob_path.display().to_string(),
                sqlite_i64(now_ms()?, "timestamp")?
            ],
        )?;
    }
    Ok(())
}

pub(super) fn ensure_private_parent(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("workspace blob path has no parent"))?;
    fs::create_dir_all(parent)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
