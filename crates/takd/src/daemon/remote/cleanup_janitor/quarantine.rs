use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};

use super::{CLEANUP_TOMBSTONE_PREFIX, storage};

pub(super) fn quarantine_stale_remote_entry(path: &Path) -> Result<Option<PathBuf>> {
    quarantine_stale_remote_entry_with(path, |from, to| fs::rename(from, to))
}

pub(super) fn quarantine_stale_remote_entry_with(
    path: &Path,
    rename: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
) -> Result<Option<PathBuf>> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("cleanup path has no parent: {}", path.display()))?;
    let tombstone = parent.join(format!(
        "{CLEANUP_TOMBSTONE_PREFIX}{}",
        uuid::Uuid::new_v4()
    ));
    match rename(path, &tombstone) {
        Ok(()) => Ok(Some(tombstone)),
        Err(error) if error.kind() == ErrorKind::NotFound && source_is_missing(path) => Ok(None),
        Err(error) => Err(error).with_context(|| {
            format!(
                "failed to quarantine stale cleanup path {} as {}",
                path.display(),
                tombstone.display()
            )
        }),
    }
}

pub(super) fn cleanup_quarantined_remote_entries(root: &Path) -> Result<()> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to read cleanup root {}", root.display()));
        }
    };
    for entry in entries {
        let entry = entry
            .with_context(|| format!("failed to read cleanup entry under {}", root.display()))?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !name.starts_with(CLEANUP_TOMBSTONE_PREFIX) {
            continue;
        }
        if let Err(error) = storage::remove_stale_remote_entry(&path) {
            if storage::is_permission_denied(&error) {
                tracing::warn!(
                    "cleanup janitor skipped tombstone {}: {error:#}",
                    path.display()
                );
                continue;
            }
            return Err(error);
        }
    }
    Ok(())
}

fn source_is_missing(path: &Path) -> bool {
    fs::symlink_metadata(path).is_err_and(|error| error.kind() == ErrorKind::NotFound)
}
