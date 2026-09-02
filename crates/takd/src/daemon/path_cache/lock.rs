use std::fs::{self, File, OpenOptions};
use std::path::Path;

use anyhow::Result;

pub(in crate::daemon) struct CacheLock {
    file: File,
}

impl CacheLock {
    pub(in crate::daemon) fn acquire(path: &Path) -> Result<Self> {
        let file = open(path)?;
        lock(&file, LockKind::Exclusive)?;
        Ok(Self { file })
    }

    pub(in crate::daemon) fn acquire_shared(path: &Path) -> Result<Self> {
        let file = open(path)?;
        lock(&file, LockKind::Shared)?;
        Ok(Self { file })
    }

    pub(in crate::daemon) fn try_acquire(path: &Path) -> Result<Option<Self>> {
        let file = open(path)?;
        if try_lock(&file)? {
            Ok(Some(Self { file }))
        } else {
            Ok(None)
        }
    }
}

fn open(path: &Path) -> Result<File> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?)
}

enum LockKind {
    Shared,
    Exclusive,
}

impl Drop for CacheLock {
    fn drop(&mut self) {
        let _ = unlock(&self.file);
    }
}

#[cfg(unix)]
fn lock(file: &File, kind: LockKind) -> Result<()> {
    use std::os::fd::AsRawFd;
    let operation = match kind {
        LockKind::Shared => libc::LOCK_SH,
        LockKind::Exclusive => libc::LOCK_EX,
    };
    // SAFETY: the descriptor belongs to `file`, which remains alive in `CacheLock`.
    let result = unsafe { libc::flock(file.as_raw_fd(), operation) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error().into())
    }
}

#[cfg(unix)]
fn try_lock(file: &File) -> Result<bool> {
    use std::os::fd::AsRawFd;
    // SAFETY: the descriptor belongs to `file`, which remains alive in `CacheLock`.
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if result == 0 {
        return Ok(true);
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::WouldBlock {
        return Ok(false);
    }
    Err(error.into())
}

#[cfg(unix)]
fn unlock(file: &File) -> Result<()> {
    use std::os::fd::AsRawFd;
    // SAFETY: the descriptor remains valid through this call.
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error().into())
    }
}

#[cfg(not(unix))]
fn lock(_file: &File, _kind: LockKind) -> Result<()> {
    Ok(())
}

#[cfg(not(unix))]
fn try_lock(_file: &File) -> Result<bool> {
    Ok(true)
}

#[cfg(not(unix))]
fn unlock(_file: &File) -> Result<()> {
    Ok(())
}
