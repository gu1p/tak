pub fn detect_workspace_root(start: &Path) -> Result<PathBuf> {
    let start = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
    let search_start = if start.is_file() {
        start
            .parent()
            .map_or_else(|| start.clone(), Path::to_path_buf)
    } else {
        start
    };

    if let Some(git) = find_ancestor_with(&search_start, ".git") {
        return Ok(git);
    }
    Ok(search_start)
}

/// Recursively discovers `TASKS.py` files while honoring gitignore-style filters.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn discover_tasks_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut builder = WalkBuilder::new(root);
    builder
        .git_ignore(true)
        .git_exclude(true)
        .parents(true)
        .require_git(false)
        .hidden(false)
        .ignore(true);

    for entry in builder.build() {
        let entry = entry?;
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        if entry
            .path()
            .file_name()
            .is_some_and(|name| name == TASKS_FILE)
        {
            files.push(entry.into_path());
        }
    }

    files.sort();
    Ok(files)
}
