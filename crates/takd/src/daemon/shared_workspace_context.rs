use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::Result;
use tak_core::v2::JobContextManifest;

use super::workspace_layer;

static VIEW_LOCK: Mutex<()> = Mutex::new(());

pub(super) fn ensure(
    base: &Path,
    shared: &Path,
    marker: &Path,
    context: &JobContextManifest,
) -> Result<()> {
    let _guard = VIEW_LOCK
        .lock()
        .map_err(|_| anyhow::anyhow!("shared workspace context lock poisoned"))?;
    let expected = serde_json::to_vec(context)?;
    match fs::read(marker) {
        Ok(actual) if actual == expected => return Ok(()),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    reconcile(base, shared, context)?;
    write_marker(marker, &expected)
}

fn reconcile(base: &Path, shared: &Path, context: &JobContextManifest) -> Result<()> {
    let allowed = context.paths.iter().collect::<BTreeSet<_>>();
    let mut entries = descendants(base)?;
    entries.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for source in &entries {
        let relative = relative(base, source)?;
        if !allowed.contains(&relative) {
            remove_unchanged(source, &shared.join(&relative))?;
        }
    }
    entries.sort_by_key(|path| path.components().count());
    for source in entries {
        let relative = relative(base, &source)?;
        if allowed.contains(&relative) {
            copy_missing(&source, &shared.join(relative))?;
        }
    }
    Ok(())
}

fn write_marker(path: &Path, contents: &[u8]) -> Result<()> {
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(contents)?;
    file.sync_all()?;
    fs::rename(&temporary, path)?;
    fs::File::open(path.parent().unwrap_or_else(|| Path::new(".")))?.sync_all()?;
    Ok(())
}

fn remove_unchanged(source: &Path, destination: &Path) -> Result<()> {
    if !same_entry(source, destination)? {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(destination)?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir(destination)?;
    } else {
        fs::remove_file(destination)?;
    }
    Ok(())
}

fn copy_missing(source: &Path, destination: &Path) -> Result<()> {
    match fs::symlink_metadata(destination) {
        Ok(_) => return Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    match workspace_layer::private_copy_shallow(source, destination) {
        Ok(()) => Ok(()),
        Err(_) if fs::symlink_metadata(destination).is_ok() => Ok(()),
        Err(error) => Err(error),
    }
}

fn same_entry(source: &Path, destination: &Path) -> Result<bool> {
    let source_meta = fs::symlink_metadata(source)?;
    let destination_meta = match fs::symlink_metadata(destination) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    if source_meta.file_type().is_symlink() || destination_meta.file_type().is_symlink() {
        return Ok(source_meta.file_type().is_symlink()
            && destination_meta.file_type().is_symlink()
            && fs::read_link(source)? == fs::read_link(destination)?);
    }
    if source_meta.is_dir() || destination_meta.is_dir() {
        return Ok(source_meta.is_dir()
            && destination_meta.is_dir()
            && fs::read_dir(destination)?.next().is_none());
    }
    Ok(source_meta.is_file()
        && destination_meta.is_file()
        && executable(&source_meta) == executable(&destination_meta)
        && source_meta.len() == destination_meta.len()
        && files_equal(source, destination)?)
}

fn files_equal(left: &Path, right: &Path) -> Result<bool> {
    let mut left = fs::File::open(left)?;
    let mut right = fs::File::open(right)?;
    let mut left_chunk = [0_u8; 64 * 1024];
    let mut right_chunk = [0_u8; 64 * 1024];
    loop {
        let left_len = left.read(&mut left_chunk)?;
        let right_len = right.read(&mut right_chunk)?;
        if left_len != right_len || left_chunk[..left_len] != right_chunk[..right_len] {
            return Ok(false);
        }
        if left_len == 0 {
            return Ok(true);
        }
    }
}

fn descendants(root: &Path) -> Result<Vec<PathBuf>> {
    let mut pending = children(root)?;
    let mut result = Vec::new();
    while let Some(path) = pending.pop() {
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            pending.extend(children(&path)?);
        }
        result.push(path);
    }
    Ok(result)
}

fn children(path: &Path) -> Result<Vec<PathBuf>> {
    fs::read_dir(path)?
        .map(|entry| entry.map(|entry| entry.path()).map_err(Into::into))
        .collect()
}

fn relative(root: &Path, path: &Path) -> Result<String> {
    Ok(path
        .strip_prefix(root)?
        .to_string_lossy()
        .replace('\\', "/"))
}

#[cfg(unix)]
fn executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn executable(_metadata: &fs::Metadata) -> bool {
    false
}
