use std::fs::{self, File, OpenOptions};
use std::path::Path;

use anyhow::{Context, Result, bail};

const SERVE_LOCK_FILE: &str = "serve.lock";

pub struct ServiceStateLock {
    file: File,
}

impl ServiceStateLock {
    pub fn acquire(state_root: &Path) -> Result<Self> {
        fs::create_dir_all(state_root)
            .with_context(|| format!("create takd state root {}", state_root.display()))?;
        let path = state_root.join(SERVE_LOCK_FILE);
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .with_context(|| format!("open takd serve lock {}", path.display()))?;
        match try_lock_file(&file) {
            Ok(()) => Ok(Self { file }),
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                bail!(
                    "another takd serve process already owns state root {} (lock file {})",
                    state_root.display(),
                    path.display()
                )
            }
            Err(err) => {
                Err(err).with_context(|| format!("lock takd state root {}", state_root.display()))
            }
        }
    }
}

impl Drop for ServiceStateLock {
    fn drop(&mut self) {
        let _ = unlock_file(&self.file);
    }
}

#[cfg(unix)]
fn try_lock_file(file: &File) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;

    // SAFETY: `flock` only observes the live file descriptor; `file` remains
    // open for at least the duration of this call and for the lock lifetime.
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
fn unlock_file(file: &File) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;

    // SAFETY: `flock` only observes the live file descriptor; unlocking during
    // drop does not outlive `file`.
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
fn try_lock_file(_file: &File) -> std::io::Result<()> {
    Ok(())
}

#[cfg(not(unix))]
fn unlock_file(_file: &File) -> std::io::Result<()> {
    Ok(())
}
