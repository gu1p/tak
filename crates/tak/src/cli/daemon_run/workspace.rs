use std::collections::BTreeSet;
use std::fs;
use std::io::Cursor;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use sha2::{Digest, Sha256};
use tak_core::v2::{WorkspaceDescriptor, WorkspaceEntry, WorkspaceEntryType, WorkspaceManifest};

mod collect;

pub(in crate::cli) struct WorkspaceBundle {
    pub(in crate::cli) descriptor: WorkspaceDescriptor,
    pub(in crate::cli) archive: Vec<u8>,
    pub(in crate::cli) gitignored_paths: BTreeSet<String>,
}

pub(super) fn build(root: &Path) -> Result<WorkspaceBundle> {
    build_for_contexts(root, &[])
}

pub(super) fn build_for_contexts(
    root: &Path,
    contexts: &[&tak_core::v2::TaskContext],
) -> Result<WorkspaceBundle> {
    let collected = collect::for_contexts(root, contexts)?;
    let entries = collected.entries;
    let manifest = WorkspaceManifest::new(entries)?;
    let archive = build_archive(root, &manifest.entries)?;
    Ok(WorkspaceBundle {
        descriptor: WorkspaceDescriptor {
            manifest,
            archive_sha256: format!("{:x}", Sha256::digest(&archive)),
            archive_size: archive.len() as u64,
        },
        archive,
        gitignored_paths: collected.gitignored_paths,
    })
}

pub(super) fn entry_from_metadata(
    absolute: &Path,
    relative: String,
    metadata: &fs::Metadata,
) -> Result<WorkspaceEntry> {
    if metadata.file_type().is_symlink() {
        let target = fs::read_link(absolute)?;
        let target = target
            .to_str()
            .ok_or_else(|| anyhow!("workspace symlink target is not UTF-8: {relative}"))?;
        return Ok(WorkspaceEntry::symlink(relative, target)?);
    }
    if metadata.is_dir() {
        return Ok(WorkspaceEntry::directory(relative)?);
    }
    if metadata.is_file() {
        let contents =
            fs::read(absolute).with_context(|| format!("read workspace entry {relative}"))?;
        return Ok(WorkspaceEntry::file(
            relative,
            executable(metadata),
            contents.len() as u64,
            &format!("{:x}", Sha256::digest(contents)),
        )?);
    }
    bail!("unsupported workspace entry: {relative}")
}

fn build_archive(root: &Path, entries: &[WorkspaceEntry]) -> Result<Vec<u8>> {
    let mut archive = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut archive);
        builder.mode(tar::HeaderMode::Deterministic);
        for entry in entries {
            append_entry(&mut builder, root, entry)?;
        }
        builder.finish().context("finalize workspace archive")?;
    }
    Ok(archive)
}

fn append_entry(
    builder: &mut tar::Builder<&mut Vec<u8>>,
    root: &Path,
    entry: &WorkspaceEntry,
) -> Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    match entry.entry_type {
        WorkspaceEntryType::File => {
            header.set_entry_type(tar::EntryType::Regular);
            header.set_mode(if entry.executable { 0o755 } else { 0o644 });
            header.set_size(entry.size);
            header.set_cksum();
            let mut file = fs::File::open(root.join(&entry.path))?;
            builder.append_data(&mut header, &entry.path, &mut file)?;
        }
        WorkspaceEntryType::Directory => {
            header.set_entry_type(tar::EntryType::Directory);
            header.set_mode(0o755);
            header.set_size(0);
            header.set_cksum();
            builder.append_data(&mut header, &entry.path, Cursor::new([]))?;
        }
        WorkspaceEntryType::Symlink => {
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_mode(0o777);
            header.set_size(0);
            header.set_link_name(entry.symlink_target.as_deref().unwrap_or_default())?;
            header.set_cksum();
            builder.append_data(&mut header, &entry.path, Cursor::new([]))?;
        }
    }
    Ok(())
}

pub(super) fn relative_path(root: &Path, path: &Path) -> Result<String> {
    let relative = path.strip_prefix(root)?;
    let components = relative
        .components()
        .map(|component| {
            component
                .as_os_str()
                .to_str()
                .ok_or_else(|| anyhow!("workspace path is not UTF-8"))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(components.join("/"))
}

pub(super) fn is_git_metadata(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root)
        .ok()
        .and_then(|relative| relative.components().next())
        .is_some_and(|component| component.as_os_str() == ".git")
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
