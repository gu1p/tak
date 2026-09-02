use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::Result;

use crate::daemon::path_cache::{ACCESS_MARKER, lock::CacheLock};

pub(super) fn children(root: &Path) -> Result<Vec<PathBuf>> {
    match fs::read_dir(root) {
        Ok(entries) => Ok(entries
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<_>>()?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn tree_size(root: &Path) -> Result<u64> {
    let mut size = 0_u64;
    let mut pending = vec![root.to_owned()];
    while let Some(path) = pending.pop() {
        for child in children(&path)? {
            let metadata = fs::symlink_metadata(&child)?;
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                pending.push(child);
            } else if metadata.is_file()
                && child.file_name().and_then(|name| name.to_str()) != Some(ACCESS_MARKER)
            {
                size = size.saturating_add(metadata.len());
            }
        }
    }
    Ok(size)
}

pub(super) fn accessed(path: &Path) -> Result<u64> {
    if let Ok(value) = fs::read_to_string(path.join(ACCESS_MARKER))
        && let Ok(accessed) = value.trim().parse()
    {
        return Ok(accessed);
    }
    Ok(fs::metadata(path)?
        .modified()?
        .duration_since(UNIX_EPOCH)?
        .as_millis()
        .try_into()?)
}

pub(super) fn evict_path(path: &Path) -> Result<()> {
    let guard = CacheLock::acquire(&path.join("cache.lock"))?;
    let tombstone = path.with_file_name(format!(".gc-{}.tmp", uuid::Uuid::new_v4()));
    fs::rename(path, &tombstone)?;
    drop(guard);
    fs::remove_dir_all(tombstone)?;
    Ok(())
}
