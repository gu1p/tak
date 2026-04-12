use super::*;

pub(super) fn stage_remote_worker_outputs(
    idempotency_key: &str,
    execution_root: &Path,
    outputs: &[RemoteWorkerOutputRecord],
) -> Result<()> {
    let artifact_root = artifact_root_for_submit_key(idempotency_key);
    clear_remote_output_artifacts(&artifact_root)?;
    if outputs.is_empty() {
        return Ok(());
    }

    fs::create_dir_all(&artifact_root)
        .with_context(|| format!("failed to create artifact root {}", artifact_root.display()))?;
    for output in outputs {
        let normalized = normalize_path_ref("workspace", &output.path)
            .map_err(|err| anyhow!("invalid remote output path `{}`: {err}", output.path))?;
        if normalized.path == "." {
            bail!(
                "invalid remote output path `{}`: must reference a file",
                output.path
            );
        }

        let source = execution_root.join(&normalized.path);
        let destination = artifact_root.join(&normalized.path);
        let Some(parent) = destination.parent() else {
            bail!(
                "failed to resolve parent directory for remote output {}",
                destination.display()
            );
        };
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create artifact parent {}", parent.display()))?;
        fs::copy(&source, &destination).with_context(|| {
            format!(
                "failed to stage remote output {} -> {}",
                source.display(),
                destination.display()
            )
        })?;
    }

    Ok(())
}

pub(super) fn consume_staged_remote_output(
    idempotency_key: &str,
    relative_path: &str,
) -> Result<Option<Vec<u8>>> {
    let artifact_root = artifact_root_for_submit_key(idempotency_key);
    if !artifact_root.exists() {
        return Ok(None);
    }

    let output_path = artifact_root.join(relative_path);
    let bytes = match fs::read(&output_path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(err).with_context(|| {
                format!(
                    "failed to read staged remote output {}",
                    output_path.display()
                )
            });
        }
    };
    if let Err(err) = fs::remove_file(&output_path)
        && err.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(
            "failed to remove served staged remote output {}: {err}",
            output_path.display()
        );
    }
    prune_empty_ancestor_dirs(&output_path, &artifact_root);

    Ok(Some(bytes))
}

fn clear_remote_output_artifacts(artifact_root: &Path) -> Result<()> {
    match fs::remove_dir_all(artifact_root) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err)
            .with_context(|| format!("failed to clear artifact root {}", artifact_root.display())),
    }
}

fn prune_empty_ancestor_dirs(path: &Path, root: &Path) {
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir == root {
            match fs::remove_dir(dir) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) if err.kind() == std::io::ErrorKind::DirectoryNotEmpty => break,
                Err(err) => {
                    tracing::warn!(
                        "failed to prune artifact directory {}: {err}",
                        dir.display()
                    );
                    break;
                }
            }
            return;
        }

        match fs::remove_dir(dir) {
            Ok(()) => current = dir.parent(),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => current = dir.parent(),
            Err(err) if err.kind() == std::io::ErrorKind::DirectoryNotEmpty => break,
            Err(err) => {
                tracing::warn!(
                    "failed to prune artifact directory {}: {err}",
                    dir.display()
                );
                break;
            }
        }
    }
}
