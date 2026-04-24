fn overlay_session_paths(
    execution_root_base: &Path,
    payload: &RemoteWorkerSubmitPayload,
    execution_root: &Path,
) -> Result<()> {
    let Some(RemoteWorkerSession {
        key,
        reuse: RemoteWorkerSessionReuse::SharePaths { .. },
    }) = payload.session.as_ref()
    else {
        return Ok(());
    };
    copy_directory_contents(&session_paths_root(execution_root_base, key), execution_root)
}

fn persist_session_paths(
    execution_root_base: &Path,
    payload: &RemoteWorkerSubmitPayload,
    execution_root: &Path,
) -> Result<()> {
    let Some(RemoteWorkerSession {
        key,
        reuse: RemoteWorkerSessionReuse::SharePaths { paths },
    }) = payload.session.as_ref()
    else {
        return Ok(());
    };
    let store = session_paths_root(execution_root_base, key);
    extract_share_paths(execution_root, &store, paths)
}

fn extract_share_paths(
    source_root: &Path,
    store_root: &Path,
    selectors: &[OutputSelectorSpec],
) -> Result<()> {
    for selector in selectors {
        match selector {
            OutputSelectorSpec::Path(path) => {
                replace_session_path(source_root, store_root, &path.path)?;
            }
            OutputSelectorSpec::Glob { pattern } => {
                copy_session_glob(source_root, store_root, pattern)?;
            }
        }
    }
    Ok(())
}

fn replace_session_path(source_root: &Path, store_root: &Path, relative: &str) -> Result<()> {
    let source = source_root.join(relative);
    let destination = store_root.join(relative);
    remove_existing_path(&destination)?;
    if source.is_dir() {
        copy_directory_contents(&source, &destination)?;
    } else if source.is_file() {
        copy_file(&source, &destination)?;
    }
    Ok(())
}

fn copy_session_glob(source_root: &Path, store_root: &Path, pattern: &str) -> Result<()> {
    let mut builder = GitignoreBuilder::new(source_root);
    builder
        .add_line(None, pattern)
        .with_context(|| format!("invalid session share glob `{pattern}`"))?;
    let matcher = builder
        .build()
        .with_context(|| format!("invalid session share glob `{pattern}`"))?;
    copy_matching_session_files(source_root, source_root, store_root, &matcher)
}

fn copy_matching_session_files(
    source_root: &Path,
    current: &Path,
    store_root: &Path,
    matcher: &Gitignore,
) -> Result<()> {
    for entry in fs::read_dir(current)
        .with_context(|| format!("failed to read session directory {}", current.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_matching_session_files(source_root, &path, store_root, matcher)?;
        } else if file_type.is_file() {
            let relative = path.strip_prefix(source_root)?;
            if matcher.matched(relative, false).is_ignore() {
                copy_file(&path, &store_root.join(relative))?;
            }
        }
    }
    Ok(())
}

fn copy_directory_contents(source: &Path, destination: &Path) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(source)
        .with_context(|| format!("failed to read directory {}", source.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(source)?;
        let destination_path = destination.join(relative);
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_directory_contents(&path, &destination_path)?;
        } else if file_type.is_file() {
            copy_file(&path, &destination_path)?;
        }
    }
    Ok(())
}

fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)?;
    Ok(())
}

fn remove_existing_path(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path)?,
        Ok(_) => fs::remove_file(path)?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err.into()),
    }
    Ok(())
}

fn session_workspace_root(execution_root_base: &Path, key: &str) -> PathBuf {
    execution_root_base
        .join("sessions")
        .join(sanitize_submit_idempotency_key(key))
}

fn session_paths_root(execution_root_base: &Path, key: &str) -> PathBuf {
    execution_root_base
        .join("session-paths")
        .join(sanitize_submit_idempotency_key(key))
}

fn is_share_workspace(payload: &RemoteWorkerSubmitPayload) -> bool {
    matches!(
        payload.session.as_ref().map(|session| &session.reuse),
        Some(RemoteWorkerSessionReuse::ShareWorkspace)
    )
}
