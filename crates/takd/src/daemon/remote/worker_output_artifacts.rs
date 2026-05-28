use super::*;

pub(super) fn stage_remote_worker_outputs(
    artifact_root: &Path,
    execution_root: &Path,
    outputs: &[RemoteWorkerOutputRecord],
) -> Result<()> {
    clear_remote_output_artifacts(artifact_root)?;
    if outputs.is_empty() {
        return Ok(());
    }

    fs::create_dir_all(artifact_root)
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

fn clear_remote_output_artifacts(artifact_root: &Path) -> Result<()> {
    match fs::remove_dir_all(artifact_root) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err)
            .with_context(|| format!("failed to clear artifact root {}", artifact_root.display())),
    }
}
