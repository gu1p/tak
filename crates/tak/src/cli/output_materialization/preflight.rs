use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Result, bail};
use tak_core::v2::{WorkspaceEntry, WorkspaceEntryType, WorkspaceManifest};

use super::snapshot;

pub(super) fn check(
    root: &Path,
    submitted: &WorkspaceManifest,
    outputs: &WorkspaceManifest,
) -> Result<()> {
    let submitted = entries(submitted);
    let mut conflicts = BTreeSet::new();
    for output_root in output_roots(outputs) {
        conflicts.extend(unsafe_ancestors(root, &output_root.path)?);
        let current = snapshot::tree(root, &output_root.path)?;
        conflicts.extend(changed_paths(&output_root.path, &submitted, &current));
    }
    if conflicts.is_empty() {
        return Ok(());
    }
    bail!(
        "checkout changed since submission at: {}; copied nothing; artifacts remain available with `tak runs outputs RUN_ID --to DIR`",
        conflicts.into_iter().collect::<Vec<_>>().join(", ")
    )
}

fn changed_paths(
    root: &str,
    submitted: &BTreeMap<String, WorkspaceEntry>,
    current: &BTreeMap<String, WorkspaceEntry>,
) -> Vec<String> {
    let keys = submitted
        .keys()
        .chain(current.keys())
        .filter(|path| within(path, root))
        .cloned()
        .collect::<BTreeSet<_>>();
    keys.into_iter()
        .filter(|path| current.get(path) != submitted.get(path))
        .collect()
}

fn unsafe_ancestors(root: &Path, output: &str) -> Result<Vec<String>> {
    let mut conflicts = Vec::new();
    let parts = output.split('/').collect::<Vec<_>>();
    for end in 1..parts.len() {
        let ancestor = parts[..end].join("/");
        if snapshot::one(root, &ancestor)?
            .get(&ancestor)
            .is_some_and(|entry| entry.entry_type != WorkspaceEntryType::Directory)
        {
            conflicts.push(ancestor);
        }
    }
    Ok(conflicts)
}

pub(super) fn output_roots(manifest: &WorkspaceManifest) -> Vec<&WorkspaceEntry> {
    let paths = manifest
        .entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<BTreeSet<_>>();
    manifest
        .entries
        .iter()
        .filter(|entry| !ancestors(&entry.path).any(|path| paths.contains(path.as_str())))
        .collect()
}

fn ancestors(path: &str) -> impl Iterator<Item = String> + '_ {
    let parts = path.split('/').collect::<Vec<_>>();
    (1..parts.len())
        .rev()
        .map(move |end| parts[..end].join("/"))
}

fn within(path: &str, root: &str) -> bool {
    path == root
        || path
            .strip_prefix(root)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn entries(manifest: &WorkspaceManifest) -> BTreeMap<String, WorkspaceEntry> {
    manifest
        .entries
        .iter()
        .cloned()
        .map(|entry| (entry.path.clone(), entry))
        .collect()
}
