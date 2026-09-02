use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use sha2::{Digest, Sha256};
use tak_core::v2::{WorkspaceEntry, WorkspaceManifest};

use super::platform::executable;

pub(super) fn scan(root: &Path) -> Result<WorkspaceManifest> {
    let mut pending = children(root)?;
    let mut entries = Vec::new();
    while let Some(path) = pending.pop() {
        let metadata = fs::symlink_metadata(&path)?;
        let relative = path
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        let entry = if metadata.file_type().is_symlink() {
            WorkspaceEntry::symlink(relative, fs::read_link(&path)?.to_string_lossy())?
        } else if metadata.is_dir() {
            pending.extend(children(&path)?);
            WorkspaceEntry::directory(relative)?
        } else if metadata.is_file() {
            let bytes = fs::read(&path)?;
            WorkspaceEntry::file(
                relative,
                executable(&metadata),
                bytes.len() as u64,
                &format!("{:x}", Sha256::digest(bytes)),
            )?
        } else {
            bail!("worker workspace contains an unsupported entry");
        };
        entries.push(entry);
    }
    WorkspaceManifest::new(entries).map_err(Into::into)
}

fn children(path: &Path) -> Result<Vec<PathBuf>> {
    let mut result = fs::read_dir(path)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    result.sort_by(|left, right| right.cmp(left));
    Ok(result)
}
