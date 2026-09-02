use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use tak_core::v2::JobContextManifest;

use super::remove_existing;

pub(super) fn filter(root: &Path, manifest: &JobContextManifest) -> Result<()> {
    let allowed = manifest.paths.iter().collect::<BTreeSet<_>>();
    let mut entries = descendants(root)?;
    entries.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for entry in entries {
        let relative = entry
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        if !allowed.contains(&relative) {
            remove_existing(&entry)?;
        }
    }
    Ok(())
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
