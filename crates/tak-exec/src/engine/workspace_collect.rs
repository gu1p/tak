use super::*;

pub(crate) fn collect_workspace_files(
    workspace_root: &Path,
    state: &CurrentStateSpec,
) -> Result<Vec<PathRef>> {
    let mut files = if uses_workspace_gitignore(state) {
        collect_workspace_files_with_gitignore(workspace_root)?
    } else {
        let mut files = Vec::new();
        collect_workspace_files_recursive(workspace_root, workspace_root, &mut files)?;
        files
    };
    collect_explicit_include_files(workspace_root, &state.include, &mut files)?;
    Ok(files)
}

fn uses_workspace_gitignore(state: &CurrentStateSpec) -> bool {
    state.origin == CurrentStateOrigin::ImplicitDefault || uses_gitignore_ignore_source(state)
}

fn uses_gitignore_ignore_source(state: &CurrentStateSpec) -> bool {
    state
        .ignored
        .iter()
        .any(|source| matches!(source, IgnoreSourceSpec::GitIgnore))
}

fn collect_workspace_files_with_gitignore(workspace_root: &Path) -> Result<Vec<PathRef>> {
    let mut builder = ignore::WalkBuilder::new(workspace_root);
    builder
        .hidden(false)
        .ignore(false)
        .git_global(false)
        .git_ignore(true)
        .git_exclude(false)
        .parents(true)
        .require_git(false);

    let mut files = Vec::new();
    for entry in builder.build() {
        let entry = entry.with_context(|| {
            format!(
                "failed to read workspace entry during gitignore-aware scan under {}",
                workspace_root.display()
            )
        })?;
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }

        let path = entry.path();
        push_workspace_file(workspace_root, path, &mut files)?;
    }

    Ok(files)
}

fn collect_workspace_files_recursive(
    workspace_root: &Path,
    current_dir: &Path,
    files: &mut Vec<PathRef>,
) -> Result<()> {
    for entry in fs::read_dir(current_dir).with_context(|| {
        format!(
            "failed to read workspace directory {}",
            current_dir.display()
        )
    })? {
        let entry = entry.with_context(|| {
            format!(
                "failed to read workspace entry under {}",
                current_dir.display()
            )
        })?;
        let path = entry.path();
        let file_type = entry.file_type().with_context(|| {
            format!(
                "failed to read file type for workspace entry {}",
                path.display()
            )
        })?;

        if file_type.is_dir() {
            collect_workspace_files_recursive(workspace_root, &path, files)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        push_workspace_file(workspace_root, &path, files)?;
    }

    Ok(())
}

fn push_workspace_file(workspace_root: &Path, path: &Path, files: &mut Vec<PathRef>) -> Result<()> {
    let relative = path.strip_prefix(workspace_root).with_context(|| {
        format!(
            "failed to compute relative path for workspace file {}",
            path.display()
        )
    })?;
    files.push(PathRef {
        anchor: PathAnchor::Workspace,
        path: normalize_filesystem_relative_path(relative),
    });
    Ok(())
}

fn collect_explicit_include_files(
    workspace_root: &Path,
    includes: &[PathRef],
    files: &mut Vec<PathRef>,
) -> Result<()> {
    for include in includes {
        if include.anchor != PathAnchor::Workspace {
            bail!(
                "unsupported non-workspace context include during staging: {:?}",
                include.anchor
            );
        }
        if include.path == "." {
            collect_workspace_files_recursive(workspace_root, workspace_root, files)?;
            continue;
        }

        let include_path = workspace_root.join(&include.path);
        if include_path.is_dir() {
            collect_workspace_files_recursive(workspace_root, &include_path, files)?;
            continue;
        }
        if !include_path.is_file() {
            continue;
        }

        files.push(PathRef {
            anchor: PathAnchor::Workspace,
            path: include.path.clone(),
        });
    }

    Ok(())
}

pub(crate) fn materialize_manifest_files(
    workspace_root: &Path,
    staged_root: &Path,
    entries: &[PathRef],
) -> Result<()> {
    for entry in entries {
        if entry.anchor != PathAnchor::Workspace {
            bail!(
                "unsupported non-workspace context manifest anchor during staging: {:?}",
                entry.anchor
            );
        }
        if entry.path == "." {
            continue;
        }

        let source = workspace_root.join(&entry.path);
        if !source.is_file() {
            continue;
        }
        let destination = staged_root.join(&entry.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create staged directory {}",
                    parent.to_string_lossy()
                )
            })?;
        }
        fs::copy(&source, &destination).with_context(|| {
            format!(
                "failed to stage context file {} -> {}",
                source.display(),
                destination.display()
            )
        })?;
    }

    Ok(())
}
