use std::fs::{self, OpenOptions};
use std::path::{Component, Path};

use anyhow::{Result, bail};
use sha2::{Digest, Sha256};
use tak_core::v2::WorkspaceEntryType;

use super::remove_existing;
use crate::daemon::run_store::output_artifacts::OutputOverlay;

pub(super) fn apply(root: &Path, overlays: &[OutputOverlay]) -> Result<()> {
    let mut directories = overlays
        .iter()
        .filter(|overlay| overlay.entry.entry_type == WorkspaceEntryType::Directory)
        .collect::<Vec<_>>();
    directories.sort_by_key(|overlay| overlay.entry.path.split('/').count());
    for overlay in directories {
        let path = destination(root, &overlay.entry.path)?;
        remove_existing(&path)?;
        fs::create_dir(&path)?;
    }
    for overlay in overlays
        .iter()
        .filter(|overlay| overlay.entry.entry_type == WorkspaceEntryType::File)
    {
        let path = destination(root, &overlay.entry.path)?;
        remove_existing(&path)?;
        let source = overlay
            .blob_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("declared output file blob is missing"))?;
        let mut input = fs::File::open(source)?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(if overlay.entry.executable {
                0o755
            } else {
                0o644
            });
        }
        let mut output = options.open(&path)?;
        let mut digest = Sha256::new();
        let mut size = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = std::io::Read::read(&mut input, &mut buffer)?;
            if read == 0 {
                break;
            }
            std::io::Write::write_all(&mut output, &buffer[..read])?;
            digest.update(&buffer[..read]);
            size += read as u64;
        }
        if size != overlay.entry.size
            || format!("{:x}", digest.finalize()) != overlay.entry.content_sha256
        {
            bail!("declared output overlay blob digest is invalid");
        }
    }
    for overlay in overlays
        .iter()
        .filter(|overlay| overlay.entry.entry_type == WorkspaceEntryType::Symlink)
    {
        let path = destination(root, &overlay.entry.path)?;
        remove_existing(&path)?;
        create_symlink(
            overlay
                .entry
                .symlink_target
                .as_deref()
                .expect("validated output symlink has a target"),
            &path,
        )?;
    }
    Ok(())
}

fn destination(root: &Path, relative: &str) -> Result<std::path::PathBuf> {
    let path = Path::new(relative);
    if !path
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        bail!("declared output overlay path is unsafe");
    }
    let mut current = root.to_owned();
    let mut components = path.components().peekable();
    while let Some(component) = components.next() {
        current.push(component);
        if components.peek().is_none() {
            break;
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_dir() => {}
            Ok(_) => bail!("declared output overlay parent is not a directory"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current)?;
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(current)
}

#[cfg(unix)]
fn create_symlink(target: &str, path: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, path)?;
    Ok(())
}

#[cfg(not(unix))]
fn create_symlink(_target: &str, _path: &Path) -> Result<()> {
    bail!("declared output symlinks are not supported on this platform")
}
