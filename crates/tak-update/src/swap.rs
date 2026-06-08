//! The atomic on-disk binary swap and `.bak` backup/restore primitives.
//!
//! Replacing a running Unix executable is safe: the kernel keeps the open inode
//! alive, so we stage the new bytes in a temp file **in the target's own
//! directory** (same filesystem ⇒ the rename is atomic and never `EXDEV`), set
//! mode `0755`, fsync, then `rename(2)` it over the live path. A failure before
//! the rename leaves the original byte-for-byte intact (the temp is auto-removed).
//! The parent-directory fsync afterwards is best-effort: the rename is already
//! visible to the running process, and a power-loss in the sub-millisecond window
//! before the dir entry is durable self-heals to either the old or new binary
//! (both valid), so a dir-fsync error must not fail an otherwise-good swap.

use std::fs::{self, File, Permissions};
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Error from the atomic swap / backup primitives.
#[derive(Debug, thiserror::Error)]
pub enum SwapError {
    /// The target has no parent directory to stage into.
    #[error("target path `{0}` has no parent directory")]
    NoParentDir(PathBuf),
    /// Staging the new bytes in the target's directory failed.
    #[error("failed to stage replacement in `{0}`: {1}")]
    Stage(PathBuf, io::Error),
    /// Writing or fsyncing the staged file failed.
    #[error("failed to write staged file in `{0}`: {1}")]
    Write(PathBuf, io::Error),
    /// Setting mode `0755` on the staged file failed.
    #[error("failed to set mode on staged file in `{0}`: {1}")]
    SetMode(PathBuf, io::Error),
    /// The atomic rename over the target failed.
    #[error("failed to install `{0}`: {1}")]
    Rename(PathBuf, io::Error),
    /// The target could not be safely copied aside for rollback.
    #[error("failed to back up `{0}`: {1}")]
    Backup(PathBuf, io::Error),
}

/// A saved copy of a binary, used to roll back a failed multi-binary install.
#[derive(Debug, Clone)]
pub struct Backup {
    backup_path: PathBuf,
    original: PathBuf,
}

impl Backup {
    /// The on-disk path of the `.bak` copy.
    pub fn path(&self) -> &Path {
        &self.backup_path
    }
}

/// Atomically replace `target` with `new_bytes`, leaving it mode `mode` (e.g. `0o755`).
///
/// The staging file is created in `target`'s parent directory so the final step is
/// an atomic same-filesystem `rename`. The original is untouched on any error.
pub fn swap_binary_atomically(target: &Path, new_bytes: &[u8], mode: u32) -> Result<(), SwapError> {
    let dir = target
        .parent()
        .ok_or_else(|| SwapError::NoParentDir(target.to_path_buf()))?;
    let mut staged = tempfile::Builder::new()
        .prefix(".tak-update-")
        .suffix(".new")
        .tempfile_in(dir)
        .map_err(|err| SwapError::Stage(dir.to_path_buf(), err))?;
    staged
        .write_all(new_bytes)
        .and_then(|()| staged.flush())
        .map_err(|err| SwapError::Write(dir.to_path_buf(), err))?;
    // Set the executable mode before the durability fsync so the persisted inode
    // is already mode 0755 even if a crash follows the rename.
    fs::set_permissions(staged.path(), Permissions::from_mode(mode))
        .map_err(|err| SwapError::SetMode(dir.to_path_buf(), err))?;
    staged
        .as_file()
        .sync_all()
        .map_err(|err| SwapError::Write(dir.to_path_buf(), err))?;
    let staged = staged.into_temp_path();
    staged
        .persist(target)
        .map_err(|err| SwapError::Rename(target.to_path_buf(), err.error))?;
    fsync_dir(dir);
    Ok(())
}

/// Copy `target` aside to `<target>.bak` so a later failure can be rolled back.
///
/// Returns `Ok(None)` when `target` does not exist (a fresh install needs no
/// backup). Refuses a non-regular target (symlink/dir): self-update only replaces
/// real binaries, so a symlinked install path is a configuration we decline to
/// silently rewrite.
pub fn back_up(target: &Path) -> Result<Option<Backup>, SwapError> {
    let meta = match fs::symlink_metadata(target) {
        Ok(meta) => meta,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(SwapError::Backup(target.to_path_buf(), err)),
    };
    if !meta.file_type().is_file() {
        return Err(SwapError::Backup(
            target.to_path_buf(),
            io::Error::other("refusing to back up a non-regular file (symlink or directory)"),
        ));
    }
    let backup_path = backup_path_for(target);
    fs::copy(target, &backup_path).map_err(|err| SwapError::Backup(target.to_path_buf(), err))?;
    let _ = fs::set_permissions(&backup_path, meta.permissions());
    Ok(Some(Backup {
        backup_path,
        original: target.to_path_buf(),
    }))
}

/// Restore a backed-up binary over its original path (atomic same-dir rename).
pub fn restore(backup: &Backup) -> Result<(), SwapError> {
    fs::rename(&backup.backup_path, &backup.original)
        .map_err(|err| SwapError::Rename(backup.original.clone(), err))
}

/// Remove a backup once the new binary is confirmed healthy.
pub fn discard(backup: Backup) {
    let _ = fs::remove_file(&backup.backup_path);
}

fn backup_path_for(target: &Path) -> PathBuf {
    let mut name = target.as_os_str().to_os_string();
    name.push(".bak");
    PathBuf::from(name)
}

fn fsync_dir(dir: &Path) {
    if let Ok(handle) = File::open(dir) {
        let _ = handle.sync_all();
    }
}
