use std::fs;
use std::path::Path;

use anyhow::Result;

pub(super) fn private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

#[cfg(unix)]
pub(super) fn executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
pub(super) fn executable(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(unix)]
pub(super) fn set_executable(path: &Path, executable: bool) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(
        path,
        fs::Permissions::from_mode(if executable { 0o700 } else { 0o600 }),
    )?;
    Ok(())
}

#[cfg(not(unix))]
pub(super) fn set_executable(_path: &Path, _executable: bool) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
pub(super) fn create_symlink(target: &str, path: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, path)?;
    Ok(())
}

#[cfg(not(unix))]
pub(super) fn create_symlink(_target: &str, _path: &Path) -> Result<()> {
    anyhow::bail!("worker symlink overlays are unsupported on this platform")
}
