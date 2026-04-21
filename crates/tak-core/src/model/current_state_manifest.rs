use super::*;

/// Builds a deterministic transfer manifest from available files and `CurrentState` boundaries.
///
/// The filter order is:
/// 1. keep files inside selected `roots`
/// 2. remove files matched by `ignored`
/// 3. re-add files matched by `include` if they are still inside `roots`
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn build_current_state_manifest(
    available_files: impl IntoIterator<Item = PathRef>,
    state: &CurrentStateSpec,
) -> ContextManifest {
    let files: Vec<PathRef> = available_files.into_iter().collect();
    let mut selected = Vec::new();

    for file in &files {
        if !matches_any_root(file, &state.roots) {
            continue;
        }
        if matches_any_ignored(file, &state.ignored) {
            continue;
        }
        selected.push(file.clone());
    }

    for include in &state.include {
        for file in &files {
            if !is_path_within(file, include) {
                continue;
            }
            if !matches_any_root(file, &state.roots) {
                continue;
            }
            selected.push(file.clone());
        }
    }

    ContextManifest::from_paths(selected)
}

pub(crate) fn compare_path_ref(left: &PathRef, right: &PathRef) -> Ordering {
    compare_anchor(&left.anchor, &right.anchor).then_with(|| left.path.cmp(&right.path))
}

fn matches_any_root(file: &PathRef, roots: &[PathRef]) -> bool {
    roots.iter().any(|root| is_path_within(file, root))
}

fn matches_any_ignored(file: &PathRef, ignored: &[IgnoreSourceSpec]) -> bool {
    ignored.iter().any(|source| match source {
        IgnoreSourceSpec::Path(path) => is_path_within(file, path),
        IgnoreSourceSpec::GitIgnore => false,
    })
}

fn is_path_within(file: &PathRef, container: &PathRef) -> bool {
    if file.anchor != container.anchor {
        return false;
    }
    if container.path == "." {
        return true;
    }
    file.path == container.path
        || file
            .path
            .strip_prefix(&container.path)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn compare_anchor(left: &PathAnchor, right: &PathAnchor) -> Ordering {
    anchor_sort_rank(left)
        .cmp(&anchor_sort_rank(right))
        .then_with(|| anchor_token(left).cmp(&anchor_token(right)))
}

fn anchor_sort_rank(anchor: &PathAnchor) -> u8 {
    match anchor {
        PathAnchor::Package => 0,
        PathAnchor::Repo(_) => 1,
        PathAnchor::Workspace => 2,
    }
}

fn anchor_token(anchor: &PathAnchor) -> String {
    match anchor {
        PathAnchor::Workspace => "workspace".to_string(),
        PathAnchor::Package => "package".to_string(),
        PathAnchor::Repo(name) => format!("repo:{name}"),
    }
}

pub(crate) fn hash_manifest_entries(entries: &[PathRef]) -> String {
    let mut hasher = Sha256::new();
    for entry in entries {
        let anchor = anchor_token(&entry.anchor);
        let anchor_bytes = anchor.as_bytes();
        let path_bytes = entry.path.as_bytes();

        hasher.update((anchor_bytes.len() as u64).to_be_bytes());
        hasher.update(anchor_bytes);
        hasher.update((path_bytes.len() as u64).to_be_bytes());
        hasher.update(path_bytes);
    }
    hex::encode(hasher.finalize())
}
