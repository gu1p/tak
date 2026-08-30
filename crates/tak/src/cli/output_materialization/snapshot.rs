use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use sha2::{Digest, Sha256};
use tak_core::v2::WorkspaceEntry;

pub(super) fn one(root: &Path, relative: &str) -> Result<BTreeMap<String, WorkspaceEntry>> {
    let mut result = BTreeMap::new();
    if let Some(entry) = read(root, &root.join(relative))? {
        result.insert(relative.to_owned(), entry);
    }
    Ok(result)
}

pub(super) fn tree(root: &Path, relative: &str) -> Result<BTreeMap<String, WorkspaceEntry>> {
    let mut result = one(root, relative)?;
    let Some(entry) = result.get(relative) else {
        return Ok(result);
    };
    if entry.entry_type != tak_core::v2::WorkspaceEntryType::Directory {
        return Ok(result);
    }
    let mut pending = children(&root.join(relative))?;
    while let Some(path) = pending.pop() {
        let entry =
            read(root, &path)?.ok_or_else(|| anyhow!("checkout changed during preflight"))?;
        if entry.entry_type == tak_core::v2::WorkspaceEntryType::Directory {
            pending.extend(children(&path)?);
        }
        result.insert(entry.path.clone(), entry);
    }
    Ok(result)
}

fn read(root: &Path, path: &Path) -> Result<Option<WorkspaceEntry>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let relative = relative(root, path)?;
    let entry = if metadata.file_type().is_symlink() {
        let target = fs::read_link(path)?;
        WorkspaceEntry::symlink(
            relative,
            target
                .to_str()
                .ok_or_else(|| anyhow!("checkout symlink target is not UTF-8"))?,
        )?
    } else if metadata.is_dir() {
        WorkspaceEntry::directory(relative)?
    } else if metadata.is_file() {
        let bytes =
            fs::read(path).with_context(|| format!("read checkout path {}", path.display()))?;
        WorkspaceEntry::file(
            relative,
            executable(&metadata),
            bytes.len() as u64,
            &format!("{:x}", Sha256::digest(bytes)),
        )?
    } else {
        bail!(
            "checkout contains an unsupported entry at {}",
            path.display()
        );
    };
    Ok(Some(entry))
}

fn children(path: &Path) -> Result<Vec<PathBuf>> {
    let mut children = fs::read_dir(path)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    children.sort_by(|left, right| right.cmp(left));
    Ok(children)
}

fn relative(root: &Path, path: &Path) -> Result<String> {
    path.strip_prefix(root)?
        .components()
        .map(|part| {
            part.as_os_str()
                .to_str()
                .ok_or_else(|| anyhow!("checkout path is not UTF-8"))
        })
        .collect::<Result<Vec<_>>>()
        .map(|parts| parts.join("/"))
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
