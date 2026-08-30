use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path};

use anyhow::{Context, Result, bail};
use ignore::gitignore::GitignoreBuilder;
use tak_core::v2::{OutputSelector, ResolvedTaskUnit, WorkspaceEntry, WorkspaceManifest};

use crate::daemon::run_store::RunStore;
use crate::daemon::scheduler::{DispatchCommand, ResultAcceptance};

pub(super) fn persist(
    store: &RunStore,
    command: &DispatchCommand,
    task: &ResolvedTaskUnit,
    workspace_root: &Path,
) -> Result<()> {
    if task.outputs.is_empty() {
        return Ok(());
    }
    let mut entries = BTreeMap::new();
    for selector in &task.outputs {
        let matched = match selector {
            OutputSelector::Path { value } => {
                safe_path(value)?;
                collect(
                    store,
                    workspace_root,
                    &workspace_root.join(value),
                    &mut entries,
                )?
            }
            OutputSelector::Glob { value } => {
                collect_glob(store, workspace_root, value, &mut entries)?
            }
        };
        if matched == 0 {
            bail!("declared output selector matched no workspace entries");
        }
    }
    let manifest = WorkspaceManifest::new(entries.into_values())?;
    match store.persist_attempt_task_outputs(command, &task.task_id, &manifest.entries)? {
        ResultAcceptance::Applied | ResultAcceptance::Duplicate => Ok(()),
        ResultAcceptance::Stale => bail!("local attempt output fence is stale"),
    }
}

fn collect_glob(
    store: &RunStore,
    root: &Path,
    pattern: &str,
    outputs: &mut BTreeMap<String, WorkspaceEntry>,
) -> Result<usize> {
    safe_glob(pattern)?;
    let mut builder = GitignoreBuilder::new(root);
    builder
        .add_line(None, pattern)
        .with_context(|| format!("invalid declared output glob `{pattern}`"))?;
    let matcher = builder.build()?;
    let mut matched = 0;
    for path in descendants(root)? {
        let metadata = fs::symlink_metadata(&path)?;
        let relative = path.strip_prefix(root)?;
        if matcher.matched(relative, metadata.is_dir()).is_ignore() {
            matched += collect(store, root, &path, outputs)?;
        }
    }
    Ok(matched)
}

fn collect(
    store: &RunStore,
    root: &Path,
    path: &Path,
    outputs: &mut BTreeMap<String, WorkspaceEntry>,
) -> Result<usize> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("declared output {} was not created", path.display()))?;
    let relative = relative(root, path)?;
    let entry = if metadata.file_type().is_symlink() {
        let target = fs::read_link(path)?;
        let target = target
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("declared output symlink target is not UTF-8"))?;
        WorkspaceEntry::symlink(relative.clone(), target)?
    } else if metadata.is_dir() {
        WorkspaceEntry::directory(relative.clone())?
    } else if metadata.is_file() {
        let captured = store.capture_output_file(path)?;
        WorkspaceEntry::file(
            relative.clone(),
            captured.executable,
            captured.size,
            &captured.sha256,
        )?
    } else {
        bail!("declared output `{relative}` has an unsupported entry type");
    };
    insert(outputs, entry)?;
    let mut matched = 1;
    if metadata.is_dir() {
        for child in children(path)? {
            matched += collect(store, root, &child, outputs)?;
        }
    }
    Ok(matched)
}

fn descendants(root: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut result = Vec::new();
    let mut pending = children(root)?;
    while let Some(path) = pending.pop() {
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.is_dir() {
            pending.extend(children(&path)?);
        }
        result.push(path);
    }
    result.sort();
    Ok(result)
}

fn children(path: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut children = fs::read_dir(path)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    children.sort_by(|left, right| right.cmp(left));
    Ok(children)
}

fn insert(outputs: &mut BTreeMap<String, WorkspaceEntry>, entry: WorkspaceEntry) -> Result<()> {
    if outputs
        .insert(entry.path.clone(), entry.clone())
        .is_some_and(|previous| previous != entry)
    {
        bail!("declared output changed while it was captured");
    }
    Ok(())
}

fn relative(root: &Path, path: &Path) -> Result<String> {
    let relative = path.strip_prefix(root)?;
    let parts = relative
        .components()
        .map(|part| {
            part.as_os_str()
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("declared output path is not UTF-8"))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(parts.join("/"))
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
