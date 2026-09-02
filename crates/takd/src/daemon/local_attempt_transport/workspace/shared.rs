use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use tak_core::v2::JobContextManifest;

use super::{create_private, local_base_root, private_dir, remove_existing, unpack};
use crate::daemon::{shared_workspace_context, workspace_layer};

pub(super) fn prepare(
    archive: &Path,
    root: &Path,
    context: &JobContextManifest,
) -> Result<PathBuf> {
    let base_root = local_base_root(archive)?;
    let base = workspace_layer::immutable_base(&base_root, |data| unpack(archive, data))?;
    if let Some(data) = existing_data(root)? {
        shared_workspace_context::ensure(&base, &data, &root.join("context.json"), context)?;
        return Ok(data);
    }
    let parent = root
        .parent()
        .ok_or_else(|| anyhow::anyhow!("shared workspace root has no parent"))?;
    prepare_parent(parent)?;
    let temporary = parent.join(format!("shared-{}.tmp", uuid::Uuid::new_v4()));
    private_dir(&temporary)?;
    let data = temporary.join("data");
    private_dir(&data)?;
    workspace_layer::private_copy(&base, &data)?;
    shared_workspace_context::ensure(&base, &data, &temporary.join("context.json"), context)?;
    let mut marker = create_private(&temporary.join("ready"))?;
    marker.write_all(b"v2\n")?;
    marker.sync_all()?;
    fs::File::open(&temporary)?.sync_all()?;
    match fs::rename(&temporary, root) {
        Ok(()) => fs::File::open(parent)?.sync_all()?,
        Err(error) => {
            let Some(data) = existing_data(root)? else {
                return Err(error).context("publish shared workspace");
            };
            remove_existing(&temporary)?;
            return Ok(data);
        }
    }
    existing_data(root)?.ok_or_else(|| anyhow::anyhow!("published shared workspace is incomplete"))
}

fn prepare_parent(parent: &Path) -> Result<()> {
    match fs::symlink_metadata(parent) {
        Ok(metadata) => ensure!(
            metadata.file_type().is_dir(),
            "shared workspace storage parent is not a directory"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => private_dir(parent)?,
        Err(error) => return Err(error).context("inspect shared workspace storage parent"),
    }
    private_dir(parent)
}

fn existing_data(root: &Path) -> Result<Option<PathBuf>> {
    let metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).context("inspect shared workspace root"),
    };
    ensure!(
        metadata.file_type().is_dir(),
        "shared workspace root is not a directory"
    );
    let ready = fs::symlink_metadata(root.join("ready"))
        .context("inspect shared workspace ready marker")?;
    ensure!(
        ready.file_type().is_file(),
        "shared workspace ready marker is not a regular file"
    );
    let data_path = root.join("data");
    let data = fs::symlink_metadata(&data_path).context("inspect shared workspace data")?;
    ensure!(
        data.file_type().is_dir(),
        "shared workspace data is not a directory"
    );
    Ok(Some(data_path))
}
