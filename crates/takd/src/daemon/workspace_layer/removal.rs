use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub(crate) fn evict_pair(archive: &Path, base: &Path) -> Result<bool> {
    let archive_tombstone = tombstone(archive)?;
    match fs::rename(archive, &archive_tombstone) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    }
    let base_tombstone = match move_base(base) {
        Ok(value) => value,
        Err(error) => {
            fs::rename(&archive_tombstone, archive)
                .context("restore workspace archive after base eviction failure")?;
            return Err(error);
        }
    };
    fs::remove_file(archive_tombstone)?;
    if let Some(path) = base_tombstone {
        remove(&path)?;
    }
    Ok(true)
}

pub(crate) fn size(root: &Path) -> Result<u64> {
    let metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Ok(0);
    }
    let mut total = 0_u64;
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            total = total.saturating_add(size(&path)?);
        } else if metadata.is_file() {
            total = total.saturating_add(metadata.len());
        }
    }
    Ok(total)
}

pub(super) fn remove(path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        fs::remove_file(path)?;
        return Ok(());
    }
    make_writable(path)?;
    fs::remove_dir_all(path)?;
    Ok(())
}

fn move_base(base: &Path) -> Result<Option<PathBuf>> {
    if !base.try_exists()? {
        return Ok(None);
    }
    let tombstone = tombstone(base)?;
    fs::rename(base, &tombstone)?;
    Ok(Some(tombstone))
}

fn tombstone(path: &Path) -> Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("cache entry has no parent"))?;
    Ok(parent.join(format!(".gc-{}.tmp", uuid::Uuid::new_v4())))
}

fn make_writable(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    writable_permissions(path, &metadata)?;
    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            make_writable(&entry?.path())?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn writable_permissions(path: &Path, metadata: &fs::Metadata) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = if metadata.is_dir() { 0o700 } else { 0o600 };
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(not(unix))]
fn writable_permissions(path: &Path, _metadata: &fs::Metadata) -> Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(false);
    fs::set_permissions(path, permissions)?;
    Ok(())
}
