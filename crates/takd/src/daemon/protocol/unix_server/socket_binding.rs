use std::fs::{File, OpenOptions};
use std::io::ErrorKind;
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};
use std::path::Path;

use anyhow::{Context, Result, bail};
use tokio::net::{UnixListener, UnixStream};

pub(super) async fn bind(path: &Path) -> Result<(UnixListener, File)> {
    let lock_path = path.with_added_extension("lock");
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&lock_path)
        .with_context(|| format!("open socket lock {}", lock_path.display()))?;
    lock.try_lock()
        .with_context(|| format!("socket is already owned: {}", path.display()))?;
    remove_stale_socket(path).await?;
    let listener = UnixListener::bind(path)
        .with_context(|| format!("failed to bind socket {}", path.display()))?;
    Ok((listener, lock))
}

async fn remove_stale_socket(path: &Path) -> Result<()> {
    let metadata = match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).context("inspect existing socket path"),
    };
    if !metadata.file_type().is_socket() {
        bail!("refusing to replace non-socket path {}", path.display());
    }
    match UnixStream::connect(path).await {
        Ok(_) => bail!("socket is already serving: {}", path.display()),
        Err(error) if error.kind() == ErrorKind::ConnectionRefused => tokio::fs::remove_file(path)
            .await
            .with_context(|| format!("remove stale socket {}", path.display())),
        Err(error) => Err(error).context("check existing socket listener"),
    }
}
