use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::{Context, Result};

use super::{CLEANUP_TOMBSTONE_PREFIX, storage};

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
