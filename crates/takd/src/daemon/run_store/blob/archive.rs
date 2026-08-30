use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use sha2::{Digest, Sha256};
use tak_core::v2::{WorkspaceEntry, WorkspaceManifest};

pub(super) fn verify_archive_manifest(path: &Path, expected: &WorkspaceManifest) -> Result<()> {
    let file = File::open(path).context("open workspace archive for manifest verification")?;
    let mut archive = tar::Archive::new(file);
    let mut manifest_entries = Vec::new();
    for entry in archive
        .entries()
        .context("workspace archive manifest is invalid")?
    {
        let mut entry = entry.context("workspace archive manifest entry is invalid")?;
        let path = entry
            .path()
            .context("workspace archive manifest path is invalid")?
            .into_owned();
        let path = path
            .to_str()
            .ok_or_else(|| anyhow!("workspace archive manifest path is not UTF-8"))?
            .to_owned();
        let entry_type = entry.header().entry_type();
        let resolved = if entry_type.is_file() {
            let executable = entry.header().mode()? & 0o111 != 0;
            let mut hasher = Sha256::new();
            let size = std::io::copy(&mut entry, &mut HashWriter(&mut hasher))?;
            WorkspaceEntry::file(path, executable, size, &format!("{:x}", hasher.finalize()))
        } else if entry_type.is_dir() {
            WorkspaceEntry::directory(path)
        } else if entry_type.is_symlink() {
            let target = entry
                .link_name()?
                .ok_or_else(|| anyhow!("workspace archive manifest symlink has no target"))?;
            let target = target
                .to_str()
                .ok_or_else(|| anyhow!("workspace archive manifest target is not UTF-8"))?;
            WorkspaceEntry::symlink(path, target)
        } else {
            bail!("workspace archive manifest contains an unsupported entry type");
        }
        .map_err(|error| anyhow!("workspace archive manifest is invalid: {error}"))?;
        manifest_entries.push(resolved);
    }
    let actual = WorkspaceManifest::new(manifest_entries)
        .map_err(|error| anyhow!("workspace archive manifest is invalid: {error}"))?;
    if &actual != expected {
        bail!("workspace archive manifest does not match the submitted manifest");
    }
    Ok(())
}

struct HashWriter<'a>(&'a mut Sha256);

impl std::io::Write for HashWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
