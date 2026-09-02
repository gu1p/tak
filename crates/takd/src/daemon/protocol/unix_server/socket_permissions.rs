use std::path::Path;

use anyhow::{Context, Result, bail};

pub(super) async fn ensure_parent(path: &Path) -> Result<bool> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) if metadata.is_dir() => Ok(false),
        Ok(_) => bail!("socket parent is not a directory: {}", path.display()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            tokio::fs::create_dir_all(path)
                .await
                .with_context(|| format!("create socket directory {}", path.display()))?;
            Ok(true)
        }
        Err(error) => {
            Err(error).with_context(|| format!("inspect socket directory {}", path.display()))
        }
    }
}

#[cfg(unix)]
pub(super) async fn verify_parent_owner_only(path: &Path) -> Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let metadata = tokio::fs::symlink_metadata(path)
        .await
        .with_context(|| format!("inspect socket directory {}", path.display()))?;
    let expected_uid = unsafe { libc::geteuid() };
    if !metadata.is_dir() || metadata.uid() != expected_uid {
        bail!("socket parent owner is invalid: {}", path.display());
    }
    if metadata.permissions().mode() & 0o777 != 0o700 {
        bail!("socket parent is not owner-only: {}", path.display());
    }
    Ok(())
}

#[cfg(not(unix))]
pub(super) async fn verify_parent_owner_only(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
pub(super) async fn set_parent_owner_only(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .await
        .with_context(|| format!("set socket directory permissions {}", path.display()))
}

#[cfg(not(unix))]
pub(super) async fn set_parent_owner_only(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
pub(super) async fn set_owner_only(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let result = tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).await;
    match result {
        Ok(()) => verify_socket_access(path).await,
        Err(error) if unsupported_socket_chmod(&error) => verify_secure_parent(path)
            .await
            .with_context(|| format!("set socket permissions {}: {error}", path.display())),
        Err(error) => {
            Err(error).with_context(|| format!("set socket permissions {}", path.display()))
        }
    }
}

#[cfg(not(unix))]
pub(super) async fn set_owner_only(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
async fn verify_secure_parent(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("socket path has no parent"))?;
    let socket = tokio::fs::symlink_metadata(path).await?;
    let parent = tokio::fs::symlink_metadata(parent).await?;
    use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
    let socket_is_socket = socket.file_type().is_socket();
    let socket_uid = socket.uid();
    let socket_mode = socket.permissions().mode();
    let parent_is_dir = parent.is_dir();
    let parent_uid = parent.uid();
    let parent_mode = parent.permissions().mode();
    if !fallback_access_is_owner_only(
        socket_is_socket,
        socket_uid,
        socket_mode,
        parent_is_dir,
        parent_uid,
        parent_mode,
    ) {
        // SAFETY: geteuid has no preconditions and only reads process credentials.
        let euid = unsafe { libc::geteuid() };
        bail!(fallback_access_diagnostic(
            socket_is_socket,
            socket_uid,
            socket_mode,
            parent_is_dir,
            parent_uid,
            parent_mode,
            euid,
        ))
    }
    Ok(())
}

#[cfg(unix)]
async fn verify_socket_access(path: &Path) -> Result<()> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};

    let metadata = tokio::fs::symlink_metadata(path).await?;
    let mode = metadata.permissions().mode();
    // SAFETY: geteuid has no preconditions and only reads process credentials.
    let expected_uid = unsafe { libc::geteuid() };
    // Linux gates pathname-socket connect/send on write permission.
    if !metadata.file_type().is_socket()
        || metadata.uid() != expected_uid
        || !mode_allows_owner_connection(mode)
    {
        bail!("socket access is not owner-only")
    }
    Ok(())
}

#[cfg(unix)]
pub(super) fn mode_allows_owner_connection(mode: u32) -> bool {
    mode & 0o200 != 0 && mode & 0o022 == 0
}

#[cfg(unix)]
pub(super) fn fallback_access_is_owner_only(
    socket_is_socket: bool,
    socket_uid: u32,
    socket_mode: u32,
    parent_is_dir: bool,
    parent_uid: u32,
    parent_mode: u32,
) -> bool {
    socket_is_socket
        && parent_is_dir
        && socket_uid == parent_uid
        && socket_mode & 0o200 != 0
        && parent_mode & 0o022 == 0
        && !(parent_mode & 0o011 != 0 && socket_mode & 0o022 != 0)
}

#[cfg(unix)]
pub(super) fn fallback_access_diagnostic(
    socket_is_socket: bool,
    socket_uid: u32,
    socket_mode: u32,
    parent_is_dir: bool,
    parent_uid: u32,
    parent_mode: u32,
    euid: u32,
) -> String {
    format!(
        "socket directory does not protect owner-only access: \
socket_is_socket={socket_is_socket} socket_uid={socket_uid} socket_mode={socket_mode:#o} \
parent_is_dir={parent_is_dir} parent_uid={parent_uid} parent_mode={parent_mode:#o} euid={euid}"
    )
}

#[cfg(all(unix, target_os = "linux"))]
fn unsupported_socket_chmod(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(libc::EINVAL)
}

#[cfg(all(unix, not(target_os = "linux")))]
fn unsupported_socket_chmod(_error: &std::io::Error) -> bool {
    false
}
