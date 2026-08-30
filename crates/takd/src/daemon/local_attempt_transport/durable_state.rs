use std::fs::{self, File, OpenOptions};
use std::path::Path;

use anyhow::{Context, Result};

use super::workspace;
use crate::daemon::attempt_coordinator::AttemptObservation;

pub(super) struct AttemptOwner {
    file: File,
}

impl AttemptOwner {
    pub(super) fn try_acquire(root: &Path) -> Result<Option<Self>> {
        private_dir(root)?;
        let path = root.join("owner.lock");
        let file = open_private(&path)?;
        match try_lock(&file) {
            Ok(()) => Ok(Some(Self { file })),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(error) => Err(error).with_context(|| format!("lock {}", path.display())),
        }
    }
}

impl Drop for AttemptOwner {
    fn drop(&mut self) {
        let _ = unlock(&self.file);
    }
}

pub(super) fn observe(root: &Path) -> Result<AttemptObservation> {
    if let Some(completion) = workspace::read_completion(root)? {
        return Ok(AttemptObservation::Completed(completion));
    }
    let Some(_owner) = AttemptOwner::try_acquire(root)? else {
        return Ok(AttemptObservation::Running);
    };
    Ok(workspace::read_completion(root)?
        .map_or(AttemptObservation::Missing, AttemptObservation::Completed))
}

fn private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn open_private(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.create(true).truncate(false).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    Ok(file)
}

#[cfg(unix)]
fn try_lock(file: &File) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    (result == 0)
        .then_some(())
        .ok_or_else(std::io::Error::last_os_error)
}

#[cfg(unix)]
fn unlock(file: &File) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
    (result == 0)
        .then_some(())
        .ok_or_else(std::io::Error::last_os_error)
}

#[cfg(not(unix))]
fn try_lock(_file: &File) -> std::io::Result<()> {
    Ok(())
}

#[cfg(not(unix))]
fn unlock(_file: &File) -> std::io::Result<()> {
    Ok(())
}
