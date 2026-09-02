use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

mod buffered_copy;
mod reflink;
mod removal;

pub(crate) use removal::{evict_pair, size as immutable_base_size};

pub(crate) fn immutable_base(
    root: &Path,
    populate: impl FnOnce(&Path) -> Result<()>,
) -> Result<PathBuf> {
    if ready(root)? {
        return Ok(root.join("data"));
    }
    let parent = root
        .parent()
        .ok_or_else(|| anyhow::anyhow!("workspace base has no parent"))?;
    private_dir(parent)?;
    let staging = parent.join(format!(".base-{}", uuid::Uuid::new_v4()));
    let data = staging.join("data");
    private_dir(&data)?;
    let result = publish_base(root, &staging, &data, populate);
    if staging.try_exists().unwrap_or(false) {
        removal::remove(&staging)?;
    }
    result.map(|()| root.join("data"))
}

pub(crate) fn private_copy(source: &Path, destination: &Path) -> Result<()> {
    private_dir(destination)?;
    copy_children(source, destination)
}

pub(crate) fn private_copy_shallow(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        return copy_symlink(source, destination);
    }
    if metadata.is_dir() {
        return private_dir(destination);
    }
    if !metadata.is_file() {
        bail!("immutable workspace base contains an unsupported entry");
    }
    if !reflink::try_reflink(source, destination)? {
        buffered_copy::copy(source, destination)?;
    }
    make_private_file(destination, &metadata)
}

fn publish_base(
    root: &Path,
    staging: &Path,
    data: &Path,
    populate: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    populate(data)?;
    make_immutable(data)?;
    fs::write(staging.join("ready"), b"v2\n")?;
    make_immutable(&staging.join("ready"))?;
    match fs::rename(staging, root) {
        Ok(()) => make_immutable(root),
        Err(_) if ready(root)? => Ok(()),
        Err(error) => Err(error).context("publish immutable workspace base"),
    }
}

fn ready(root: &Path) -> Result<bool> {
    let root_meta = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    if !root_meta.is_dir() || root_meta.file_type().is_symlink() {
        bail!("immutable workspace base is not a directory");
    }
    Ok(is_plain_dir(&root.join("data"))? && is_plain_file(&root.join("ready"))?)
}

fn is_plain_dir(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.is_dir() && !metadata.file_type().is_symlink()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn is_plain_file(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.is_file() && !metadata.file_type().is_symlink()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn copy_children(source: &Path, destination: &Path) -> Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        copy_entry(&entry.path(), &destination.join(entry.file_name()))?;
    }
    Ok(())
}

fn copy_entry(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        return copy_symlink(source, destination);
    }
    if metadata.is_dir() {
        private_dir(destination)?;
        return copy_children(source, destination);
    }
    if !metadata.is_file() {
        bail!("immutable workspace base contains an unsupported entry");
    }
    if !reflink::try_reflink(source, destination)? {
        buffered_copy::copy(source, destination)?;
    }
    make_private_file(destination, &metadata)
}

#[cfg(unix)]
fn copy_symlink(source: &Path, destination: &Path) -> Result<()> {
    std::os::unix::fs::symlink(fs::read_link(source)?, destination)?;
    Ok(())
}

#[cfg(not(unix))]
fn copy_symlink(_source: &Path, _destination: &Path) -> Result<()> {
    bail!("workspace symlink copies are unsupported on this platform")
}

#[cfg(unix)]
fn private_dir(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(not(unix))]
fn private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    Ok(())
}

#[cfg(unix)]
fn make_private_file(path: &Path, source: &fs::Metadata) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let executable = source.permissions().mode() & 0o111 != 0;
    fs::set_permissions(
        path,
        fs::Permissions::from_mode(if executable { 0o700 } else { 0o600 }),
    )?;
    Ok(())
}

#[cfg(not(unix))]
fn make_private_file(path: &Path, _source: &fs::Metadata) -> Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(false);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(unix)]
fn make_immutable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            make_immutable(&entry?.path())?;
        }
    }
    let executable = metadata.permissions().mode() & 0o111 != 0;
    let mode = if metadata.is_dir() || executable {
        0o500
    } else {
        0o400
    };
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(not(unix))]
fn make_immutable(path: &Path) -> Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions)?;
    Ok(())
}
