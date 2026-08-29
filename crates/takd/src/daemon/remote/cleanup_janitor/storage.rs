use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::path::Path;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};

use super::{CLEANUP_TOMBSTONE_PREFIX, WORKSPACE_UPLOADS_DIR_NAME};

pub(super) fn cleanup_stale_remote_entries_with<F>(
    root: &Path,
    active_jobs: &BTreeSet<String>,
    ttl: Duration,
    mut remove_stale: F,
) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    let read_dir = match std::fs::read_dir(root) {
        Ok(read_dir) => read_dir,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(err)
                .with_context(|| format!("failed to read cleanup root {}", root.display()));
        }
    };

    for entry in read_dir {
        let entry = entry
            .with_context(|| format!("failed to read cleanup entry under {}", root.display()))?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        // The workspace-upload blob cache is reaped per-blob because it is
        // shared across a job's tasks and refreshed on every resolve.
        if name == WORKSPACE_UPLOADS_DIR_NAME || name.starts_with(CLEANUP_TOMBSTONE_PREFIX) {
            continue;
        }
        if active_jobs.contains(name) || !is_stale(&path, ttl)? {
            continue;
        }
        if let Err(err) = remove_stale(&path) {
            if is_permission_denied(&err) {
                tracing::warn!(
                    "remote cleanup janitor skipped stale entry {}: {err:#}",
                    path.display()
                );
                continue;
            }
            return Err(err);
        }
    }

    Ok(())
}

pub(super) fn is_stale(path: &Path, ttl: Duration) -> Result<bool> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(false),
        Err(err) => {
            return Err(err)
                .with_context(|| format!("failed to stat cleanup path {}", path.display()));
        }
    };
    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or_else(|_| Duration::from_secs(0));
    Ok(age >= ttl)
}

pub(super) fn remove_stale_remote_entry(path: &Path) -> Result<()> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(err)
                .with_context(|| format!("failed to stat stale cleanup path {}", path.display()));
        }
    };
    let file_type = metadata.file_type();
    if file_type.is_dir() && !file_type.is_symlink() {
        std::fs::remove_dir_all(path)
            .with_context(|| format!("failed to remove stale directory {}", path.display()))?;
    } else {
        std::fs::remove_file(path)
            .with_context(|| format!("failed to remove stale file {}", path.display()))?;
    }
    Ok(())
}

/// Reaps individual stale workspace-upload blobs under `.workspace-uploads`.
///
/// The directory is excluded from the generic per-job sweep because one blob
/// can be shared by every task of a job. Blobs are removed individually after
/// their mtime has remained stale for `ttl`.
///
/// ```no_run
/// # // Reason: This helper mutates daemon-owned workspace-upload storage and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(super) fn cleanup_stale_workspace_uploads(upload_dir: &Path, ttl: Duration) -> Result<()> {
    cleanup_stale_workspace_uploads_with(upload_dir, ttl, remove_stale_workspace_upload_file)
}

pub(super) fn cleanup_stale_workspace_uploads_with<F>(
    upload_dir: &Path,
    ttl: Duration,
    mut remove_stale: F,
) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    let read_dir = match std::fs::read_dir(upload_dir) {
        Ok(read_dir) => read_dir,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(err).with_context(|| {
                format!(
                    "failed to read workspace upload dir {}",
                    upload_dir.display()
                )
            });
        }
    };

    for entry in read_dir {
        let entry = entry.with_context(|| {
            format!(
                "failed to read workspace upload entry under {}",
                upload_dir.display()
            )
        })?;
        let path = entry.path();
        if !path.is_file() || !is_stale(&path, ttl)? {
            continue;
        }
        if let Err(err) = remove_stale(&path) {
            if is_permission_denied(&err) {
                tracing::warn!(
                    "remote cleanup janitor skipped stale workspace upload {}: {err:#}",
                    path.display()
                );
                continue;
            }
            return Err(err);
        }
    }

    Ok(())
}

pub(super) fn remove_stale_workspace_upload_file(path: &Path) -> Result<()> {
    std::fs::remove_file(path)
        .with_context(|| format!("failed to remove stale workspace upload {}", path.display()))
}

pub(super) fn is_permission_denied(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|err| err.kind() == ErrorKind::PermissionDenied)
    })
}
