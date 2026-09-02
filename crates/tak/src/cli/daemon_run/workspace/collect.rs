use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tak_core::v2::{TaskContext, WorkspaceEntry};

use super::{entry_from_metadata, is_git_metadata, relative_path};

pub(super) struct Collected {
    pub(super) entries: Vec<WorkspaceEntry>,
    pub(super) gitignored_paths: BTreeSet<String>,
}

#[derive(Clone)]
struct Candidate {
    absolute: PathBuf,
    metadata: fs::Metadata,
}

pub(super) fn for_contexts(root: &Path, contexts: &[&TaskContext]) -> Result<Collected> {
    let baseline = collect(root, true)?;
    if contexts
        .iter()
        .all(|context| context.include.is_empty() && context.use_gitignore)
    {
        return Ok(Collected {
            entries: materialize(baseline)?,
            gitignored_paths: BTreeSet::new(),
        });
    }
    let complete = collect(root, false)?;
    let ignored = complete
        .keys()
        .filter(|path| !baseline.contains_key(*path))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut selected = baseline;
    for context in contexts {
        add_context_entries(&complete, &ignored, context, &mut selected);
    }
    add_parent_directories(&complete, &mut selected);
    let gitignored_paths = selected
        .keys()
        .filter(|path| ignored.contains(*path))
        .cloned()
        .collect();
    Ok(Collected {
        entries: materialize(selected)?,
        gitignored_paths,
    })
}

fn collect(root: &Path, use_gitignore: bool) -> Result<BTreeMap<String, Candidate>> {
    let mut walker = ignore::WalkBuilder::new(root);
    walker
        .hidden(false)
        .ignore(false)
        .git_global(false)
        .git_ignore(use_gitignore)
        .git_exclude(use_gitignore)
        .parents(true)
        .require_git(false);
    let mut entries = BTreeMap::new();
    for entry in walker.build() {
        let entry = entry.context("scan workspace")?;
        if entry.path() == root || is_git_metadata(root, entry.path()) {
            continue;
        }
        let relative = relative_path(root, entry.path())?;
        let metadata = fs::symlink_metadata(entry.path())
            .with_context(|| format!("inspect workspace entry {relative}"))?;
        entries.insert(
            relative,
            Candidate {
                absolute: entry.path().to_path_buf(),
                metadata,
            },
        );
    }
    Ok(entries)
}

fn materialize(entries: BTreeMap<String, Candidate>) -> Result<Vec<WorkspaceEntry>> {
    entries
        .into_iter()
        .map(|(relative, candidate)| {
            entry_from_metadata(&candidate.absolute, relative, &candidate.metadata)
        })
        .collect()
}

fn add_context_entries(
    complete: &BTreeMap<String, Candidate>,
    gitignored: &BTreeSet<String>,
    context: &TaskContext,
    selected: &mut BTreeMap<String, Candidate>,
) {
    selected.extend(
        complete
            .iter()
            .filter(|(path, _)| inside_any(path, &context.roots))
            .filter(|(path, _)| {
                let explicitly_included = inside_any(path, &context.include);
                explicitly_included
                    || (!inside_any(path, &context.ignored_paths)
                        && (!context.use_gitignore || !gitignored.contains(*path)))
            })
            .map(|(path, entry)| (path.clone(), entry.clone())),
    );
}

fn add_parent_directories(
    complete: &BTreeMap<String, Candidate>,
    selected: &mut BTreeMap<String, Candidate>,
) {
    let paths = selected.keys().cloned().collect::<Vec<_>>();
    selected.extend(
        complete
            .iter()
            .filter(|(_, entry)| entry.metadata.is_dir())
            .filter(|(directory, _)| paths.iter().any(|path| inside(path, directory)))
            .map(|(path, entry)| (path.clone(), entry.clone())),
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
