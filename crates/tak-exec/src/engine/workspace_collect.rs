/// Collects all regular files under the workspace root as normalized workspace-anchored refs.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn collect_workspace_files(workspace_root: &Path) -> Result<Vec<PathRef>> {
    let mut files = Vec::new();
    collect_workspace_files_recursive(workspace_root, workspace_root, &mut files)?;
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
    }

    Ok(())
}

/// Copies manifest-selected files into the staged workspace while preserving relative layout.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn materialize_manifest_files(
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
