fn refresh_session_storage_parent_for_submit(
    idempotency_key: &str,
    execution_root_base: &Path,
    payload: &RemoteWorkerSubmitPayload,
) {
    if let Err(error) =
        refresh_session_storage_parent(execution_root_base, payload.session.as_ref())
    {
        tracing::warn!("failed to refresh session storage for submit {idempotency_key}: {error:#}");
    }
}

fn refresh_session_storage_parent(
    execution_root_base: &Path,
    session: Option<&RemoteWorkerSession>,
) -> Result<()> {
    refresh_session_storage_parent_with(execution_root_base, session, open_session_storage_parent)
}

fn refresh_session_storage_parent_with(
    execution_root_base: &Path,
    session: Option<&RemoteWorkerSession>,
    open: impl FnOnce(&Path) -> std::io::Result<fs::File>,
) -> Result<()> {
    let directory_name = match session.map(|session| &session.reuse) {
        Some(RemoteWorkerSessionReuse::ShareWorkspace) => SESSION_WORKSPACES_DIR_NAME,
        Some(RemoteWorkerSessionReuse::SharePaths { .. }) => SESSION_PATHS_DIR_NAME,
        Some(RemoteWorkerSessionReuse::Container) | None => return Ok(()),
    };
    let storage_parent = execution_root_base.join(directory_name);
    let storage_parent = match open(&storage_parent) {
        Ok(storage_parent) => storage_parent,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "failed to open session storage parent {}",
                    storage_parent.display()
                )
            });
        }
    };
    storage_parent
        .set_modified(SystemTime::now())
        .with_context(|| {
            format!(
                "failed to refresh session storage parent {}",
                execution_root_base.join(directory_name).display()
            )
        })
}

#[cfg(unix)]
fn open_session_storage_parent(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;

    fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(path)
}

#[cfg(not(unix))]
fn open_session_storage_parent(path: &Path) -> std::io::Result<fs::File> {
    let file = fs::File::open(path)?;
    if !file.metadata()?.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotADirectory,
            format!(
                "session storage parent is not a directory: {}",
                path.display()
            ),
        ));
    }
    Ok(file)
}
