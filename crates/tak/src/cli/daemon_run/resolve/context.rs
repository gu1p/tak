use std::collections::BTreeSet;

use anyhow::Result;
use tak_core::v2::{Session, TaskContext, WorkspaceEntryType, WorkspaceManifest};

pub(super) fn effective<'a>(
    task: Option<&'a TaskContext>,
    session: Option<&'a Session>,
) -> Option<&'a TaskContext> {
    task.or_else(|| session.and_then(|session| session.context.as_ref()))
}

pub(super) fn paths(
    manifest: &WorkspaceManifest,
    context: Option<&TaskContext>,
    gitignored_paths: &BTreeSet<String>,
) -> Result<Vec<String>> {
    let Some(context) = context else {
        return Ok(manifest
            .entries
            .iter()
            .filter(|entry| !gitignored_paths.contains(&entry.path))
            .map(|entry| entry.path.clone())
            .collect());
    };
    let mut selected = manifest
        .entries
        .iter()
        .filter(|entry| {
            inside_any(&entry.path, &context.roots)
                && !inside_any(&entry.path, &context.ignored_paths)
                && (!context.use_gitignore || !gitignored_paths.contains(&entry.path))
        })
        .map(|entry| entry.path.clone())
        .collect::<BTreeSet<_>>();
    selected.extend(
        manifest
            .entries
            .iter()
            .filter(|entry| {
                inside_any(&entry.path, &context.roots) && inside_any(&entry.path, &context.include)
            })
            .map(|entry| entry.path.clone()),
    );
    add_parent_directories(manifest, &mut selected);
    Ok(selected.into_iter().collect())
}

fn add_parent_directories(manifest: &WorkspaceManifest, selected: &mut BTreeSet<String>) {
    let paths = selected.iter().cloned().collect::<Vec<_>>();
    selected.extend(
        manifest
            .entries
            .iter()
            .filter(|entry| entry.entry_type == WorkspaceEntryType::Directory)
            .filter(|entry| paths.iter().any(|path| inside(path, &entry.path)))
            .map(|entry| entry.path.clone()),
    );
}

fn inside_any(path: &str, roots: &[String]) -> bool {
    roots.iter().any(|root| inside(path, root))
}

fn inside(path: &str, root: &str) -> bool {
    root == "."
        || path == root
        || path
            .strip_prefix(root)
            .is_some_and(|suffix| suffix.starts_with('/'))
}
