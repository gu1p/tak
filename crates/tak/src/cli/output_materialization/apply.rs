use std::fs;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use tak_core::v2::WorkspaceManifest;

pub(super) fn all(root: &Path, staging: &Path, outputs: &WorkspaceManifest) -> Result<()> {
    for entry in super::preflight::output_roots(outputs) {
        let source = staging.join(&entry.path);
        let destination = root.join(&entry.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("create output parent in checkout {}", parent.display())
            })?;
        }
        remove_existing(&destination)?;
        fs::rename(&source, &destination).with_context(|| {
            format!(
                "materialize output {} into checkout {}",
                entry.path,
                root.display()
            )
        })?;
    }
    Ok(())
}

fn remove_existing(path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    ensure!(
        path.file_name().is_some(),
        "refusing to replace checkout root"
    );
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}
