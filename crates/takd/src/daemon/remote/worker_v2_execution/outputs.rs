use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use ignore::gitignore::GitignoreBuilder;
use sha2::{Digest, Sha256};
use tak_core::v2::{OutputSelector, ResolvedTaskUnit, WorkspaceEntry};
use tak_proto::worker_v2::WorkerAttemptIdentity;

use super::super::SubmitAttemptStore;

pub(super) fn publish(
    store: &SubmitAttemptStore,
    identity: &WorkerAttemptIdentity,
    task: &ResolvedTaskUnit,
    root: &Path,
) -> Result<()> {
    let mut captured = BTreeMap::new();
    for selector in &task.outputs {
        let matched = match selector {
            OutputSelector::Path { value } => {
                safe_path(value)?;
                collect(root, &root.join(value), &mut captured)?
            }
            OutputSelector::Glob { value } => collect_glob(root, value, &mut captured)?,
        };
        if matched == 0 {
            bail!("declared output selector matched no workspace entries");
        }
    }
    for (_, (entry, content)) in captured {
        store.publish_worker_v2_output(identity, &task.task_id, entry, &content)?;
    }
    Ok(())
}

fn collect_glob(
    root: &Path,
    pattern: &str,
    captured: &mut BTreeMap<String, (WorkspaceEntry, Vec<u8>)>,
) -> Result<usize> {
    safe_glob(pattern)?;
    let mut builder = GitignoreBuilder::new(root);
    builder.add_line(None, pattern)?;
    let matcher = builder.build()?;
    let mut matched = 0;
    for path in descendants(root)? {
        let metadata = fs::symlink_metadata(&path)?;
        if matcher
            .matched(path.strip_prefix(root)?, metadata.is_dir())
            .is_ignore()
        {
            matched += collect(root, &path, captured)?;
        }
    }
    Ok(matched)
}

fn collect(
    root: &Path,
    path: &Path,
    captured: &mut BTreeMap<String, (WorkspaceEntry, Vec<u8>)>,
) -> Result<usize> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("declared output {} was not created", path.display()))?;
    let relative = relative(root, path)?;
    let (entry, content) = if metadata.file_type().is_symlink() {
        let target = fs::read_link(path)?;
        (
            WorkspaceEntry::symlink(relative.clone(), target.to_string_lossy())?,
            Vec::new(),
        )
    } else if metadata.is_dir() {
        (WorkspaceEntry::directory(relative.clone())?, Vec::new())
    } else if metadata.is_file() {
        let bytes = fs::read(path)?;
        (
            WorkspaceEntry::file(
                relative.clone(),
                executable(&metadata),
                bytes.len() as u64,
                &format!("{:x}", Sha256::digest(&bytes)),
            )?,
            bytes,
        )
    } else {
        bail!("declared output `{relative}` has an unsupported entry type");
    };
    if captured.insert(relative, (entry, content)).is_some() {
        return Ok(0);
    }
    let mut matched = 1;
    if metadata.is_dir() {
        for child in children(path)? {
            matched += collect(root, &child, captured)?;
        }
    }
    Ok(matched)
}

fn descendants(root: &Path) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    let mut pending = children(root)?;
    while let Some(path) = pending.pop() {
        if fs::symlink_metadata(&path)?.is_dir() {
            pending.extend(children(&path)?);
        }
        result.push(path);
    }
    result.sort();
    Ok(result)
}

fn children(path: &Path) -> Result<Vec<PathBuf>> {
    let mut children = fs::read_dir(path)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    children.sort_by(|left, right| right.cmp(left));
    Ok(children)
}

fn relative(root: &Path, path: &Path) -> Result<String> {
    Ok(path
        .strip_prefix(root)?
        .components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

fn safe_path(value: &str) -> Result<()> {
    if value.is_empty()
        || value.contains('\\')
        || !Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        bail!("declared output path must be relative and non-escaping");
    }
    Ok(())
}

fn safe_glob(value: &str) -> Result<()> {
    if value.is_empty()
        || value.contains('\\')
        || Path::new(value).components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        bail!("declared output glob must stay inside the workspace");
    }
    Ok(())
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
